use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use eyre::{eyre, Ok};
use lazy_static::lazy_static;
use tempdir::TempDir;

pub fn cmd(cmd: &str, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> eyre::Result<String> {
    let res = Command::new(cmd).args(args).output()?;

    if res.status.success() {
        Ok(String::from_utf8(res.stdout)?)
    } else {
        println!("{}", String::from_utf8(res.stdout)?);
        println!("{}", String::from_utf8(res.stderr)?);
        Err(eyre!("Exited with exit code {}", res.status))
    }
}

pub fn pdf_info(path: impl AsRef<OsStr> + std::fmt::Debug) -> eyre::Result<String> {
    cmd("pdfinfo", [path])
}

pub fn img2pdf(
    args: impl IntoIterator<Item = impl AsRef<OsStr>> + std::fmt::Debug,
) -> eyre::Result<String> {
    cmd("img2pdf", args)
}

pub fn pdfunite(
    pdfs: impl IntoIterator<Item = impl AsRef<OsStr>> + std::fmt::Debug,
) -> eyre::Result<String> {
    cmd("pdfunite", pdfs)
}

pub fn wkhtmltopdf(
    args: &[impl AsRef<OsStr> + std::fmt::Debug],
    input: impl AsRef<OsStr>,
    output: impl AsRef<OsStr>,
) -> eyre::Result<String> {
    let mut new_args = Vec::from_iter(args.iter().map(|arg| arg.as_ref()));

    new_args.push(input.as_ref());
    new_args.push(output.as_ref());

    cmd("wkhtmltopdf", new_args)
}

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

pub struct DenoArgs<'a> {
    pub in_path: &'a Path,
    pub out_path: &'a Path,
    pub left_text: &'a str,
    pub right_text: &'a str,
    pub bottom_text: &'a str,
}

pub fn deno(args: DenoArgs) -> eyre::Result<String> {
    let deno_file = DENO_FILE.to_str().unwrap();
    let font_path = ROBOTO_FONT_FILE.to_str().unwrap();

    let args = [
        "run",
        "--allow-read",
        "--allow-write",
        deno_file,
        "-i",
        &args.in_path.to_string_lossy(),
        "-o",
        &args.out_path.to_string_lossy(),
        "-l",
        args.left_text,
        "-r",
        args.right_text,
        "-b",
        args.bottom_text,
        "-f",
        font_path,
    ];

    cmd("deno", args)
}
