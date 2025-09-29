#![feature(exit_status_error)]
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use backon::{BlockingRetryable, ConstantBuilder};
use clap::Parser;
use eyre::Ok;
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use scraper::Html;

use crate::app::App;
use crate::categorizer::Categorizer;
use crate::pdf_tools::PdfTools;
use crate::structs::{Export, Message, Record};
use crate::toc::{Toc, TocItem};

mod app;
mod categorizer;
mod command;
mod pdf_tools;
mod structs;
mod toc;

/// Simple program to greet a person
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Preserve tmp directories
    #[arg(long)]
    preserve_tmp: bool,

    /// Skip images processing with ocr
    #[arg(long)]
    no_ocr: bool,

    /// Skip images processing with ocr
    #[arg(long, default_value_t = 3)]
    ocr_retries: usize,

    /// Do not shrink or extend text pages (including toc)
    #[arg(long)]
    unadaptive_text_pages: bool,

    /// Source locations
    #[arg(default_values_t = vec![".".to_string()])]
    sources: Vec<String>,

    /// Filter people to process (all by default)
    #[arg(short = 'p')]
    people: Vec<String>,
}

fn get_export_result(export_path: &str) -> eyre::Result<Export> {
    let result_json = &format!("{export_path}/result.json");

    let red = String::from_utf8(fs::read(result_json)?)?;
    let mut data: structs::Export = serde_json::from_str(&red)?;

    data.messages
        .iter_mut()
        .for_each(|msg| msg.export_path = Some(export_path.into()));

    Ok(data)
}

fn main() -> eyre::Result<()> {
    let cli_args = Cli::parse();

    let res = app(cli_args);

    let Err(err) = res else { return Ok(()) };

    write_err(format!("{err:?}"))?;

    Err(err)
}

fn strip_html_tags(html: &str) -> String {
    let fragment = Html::parse_fragment(html);
    fragment.root_element().text().collect::<Vec<_>>().join(" ")
}

fn write_err(data: impl Display) -> eyre::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("medpack-err.log")?;

    eprintln!("{data}");

    writeln!(file, "{data}")?;

    Ok(())
}

fn app(args: Cli) -> eyre::Result<()> {
    let app = App::new(args.clone())?;

    if args.preserve_tmp {
        println!(
            "tmp folders: {tmp_html} {tmp_img} {tmp_label}",
            tmp_html = app.tmp_html("").to_string_lossy(),
            tmp_label = app.tmp_label("").to_string_lossy(),
            tmp_img = app.tmp_img("").to_string_lossy(),
        );
    }

    let exports = args
        .sources
        .iter()
        .map(|path| get_export_result(path))
        .collect::<Result<Vec<_>, _>>()?;

    let chat_id = exports.first().map(|export| export.id).unwrap_or_default();

    let mut person_records = Categorizer::process_exports(exports);

    if !args.people.is_empty() {
        person_records.retain(|name, _| args.people.contains(name));
    }

    let prefix_width = person_records
        .keys()
        .map(|name| name.chars().count())
        .max()
        .unwrap_or(10);

    let m = MultiProgress::new();

    let records_len: usize = person_records.values().map(|records| records.len()).sum();
    let progress_width = records_len.to_string().chars().count();

    let pb_total_style = ProgressStyle::with_template(
        &"{spinner:.green} [emoji]{prefix:<[prefix_width].red} | {msg}\n[{elapsed_precise}] {wide_bar:.cyan/blue} {pos:>[progress_width]}/{len:[progress_width]} [{eta_precise}]"
            .replace("[emoji]", &console::Emoji("⌛️", "").to_string())
            .replace("[prefix_width]", &prefix_width.to_string())
            .replace("[progress_width]", &progress_width.to_string()),
    )?;

    // ToC and Unite
    let extra_steps = 2;

    let pb_total = m
        .add(ProgressBar::new(
            (records_len + person_records.len() * extra_steps) as _,
        ))
        .with_style(pb_total_style)
        .with_prefix("total")
        .with_message("total progress of all messages");

    pb_total.enable_steady_tick(Duration::from_millis(100));

    let pb_style = ProgressStyle::with_template(
        &"{spinner:.green} [emoji]{prefix:<[prefix_width].red} | {msg}\n[{elapsed_precise}] {wide_bar:.cyan/blue} {pos:>[progress_width]}/{len:[progress_width]}"
            .replace("[emoji]", &console::Emoji("👤", "").to_string())
            .replace("[prefix_width]", &prefix_width.to_string())
            .replace("[progress_width]", &progress_width.to_string()),
    )?;

    let person_records_with_pbs: HashMap<_, (Vec<Record>, ProgressBar)> = person_records
        .into_iter()
        .map(|(person, records)| {
            let pb = m
                .add(ProgressBar::new((records.len() + extra_steps) as _))
                .with_style(pb_style.clone())
                .with_message("Starting")
                .with_prefix(person.clone());

            pb.enable_steady_tick(Duration::from_millis(100));

            (person, (records, pb))
        })
        .collect();

    let result: Result<Vec<_>, _> = person_records_with_pbs
        .into_par_iter()
        // .filter(|(name, _)| name == "nataly")
        // .take_any(1)
        .map(|(name, (recs, pb))| process_person(&app, &name, chat_id, &recs, &pb, &pb_total))
        .collect();

    pb_total.finish_with_message("everything is done");

    result?;

    Ok(())
}

fn process_message(app: &App, msg: &Message, pb: &ProgressBar) -> eyre::Result<PathBuf> {
    let path = if msg.is_pdf() {
        msg.unwrap_file()
    } else if msg.is_photo() {
        let path_img = app.tmp_img(format!("{}-img.pdf", msg.id));

        command::img2pdf([
            "--imgsize",
            "595x5000",
            "--fit",
            "into",
            &msg.unwrap_photo().to_string_lossy(),
            "-o",
            &path_img.to_string_lossy(),
        ])?;

        path_img
    } else {
        let content = msg.text_entities[1..]
            .iter()
            .map(|entity| entity.to_html())
            .join("");

        PdfTools::from_html(app, msg.id, &content, pb)?
    };

    Ok(path)
}

fn process_record<'a>(
    app: &App,
    chat_id: i64,
    rec: &'a Record,
    pb: &ProgressBar,
) -> eyre::Result<(PathBuf, TocItem<'a>)> {
    let mut pdfs = vec![];

    for msg in &rec.messages {
        pb.set_message(format!("process {} message", msg.id));

        let pdf = process_message(app, msg, pb)?;

        pdfs.push(pdf);

        pb.set_message(format!("complete {} message", msg.id));
    }

    let record_pdf = if pdfs.len() == 1 {
        pdfs.first().cloned().expect("Should be one")
    } else {
        let record_pdf = app.tmp_records(format!("{}.pdf", rec.record_id()));

        pdfs.push(record_pdf.clone());

        command::pdfunite(pdfs)?;

        record_pdf
    };

    // OCR
    let record_pdf = if app.cli().no_ocr || !rec.is_images() {
        record_pdf
    } else {
        pb.set_message(format!("process ocr for {} record", rec.record_id()));
        let path_res = app.tmp_records(format!("{}-ocr.pdf", rec.record_id()));

        let started = Instant::now();

        let ocr = || {
            command::ocrmypdf([
                "-l",
                "rus+eng",
                "-O0",
                "--tesseract-oem",
                "1",
                "--output-type",
                "pdf",
                &record_pdf.to_string_lossy(),
                &path_res.to_string_lossy(),
            ])
        };

        ocr.retry(ConstantBuilder::new().with_max_times(app.cli().ocr_retries))
            .sleep(std::thread::sleep)
            .notify(|err: &eyre::Error, dur: Duration| {
                pb.println(format!(
                    "ocr is failed for {} record. retrying {err:?} after {}",
                    rec.record_id(),
                    HumanDuration(dur)
                ));
            })
            .call()?;

        pb.println(format!(
            "ocr processing for {} record is done in {}",
            rec.record_id(),
            HumanDuration(started.elapsed())
        ));

        pb.set_message(format!(
            "process ocr complete for {} record",
            rec.record_id()
        ));

        path_res
    };

    // Label pdf
    let mut tags = rec
        .tags
        .iter()
        .map(|tag| strip_html_tags(tag))
        .map(|tag| tag.trim().to_string())
        .join(", ");

    if tags.chars().count() > 58 {
        tags = format!("{}...", tags.chars().take(55).collect::<String>());
    }
    let label = format!("{}: {}", tags, &rec.date);

    let paging = "стр %Page из %EndPage".to_string();

    let labeled_pdf = app.tmp_label(format!("{}.pdf", rec.record_id()));
    let labeled_pdf = PdfTools::label(
        &record_pdf,
        &labeled_pdf,
        &paging,
        &label,
        &rec.first_message_id().to_string(),
        &format!("https://t.me/c/{chat_id}/{id}", id = rec.first_message_id()),
    )?;

    let pages = PdfTools::get_pages_count(&labeled_pdf)?;

    Ok((labeled_pdf, TocItem { record: rec, pages }))
}

fn generate_toc_file(
    app: &App,
    person_name: &str,
    toc: Toc,
    pb: &ProgressBar,
) -> eyre::Result<PathBuf> {
    let mut shift = 1;

    let mut output_path = "".into();

    // NOTE: Two iterations are required to properly calculate pages shift
    // as toc can be multipaged
    for _ in ["first", "second"] {
        output_path = PdfTools::from_html(
            app,
            "toc-".to_string() + person_name,
            &toc.generate_html(shift),
            pb,
        )?;

        shift = PdfTools::get_pages_count(&output_path)?;

        if shift == 1 {
            break;
        }
    }

    Ok(output_path)
}

fn process_person(
    app: &App,
    name: &str,
    chat_id: i64,
    recs: &[Record],
    pb: &ProgressBar,
    pb_total: &ProgressBar,
) -> eyre::Result<()> {
    let results = recs
        .par_iter()
        .map(|rec| {
            pb.set_message(format!("process {} record", rec.record_id()));

            let res = process_record(app, chat_id, rec, pb);

            pb.inc(1);
            pb_total.inc(1);

            pb.set_message(format!("complete {} record", rec.record_id()));

            res
        })
        .collect::<Result<Vec<_>, _>>()?;

    let (mut pdfs, toc_items): (Vec<_>, Vec<_>) = results.into_iter().unzip();

    let mut toc = Toc::new(chat_id);
    toc.append(toc_items);

    let toc_path = generate_toc_file(app, name, toc, pb)?;

    pb.inc(1);
    pb_total.inc(1);

    pdfs.insert(0, toc_path);

    pb.set_message(format!("unite {} pdf files", pdfs.len()));

    // Output file as last parameter
    let united_pdf = app.tmp_label(format!("{name}.pdf"));

    pdfs.push(united_pdf.clone());

    command::pdfunite(pdfs)?;

    pb.inc(1);
    pb_total.inc(1);

    let result_pds = format!("{name}.pdf");

    PdfTools::add_page_numbers(&united_pdf, result_pds.as_ref())?;

    pb.finish_with_message(format!("finished - result file {result_pds}"));

    Ok(())
}
