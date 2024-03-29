use std::fmt::Display;
use std::fs;

use eyre::Ok;

use crate::command::cmd;
use crate::structs::App;

pub fn text_to_pdf(app: &App, slug: impl Display, content: &str) -> eyre::Result<String> {
    let css_style = r#"
    html * {
        font-family: "DejaVu Sans", sans-serif;
        line-height: 1.1;
    }

/*
    table {
        border-collapse: collapse;
        width: 100%;
    }

    td {
        padding: 4px;
    }

    th {
        padding: 4px;
        background-color: #54585d;
        color: #ffffff;
        border: 1px solid #54585d;
    }

    tr {
        background-color: #dddddd;
        border: 1px solid #1A1D23;
        border-top: none;
    }

    table tr:nth-child(odd) {
        background-color: #ffffff;
    }
    */

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

    cmd(
        "wkhtmltopdf",
        ["--encoding", "utf-8", "--zoom", "1.4", &path, &output_path],
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

    cmd(
        "/usr/bin/cpdf",
        [
            &in_path,
            "-add-text",
            &bottom.to_string(),
            "-bottom",
            "5",
            "-load-ttf",
            "Roboto=/usr/local/share/fonts/Roboto/Roboto-Regular.ttf",
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
            "Roboto=/usr/local/share/fonts/Roboto/Roboto-Regular.ttf",
            "-font",
            "Roboto",
            "-font-size",
            "12",
            "-color",
            &outline_color,
            "-outline",
            "-linewidth",
            "1.5",
            "AND",
            "-add-text",
            &right.to_string(),
            "-topright",
            "10 15",
            "-load-ttf",
            "Roboto=/usr/local/share/fonts/Roboto/Roboto-Regular.ttf",
            "-font",
            "Roboto",
            "-font-size",
            "12",
            "-color",
            &text_color,
            "AND",
            "-add-text",
            &left.to_string(),
            "-topleft",
            "10 15",
            "-load-ttf",
            "Roboto=/usr/local/share/fonts/Roboto/Roboto-Regular.ttf",
            "-font",
            "Roboto",
            "-font-size",
            "12",
            "-color",
            &outline_color,
            "-outline",
            "-linewidth",
            "1",
            "AND",
            "-add-text",
            &left.to_string(),
            "-topleft",
            "10 15",
            "-load-ttf",
            "Roboto=/usr/local/share/fonts/Roboto/Roboto-Regular.ttf",
            "-font",
            "Roboto",
            "-font-size",
            "12",
            "-color",
            &text_color,
            "-o",
            &out_path,
        ],
    )?;

    Ok(out_path.into())
}
