#![feature(exit_status_error)]
use std::fs;
use std::path::Path;

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
use crate::command::{img2pdf, pdfunite};
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

    let mut group: Vec<Message> = vec![];
    let mut result = vec![];
    let mut continue_group = false;

    for msg in msgs {
        let mut to_push = false;
        let mut close_prev = false;

        if msg.has_record() {
            to_push = true;
            close_prev = true;

            continue_group = msg.is_photo();
        } else if msg.is_photo() && msg.is_text_empty() && continue_group {
            to_push = true;
        } else {
            continue_group = false;
            close_prev = true;
        }

        if close_prev && !group.is_empty() {
            result.push(group_to_record(group));
            group = vec![];
        }

        if to_push {
            group.push(msg);
        }
    }

    if !group.is_empty() {
        result.push(group_to_record(group));
    }

    result
}

fn get_export_result(path: impl AsRef<Path>) -> eyre::Result<Export> {
    let red = String::from_utf8(fs::read(path)?)?;
    let data: structs::Export = serde_json::from_str(&red)?;
    Ok(data)
}

fn main() -> eyre::Result<()> {
    let res = app();

    let Err(err) = res else { return Ok(()) };

    println!("{:?}", err);

    Ok(())
}

fn app() -> eyre::Result<()> {
    let args: Vec<_> = std::env::args().collect();

    let export_path = args.get(1).cloned().unwrap_or(".".into());
    let app = App::new(&export_path)?;

    let data = get_export_result(format!("{export_path}/result.json"))?;

    let mut json = data
        .messages
        .into_iter()
        .filter(|msg| msg.type_field == "message" && msg.contact_information.is_none())
        .collect_vec();

    json.sort_by_key(|msg| msg.id);

    let grouped_by_topic = json
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
        tmp_html = app.tmp_html(""),
        tmp_label = app.tmp_label(""),
        tmp_img = app.tmp_img(""),
    );

    let result: Result<Vec<_>, _> = collection
        .into_par_iter()
        // .into_iter()
        .map(|(name, recs)| process_person(&app, &name, &recs))
        .collect();

    result?;

    Ok(())
}

fn process_message(app: &App, msg: &Message) -> eyre::Result<String> {
    let path = if msg.is_pdf() {
        format!("{}/{}", app.export_path(), msg.unwrap_file())
    } else if msg.is_photo() {
        let path = app.tmp_img(format!("{}.pdf", msg.id));

        img2pdf([
            "--imgsize",
            "595x5000",
            "--fit",
            "into",
            &format!("{}/{}", app.export_path(), msg.unwrap_photo()),
            "-o",
            &path,
        ])?;

        path
    } else {
        let content = msg.text_entities[1..]
            .iter()
            .map(|entity| entity.to_html())
            .join("");

        PdfTools::from_html(app, msg.id, &content)?
    };

    Ok(path)
}

fn process_record<'a>(app: &App, rec: &'a Record) -> eyre::Result<(Vec<String>, Vec<TocItem<'a>>)> {
    let mut pdfs = vec![];
    let mut toc_items = vec![];
    let mut pages = 0;

    for (i, msg) in rec.messages.iter().enumerate() {
        let pdf = process_message(app, msg)?;

        let label = format!("{}: {}", rec.tags.join(", "), &rec.date);
        // label_pdf(label)?;

        let paging = if msg.is_photo() {
            format!("стр {} из {}", i + 1, rec.messages.len())
        } else {
            "стр %Page из %EndPage".to_string()
        };

        let labeled_pdf = app.tmp_label(format!("{}.pdf", msg.id));
        let labeled_pdf = PdfTools::label(&pdf, &labeled_pdf, &paging, &label, msg.id)?;

        pages += PdfTools::get_pages_count(&pdf)?;
        println!("  - {labeled_pdf}");

        pdfs.push(labeled_pdf);
    }

    toc_items.push(TocItem { record: rec, pages });

    Ok((pdfs, toc_items))
}

fn generate_toc_file(app: &App, person_name: &str, toc: Toc) -> eyre::Result<String> {
    let mut shift = 1;

    let unadaptive = std::env::var("UNADAPTIVE_TEXT_PAGES").is_ok();

    let mut output_path = "".into();

    for _ in 0..2 {
        output_path = PdfTools::from_html(
            app,
            "toc-".to_string() + person_name,
            &toc.generate_html(shift),
        )?;

        shift = PdfTools::get_pages_count(&output_path)?;

        if !unadaptive {
            break;
        }
    }

    Ok(output_path)
}

fn process_person(app: &App, name: &str, recs: &[Record]) -> eyre::Result<()> {
    println!("## Process person {}", name);

    let results = recs
        .par_iter()
        // .iter()
        .enumerate()
        .map(|(i, rec)| {
            println!("{} - {} of {}", name, i + 1, recs.len());

            process_record(app, rec)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let (pdfs, toc_items): (Vec<_>, Vec<_>) = results.into_iter().unzip();

    let mut pdfs = pdfs.into_iter().flatten().collect_vec();

    let mut toc = Toc::new();
    toc_items.into_iter().for_each(|item| toc.append(item));

    let toc_path = generate_toc_file(app, name, toc)?;

    pdfs.insert(0, toc_path);

    println!("{name} - Unite {} pdf files", pdfs.len());

    // Output file as last parameter
    let result_pdf = format!("{}.pdf", name);

    pdfs.push(result_pdf.clone());

    pdfunite(pdfs)?;

    println!("{name} - result file {}\n", result_pdf);

    Ok(())
}
