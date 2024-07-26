use std::fmt::Display;
use std::fs;

use eyre::Ok;
use regex::Regex;

use crate::app::App;
use crate::command::{self, cmd, cpdf};

pub struct PdfTools;

impl PdfTools {
    pub fn from_html(app: &App, slug: impl Display, content: &str) -> eyre::Result<String> {
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

        let page_margin = 10;

        if std::env::var("UNADAPTIVE_TEXT_PAGES").is_ok() {
            generate_file(297, page_margin)?;

            return Ok(output_path);
        }

        let page_chunk_height = 40;

        // Create PDF file with small pages to have an idea what the size on the whole page
        generate_file(page_chunk_height, 0)?;

        let pages = Self::get_pages_count(&output_path)? as u64;

        let mut new_page_height = pages * page_chunk_height + page_margin * 2;
        let mut pages = 1;

        // Shrink the page 'til it splits into two
        while pages == 1 && new_page_height >= (page_chunk_height + page_margin * 2) {
            generate_file(new_page_height, page_margin)?;

            pages = Self::get_pages_count(&output_path)? as _;

            println!("{output_path} shrink. Size {new_page_height}mm. Pages {pages}");

            if pages == 1 {
                new_page_height -= page_chunk_height;
            }
        }

        // Increase size by small steps to fit all content in one page
        loop {
            if pages == 1 {
                break;
            }

            new_page_height += 10;

            generate_file(new_page_height, page_margin)?;

            pages = Self::get_pages_count(&output_path)? as _;

            println!("{output_path} expand. Size {new_page_height}mm. Pages {pages}");
        }

        println!("{output_path} ready. Size {new_page_height}mm. Pages {pages}");

        Ok(output_path)
    }

    pub fn label(
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

    pub fn get_pages_count(path: &str) -> eyre::Result<u8> {
        let out = command::pdf_info(path)?;
        let re = Regex::new(r"(?m)^Pages:\s+(\d+)$")?;

        let cap = re.captures(&out).ok_or(eyre::eyre!("Need captures"))?;

        let page = cap
            .get(1)
            .ok_or(eyre::eyre!("Need 1 capture"))?
            .as_str()
            .parse()?;

        Ok(page)
    }
}
