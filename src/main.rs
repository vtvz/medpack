#![feature(exit_status_error)]
use std::fs;
use std::path::Path;

use eyre::Ok;
use itertools::Itertools;
use regex::Regex;
use structs::{Export, Message, Record, TocItem};

use crate::command::cmd;
use crate::pdf_tools::text_to_pdf;
use crate::structs::App;

mod command;
mod pdf_tools;
mod structs;

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

fn get_pdf_pages(path: &str) -> eyre::Result<u8> {
    let out = cmd("/usr/bin/pdfinfo", [path])?;
    let re = Regex::new(r"(?m)^Pages:\s+(\d+)$")?;

    let cap = re.captures(&out).ok_or(eyre::eyre!("Need captures"))?;

    let page = cap
        .get(1)
        .ok_or(eyre::eyre!("Need 1 capture"))?
        .as_str()
        .parse()?;

    Ok(page)
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

    let export_path = args.get(1).expect("must provide path to exported data");
    let app = App::new(export_path)?;

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

    for (name, recs) in collection.iter() {
        process_person(&app, name, recs)?;
    }

    Ok(())
}

fn process_message(app: &App, msg: &Message) -> eyre::Result<String> {
    let path = if msg.is_pdf() {
        format!("{}/{}", app.export_path(), msg.file.as_ref().unwrap())
    } else if msg.is_photo() {
        // img2pdf --imgsize 595x5000 --fit into {} -o "${TMP}/{/.}.img.pdf"
        let path = app.tmp_img(format!("{}.pdf", msg.id));

        cmd(
            "/usr/bin/img2pdf",
            [
                "--imgsize",
                "595x5000",
                "--fit",
                "into",
                &format!("{}/{}", app.export_path(), &msg.photo.as_ref().unwrap()),
                "-o",
                &path,
            ],
        )?;

        path
    } else {
        let content = msg.text_entities[1..]
            .iter()
            .map(|entity| entity.to_html())
            .join("");

        text_to_pdf(app, msg.id, &content)?
    };

    Ok(path)
}

fn process_record<'a>(app: &App, rec: &'a Record) -> eyre::Result<(Vec<String>, Vec<TocItem<'a>>)> {
    let mut pdfs = vec![];
    let mut toc_items = vec![];
    let mut pages = 0;
    // let toc_item = format!("{}: {}", &rec.date, rec.tags.join(", "));
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
        let labeled_pdf = pdf_tools::label_pdf(&pdf, &labeled_pdf, &paging, &label, msg.id)?;

        pages += get_pdf_pages(&pdf)?;
        println!("  - {labeled_pdf}");

        pdfs.push(labeled_pdf);
    }

    toc_items.push(TocItem { record: rec, pages });

    Ok((pdfs, toc_items))
}

fn generate_toc_file(app: &App, toc_items: &[TocItem]) -> eyre::Result<String> {
    let mut shift = 0;
    let mut output_path = "".into();

    for _ in 0..2 {
        let mut current_page = shift;
        let content = toc_items
            .iter()
            .map(|item| {
                current_page += item.pages;
                format!(
                    r#"
                    <tr>
                        <td>{}</td>
                        <td style="width: 100%"><ul><li>{}</li></ul></td>
                        <td style="text-align: right"> {}</td>
                    </tr>
                    "#,
                    item.record.date,
                    item.record.tags.join("</li><li>"),
                    current_page - item.pages + 1,
                )
            })
            .join("");

        output_path = text_to_pdf(
            app,
            "toc",
            &format!(
                r#"
                <table class="table table-striped table-sm">
                    <tr class="thead-dark">
                        <th style="text-align: left">date</th>
                        <th style="width: 100%; text-align: left">info</th>
                        <th style="text-align: right">#</th>
                    </tr>
                    {content}
                </table>
                "#
            ),
        )?;

        shift = get_pdf_pages(&output_path)?;
    }

    Ok(output_path)
}

fn process_person(app: &App, name: &str, recs: &[Record]) -> eyre::Result<()> {
    println!("## Process person {}", name);

    let mut pdfs = vec![];
    let mut toc_items = vec![];

    for (i, rec) in recs.iter().enumerate() {
        println!("{} - {} of {}", name, i + 1, recs.len());

        let (out_pdfs, out_toc_items) = process_record(app, rec)?;

        pdfs.extend(out_pdfs);
        toc_items.extend(out_toc_items);
    }

    let toc_path = generate_toc_file(app, &toc_items)?;

    pdfs.insert(0, toc_path);

    println!("{name} - Unite {} pdf files", pdfs.len());

    // Output file as last parameter
    let result_pdf = format!("{}.pdf", name);

    pdfs.push(result_pdf.clone());

    cmd("pdfunite", pdfs)?;

    println!("{name} - result file {}\n", result_pdf);

    Ok(())
}
