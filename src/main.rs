#![feature(exit_status_error)]
use std::fs;
use std::path::PathBuf;

use eyre::Ok;
use itertools::Itertools;
use rayon::iter::{
    IndexedParallelIterator,
    IntoParallelIterator,
    IntoParallelRefIterator,
    ParallelIterator,
};
use structs::{Export, Message, Record};

use crate::app::App;
use crate::pdf_tools::PdfTools;
use crate::toc::{Toc, TocItem};

mod app;
mod command;
mod pdf_tools;
mod structs;
mod toc;

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
    let res = app();

    let Err(err) = res else { return Ok(()) };

    println!("{err:?}");

    Err(err)
}

fn app() -> eyre::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();

    // let export_path = args.get(1).cloned().unwrap_or(".".into());
    let export_paths = if args.is_empty() {
        vec![".".to_string()]
    } else {
        args
    };

    let app = App::new()?;

    let exports = export_paths
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

    let collection = grouped_by_topic
        .into_values()
        .flat_map(group_messages)
        .sorted_by_key(|rec| rec.date.clone())
        .rev()
        .map(|rec| (rec.person.clone(), rec))
        .into_group_map();

    println!(
        "{tmp_html} {tmp_img} {tmp_label}",
        tmp_html = app.tmp_html("").to_string_lossy(),
        tmp_label = app.tmp_label("").to_string_lossy(),
        tmp_img = app.tmp_img("").to_string_lossy(),
    );

    let result: Result<Vec<_>, _> = collection
        .into_par_iter()
        // .take_any(1)
        .map(|(name, recs)| process_person(&app, &name, chat_id, &recs))
        .collect();

    result?;

    Ok(())
}

fn process_message(app: &App, msg: &Message) -> eyre::Result<PathBuf> {
    let path = if msg.is_pdf() {
        msg.unwrap_file()
    } else if msg.is_photo() {
        let path_img = app.tmp_img(format!("{}-img.pdf", msg.id));
        let to_ocr = true;

        command::img2pdf([
            "--imgsize",
            "595x5000",
            "--fit",
            "into",
            &msg.unwrap_photo().to_string_lossy(),
            "-o",
            &path_img.to_string_lossy(),
        ])?;

        if !to_ocr {
            path_img
        } else {
            let path_res = app.tmp_img(format!("{}-ocr.pdf", msg.id));

            command::ocrmypdf([
                "-l",
                "rus+eng",
                &path_img.to_string_lossy(),
                &path_res.to_string_lossy(),
            ])?;

            path_res
        }
    } else {
        let content = msg.text_entities[1..]
            .iter()
            .map(|entity| entity.to_html())
            .join("");

        PdfTools::from_html(app, msg.id, &content)?
    };

    Ok(path)
}

fn process_record<'a>(
    app: &App,
    chat_id: i64,
    rec: &'a Record,
) -> eyre::Result<(Vec<PathBuf>, Vec<TocItem<'a>>)> {
    let mut pdfs = vec![];
    let mut toc_items = vec![];
    let mut pages = 0;

    for (i, msg) in rec.messages.iter().enumerate() {
        let pdf = process_message(app, msg)?;

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
        println!(
            "  - {labeled_pdf}",
            labeled_pdf = labeled_pdf.to_string_lossy()
        );

        pdfs.push(labeled_pdf);
    }

    toc_items.push(TocItem { record: rec, pages });

    Ok((pdfs, toc_items))
}

fn generate_toc_file(app: &App, person_name: &str, toc: Toc) -> eyre::Result<PathBuf> {
    let mut shift = 1;

    let mut output_path = "".into();

    for _ in ["first", "second"] {
        output_path = PdfTools::from_html(
            app,
            "toc-".to_string() + person_name,
            &toc.generate_html(shift),
        )?;

        shift = PdfTools::get_pages_count(&output_path)?;

        if shift == 1 {
            break;
        }
    }

    Ok(output_path)
}

fn process_person(app: &App, name: &str, chat_id: i64, recs: &[Record]) -> eyre::Result<()> {
    println!("## Process person {name}");

    let results = recs
        .par_iter()
        .enumerate()
        .map(|(i, rec)| {
            println!("{} - {} of {}", name, i + 1, recs.len());

            process_record(app, chat_id, rec)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let (pdfs, toc_items): (Vec<_>, Vec<_>) = results.into_iter().unzip();

    let mut pdfs = pdfs.into_iter().flatten().collect_vec();

    let mut toc = Toc::new(chat_id);
    toc_items.into_iter().for_each(|item| toc.append(item));

    let toc_path = generate_toc_file(app, name, toc)?;

    pdfs.insert(0, toc_path);

    println!("{name} - Unite {} pdf files", pdfs.len());

    // Output file as last parameter
    let united_pdf = app.tmp_label(format!("{name}.pdf"));

    pdfs.push(united_pdf.clone());

    command::pdfunite(pdfs)?;

    let result_pds = format!("{name}.pdf");

    PdfTools::add_pages(&united_pdf, result_pds.as_ref())?;

    println!("{name} - result file {result_pds}\n");

    Ok(())
}
