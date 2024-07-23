use std::fmt::Display;
use std::fs;

use eyre::Ok;

use crate::command::{self, cmd, cpdf};
use crate::get_pdf_pages;
use crate::structs::App;

pub fn text_to_pdf(app: &App, slug: impl Display, content: &str) -> eyre::Result<String> {
    let bootstrap = include_str!("bootstrap-v4.6.2.min.css");
    let css_style = r#"
    html * {
        font-family: "DejaVu Sans", sans-serif;
        line-height: 1.1;
    }

    ul {
        margin: 0;
        padding-left: 30px;
    }
    "#;

    let content = format!(
        r#"
        <!doctype html>
        <html>
            <head>
                <title>Page Title</title>
                <style>{bootstrap}</style>
                <style>{css_style}</style>
            </head>
            <body>
                {content}
            </body>
        </html>
        "#
    );

    let path = app.tmp_html(format!("{}.html", slug));
    // let path = format!("test/{}.html", slug);
    let output_path = app.tmp_html(format!("{}.pdf", slug));

    fs::write(&path, content).expect("Should have been able to read the file");

    let page_chunk_height = 50;
    let page_margin = 10;
    let generate_file = |height: u64, margin: u64| {
        command::wkhtmltopdf(
            &[
                "--encoding",
                "utf-8",
                "--zoom",
                "1.4",
                "--page-width",
                "210mm",
                "--page-height",
                &format!("{}mm", height),
                "-T",
                &format!("{}mm", margin),
                "-B",
                &format!("{}mm", margin),
            ],
            &path,
            &output_path,
        )
    };

    // Create PDF file with small pages to have an idea what the size on the whole page
    generate_file(page_chunk_height, 0)?;

    let pages = get_pdf_pages(&output_path)? as u64;

    let mut new_page_height = pages * page_chunk_height + page_margin * 2;
    let mut pages = 1;

    // Shrink the page 'til it splits into two
    while pages == 1 && new_page_height >= (page_chunk_height + page_margin * 2) {
        generate_file(new_page_height, page_margin)?;

        pages = get_pdf_pages(&output_path)? as _;

        if pages == 1 {
            new_page_height -= page_chunk_height;
        }

        println!("{} dec {} {}", output_path, new_page_height, pages);
    }

    // Increase size by small steps to fit all content in one page
    loop {
        pages = get_pdf_pages(&output_path)? as _;

        if pages == 1 {
            break;
        }

        generate_file(new_page_height, page_margin)?;

        new_page_height += 10;

        println!("{} inc {} {}", output_path, new_page_height, pages);
    }

    Ok(output_path)
}

pub fn label_pdf(
    in_path: &str,
    out_path: &str,
    left: impl Display,
    right: impl Display,
    bottom: impl Display,
) -> eyre::Result<String> {
    let text_color = "black";
    let outline_color = "white";

    let font_path = cmd("fc-list", ["Roboto:style=Regular", "-f", "%{file}"])?;
    let font_arg = format!("Roboto={font_path}");

    cpdf([
        in_path,
        "-add-text",
        &bottom.to_string(),
        "-bottom",
        "5",
        "-load-ttf",
        &font_arg,
        "-font",
        "Roboto",
        "-font-size",
        "5",
        "-color",
        "0.5",
        "AND",
        "-add-text",
        &right.to_string(),
        "-topright",
        "10 15",
        "-load-ttf",
        &font_arg,
        "-font",
        "Roboto",
        "-font-size",
        "12",
        "-color",
        outline_color,
        "-outline",
        "-linewidth",
        "1.5",
        "AND",
        "-add-text",
        &right.to_string(),
        "-topright",
        "10 15",
        "-load-ttf",
        &font_arg,
        "-font",
        "Roboto",
        "-font-size",
        "12",
        "-color",
        text_color,
        "AND",
        "-add-text",
        &left.to_string(),
        "-topleft",
        "10 15",
        "-load-ttf",
        &font_arg,
        "-font",
        "Roboto",
        "-font-size",
        "12",
        "-color",
        outline_color,
        "-outline",
        "-linewidth",
        "1",
        "AND",
        "-add-text",
        &left.to_string(),
        "-topleft",
        "10 15",
        "-load-ttf",
        &font_arg,
        "-font",
        "Roboto",
        "-font-size",
        "12",
        "-color",
        text_color,
        "-o",
        out_path,
    ])?;

    Ok(out_path.into())
}
