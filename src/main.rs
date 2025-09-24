#![feature(exit_status_error)]
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::Parser;
use eyre::Ok;
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::app::App;
use crate::pdf_tools::PdfTools;
use crate::structs::{Export, Message, Record};
use crate::toc::{Toc, TocItem};

mod app;
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

    /// Do not shrink or extend text pages (including toc)
    #[arg(long)]
    unadaptive_text_pages: bool,

    /// Source locations
    #[arg(default_values_t = vec![".".to_string()])]
    sources: Vec<String>,
}

fn group_to_record(group: Vec<Message>) -> Record {
    let mut record = group
        .first()
        .expect("Group shouldn't empty")
        .get_record()
        .expect("Group must be pre-parsed");

    record.messages = group;

    record
}

fn group_messages(mut msgs: Vec<Message>) -> Vec<Record> {
    msgs.sort_by_key(|msg| msg.id);

    // Group is a collection of related messages
    let mut group: Vec<Message> = vec![];
    let mut records = vec![];
    let mut continue_group = false;

    for msg in msgs {
        // Will this messages be pushed to group
        let to_push;

        // If true will create record with grouped messages
        // `group` variable will be emptied
        let close_prev;

        // Record in msg is an `yaml` block with metadata
        if msg.has_record() {
            to_push = true;
            close_prev = true;

            // Image with record could have following image
            continue_group = msg.is_photo();
        } else if msg.is_photo() && msg.is_text_empty() && continue_group {
            // Image without record can be a part of group
            // True if group continues
            to_push = true;
            close_prev = false;
        } else {
            // Only images can create group.
            // Text and PDFs without Record won't be added to document
            to_push = false;
            close_prev = true;

            continue_group = false;
        }

        if close_prev && !group.is_empty() {
            records.push(group_to_record(group));
            group = vec![];
        }

        if to_push {
            group.push(msg);
        }
    }

    if !group.is_empty() {
        records.push(group_to_record(group));
    }

    records
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

fn write_err(data: impl Display) -> eyre::Result<()> {
    let mut file = OpenOptions::new().append(true).open("medpack-err.log")?;

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

    let messages = exports
        .into_iter()
        .flat_map(|export| export.messages)
        .filter(|msg| msg.type_field == "message" && msg.contact_information.is_none())
        .sorted_by_key(|msg| (msg.id, msg.date, msg.edited.unwrap_or_default()))
        .rev()
        .dedup_by(|a, b| a.id == b.id)
        .collect_vec();

    // I do this for consistency as messages in different topics can interfere with each other
    let grouped_by_topic = messages
        .into_iter()
        .map(|msg| (msg.reply_to_message_id, msg))
        .into_group_map();

    let person_records = grouped_by_topic
        .into_values()
        .flat_map(group_messages)
        .sorted_by_key(|rec| rec.date.clone())
        .rev()
        .map(|rec| (rec.person.clone(), rec))
        .into_group_map();

    let messages_len: usize = person_records
        .values()
        .flat_map(|records| records.iter())
        .map(|record| record.messages.len())
        .sum();

    let prefix_width = person_records
        .keys()
        .map(|name| name.chars().count())
        .max()
        .unwrap_or(10)
        + 1; // 1 for emoji

    let m = MultiProgress::new();

    let pb_total_style = ProgressStyle::with_template(
        &"{spinner:.green} [emoji]{prefix:<[prefix_width].red} : {msg}\n[{elapsed_precise}] {wide_bar:.cyan/blue} {pos:>[progress_width]}/{len:[progress_width]} [{eta_precise}]"
            .replace("[emoji]", &console::Emoji("⌛️", "").to_string())
            .replace("[prefix_width]", &prefix_width.to_string())
            .replace("[progress_width]", &messages_len.to_string().chars().count().to_string()),
    )?;

    let extra_steps = 2;

    let pb_total = m
        .add(ProgressBar::new(
            (messages_len + person_records.len() * extra_steps) as _, // 2 per person for toc and unite
        ))
        .with_style(pb_total_style.clone())
        .with_prefix("total")
        .with_message("total progress of all messages");

    pb_total.enable_steady_tick(Duration::from_millis(100));

    let pb_style = ProgressStyle::with_template(
        &"{spinner:.green} [emoji]{prefix:<[prefix_width].red} : {msg}\n[{elapsed_precise}] {wide_bar:.cyan/blue} {pos:>[progress_width]}/{len:[progress_width]}"
            .replace("[emoji]", &console::Emoji("👤", "").to_string())
            .replace("[prefix_width]", &prefix_width.to_string())
            .replace("[progress_width]", &messages_len.to_string().chars().count().to_string()),
    )?;

    let person_records_with_pbs: HashMap<_, (Vec<Record>, ProgressBar)> = person_records
        .into_iter()
        .map(|(person, records)| {
            let messages_len: usize = records.iter().map(|rec| rec.messages.len()).sum();
            let pb = m
                .add(ProgressBar::new((messages_len + extra_steps) as _)) // 2 for toc and unite
                .with_style(pb_style.clone())
                .with_message("Starting")
                .with_prefix(person.clone());

            pb.enable_steady_tick(Duration::from_millis(100));

            (person, (records, pb))
        })
        .collect();

    let result: Result<Vec<_>, _> = person_records_with_pbs
        .into_par_iter()
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

        if !app.process_ocr {
            path_img
        } else {
            pb.set_message(format!("process ocr for {} message", msg.id));
            let path_res = app.tmp_img(format!("{}-ocr.pdf", msg.id));

            let started = Instant::now();

            command::ocrmypdf([
                "-l",
                "rus+eng",
                &path_img.to_string_lossy(),
                &path_res.to_string_lossy(),
            ])?;

            pb.println(format!(
                "ocr processing for {} message is done in {}",
                msg.id,
                HumanDuration(started.elapsed())
            ));

            pb.set_message(format!("process ocr complete for {} message", msg.id));

            path_res
        }
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
    pb_total: &ProgressBar,
) -> eyre::Result<(Vec<PathBuf>, Vec<TocItem<'a>>)> {
    let mut pdfs = vec![];
    let mut toc_items = vec![];
    let mut pages = 0;

    for (i, msg) in rec.messages.iter().enumerate() {
        pb.set_message(format!("process {} message", msg.id));

        let pdf = process_message(app, msg, pb)?;

        let mut tags = rec.tags.join(", ");
        if tags.chars().count() > 58 {
            tags = format!("{}...", tags.chars().take(55).collect::<String>());
        }
        let label = format!("{}: {}", tags, &rec.date);

        let paging = if msg.is_photo() {
            format!("стр {} из {}", i + 1, rec.messages.len())
        } else {
            "стр %Page из %EndPage".to_string()
        };

        let labeled_pdf = app.tmp_label(format!("{}.pdf", msg.id));
        let labeled_pdf = PdfTools::label(
            &pdf,
            &labeled_pdf,
            &paging,
            &label,
            &msg.id.to_string(),
            &format!("https://t.me/c/{chat_id}/{id}", id = msg.id),
        )?;

        pages += PdfTools::get_pages_count(&pdf)?;

        pdfs.push(labeled_pdf);

        pb.inc(1);
        pb_total.inc(1);

        pb.set_message(format!("complete {} message", msg.id));
    }

    toc_items.push(TocItem { record: rec, pages });

    Ok((pdfs, toc_items))
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
            pb.set_message(format!("process {} record", rec.date));

            let res = process_record(app, chat_id, rec, pb, pb_total);

            pb.set_message(format!("complete {} record", rec.date));

            res
        })
        .collect::<Result<Vec<_>, _>>()?;

    let (pdfs, toc_items): (Vec<_>, Vec<_>) = results.into_iter().unzip();

    let mut pdfs = pdfs.into_iter().flatten().collect_vec();

    let mut toc = Toc::new(chat_id);
    toc_items.into_iter().for_each(|item| toc.append(item));

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
