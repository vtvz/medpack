use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};

use eyre::Ok;
use regex::Regex;

use crate::app::App;
use crate::command::{self, DenoArgs, ROBOTO_FONT_FILE};

pub struct PdfTools;

impl PdfTools {
    pub fn from_html(app: &App, slug: impl Display, content: &str) -> eyre::Result<PathBuf> {
        let bootstrap = include_str!("assets/bootstrap-v4.6.2.min.css");

        let css_style = r#"
        html * {
            font-family: "DejaVu Sans", sans-serif;
            line-height: 1.1;
        }

        ul {
            margin: 0;
            padding-left: 30px;
        }

        .small-font {
            font-size: 0.8em;
        }

        .message-id {
            font-size: 0.5em;
        }

        .message-id a {
            color: #c6c6c6;
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

        let path = app.tmp_html(format!("{slug}.html"));
        // let path = format!("test/{}.html", slug);
        let output_path = app.tmp_html(format!("{slug}.pdf"));

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
                    &format!("{height}mm"),
                    "-T",
                    &format!("{margin}mm"),
                    "-B",
                    &format!("{margin}mm"),
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

            println!(
                "{} shrink. Size {new_page_height}mm. Pages {pages}",
                output_path.to_string_lossy(),
            );

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

            println!(
                "{} expand. Size {new_page_height}mm. Pages {pages}",
                output_path.to_string_lossy()
            );
        }

        println!(
            "{} ready. Size {new_page_height}mm. Pages {pages}",
            output_path.to_string_lossy()
        );

        Ok(output_path)
    }

    pub fn add_pages(in_path: &Path, out_path: &Path) -> eyre::Result<PathBuf> {
        let font_path = ROBOTO_FONT_FILE.to_str().unwrap();
        let font = format!("Roboto={font_path}");

        let args: Vec<&dyn AsRef<std::ffi::OsStr>> = vec![
            &"-add-text",
            &"ст. %Page",
            &"-bottomright",
            &"5 5",
            &"-load-ttf",
            &font,
            &"-font",
            &"Roboto",
            &"-font-size",
            &"11",
            &in_path,
            &"-o",
            &out_path,
        ];

        command::cpdf(args)?;

        Ok(out_path.to_path_buf())
    }

    pub fn label(
        in_path: &Path,
        out_path: &Path,
        left_text: &str,
        right_text: &str,
        bottom_text: &str,
        bottom_link: &str,
    ) -> eyre::Result<PathBuf> {
        // let text_color = "black";
        // let outline_color = "white";

        // let font_path = cmd("fc-list", ["Roboto:style=Regular", "-f", "%{file}"])?;
        // let font_arg = format!("Roboto={font_path}");

        command::deno(DenoArgs {
            in_path,
            out_path,
            left_text,
            right_text,
            bottom_text,
            bottom_link,
        })?;

        Ok(out_path.to_path_buf())
    }

    pub fn get_pages_count(path: &PathBuf) -> eyre::Result<u8> {
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
