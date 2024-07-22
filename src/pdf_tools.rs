use std::fmt::Display;
use std::fs;

use eyre::Ok;

use crate::command::{self, cmd, cpdf};
use crate::structs::App;

pub fn text_to_pdf(app: &App, slug: impl Display, content: &str) -> eyre::Result<String> {
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
                <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bootstrap@4.0.0/dist/css/bootstrap.min.css" integrity="sha384-Gn5384xqQ1aoWXA+058RXPxPg6fy4IWvTNh0E263XmFcJlSAwiGgFAW/dAiS6JXm" crossorigin="anonymous">
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

    command::wkhtmltopdf(
        &[
            "--encoding",
            "utf-8",
            "--zoom",
            "1.4",
            "--page-width",
            "210mm",
            "--disable-smart-shrinking",
        ],
        &path,
        &output_path,
    )?;

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
