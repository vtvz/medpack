use std::fmt::Display;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use eyre::Ok;
use lazy_static::lazy_static;
use regex::Regex;
use tempdir::TempDir;

use crate::app::App;
use crate::command;

lazy_static! {
    static ref TEMP_DIR: TempDir = TempDir::new("tmp_medpack_assets").unwrap();
    static ref DENO_FILE: PathBuf = {
        let file_path = TEMP_DIR.path().join("index.ts");

        let mut tmp_file = fs::File::create(file_path.clone()).unwrap();
        let content = include_bytes!("assets/index.ts");

        tmp_file.write_all(content).unwrap();

        file_path
    };
    static ref ROBOTO_FONT_FILE: PathBuf = {
        let file_path = TEMP_DIR.path().join("Roboto-Regular.ttf");

        let mut tmp_file = fs::File::create(file_path.clone()).unwrap();
        let content = include_bytes!("./assets/Roboto-Regular.ttf");

        tmp_file.write_all(content).unwrap();

        file_path
    };
}

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

    pub fn label(
        in_path: &Path,
        out_path: &Path,
        left: impl Display,
        right: impl Display,
        bottom: impl Display,
    ) -> eyre::Result<PathBuf> {
        // let text_color = "black";
        // let outline_color = "white";

        // let font_path = cmd("fc-list", ["Roboto:style=Regular", "-f", "%{file}"])?;
        // let font_arg = format!("Roboto={font_path}");

        let deno_file = DENO_FILE.to_str().unwrap();
        let font_path = ROBOTO_FONT_FILE.to_str().unwrap();

        command::deno([
            deno_file,
            "-i",
            &in_path.to_string_lossy(),
            "-o",
            &out_path.to_string_lossy(),
            "-l",
            &left.to_string(),
            "-r",
            &right.to_string(),
            "-b",
            &bottom.to_string(),
            "-f",
            font_path,
        ])?;

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
