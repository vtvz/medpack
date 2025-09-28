use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use eyre::eyre;
use lazy_static::lazy_static;
use tempdir::TempDir;

use crate::write_err;

pub struct CommandResult {
    cmd: Command,
    output: Output,
}

impl CommandResult {
    pub fn stdout(&self) -> eyre::Result<String> {
        let res = String::from_utf8(self.output.stdout.clone())?;

        Ok(res)
    }

    pub fn stderr(&self) -> eyre::Result<String> {
        let res = String::from_utf8(self.output.stderr.clone())?;

        Ok(res)
    }
}

impl std::fmt::Display for CommandResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cmd = format!("{:?}", self.cmd);
        let stderr = self.stderr().map_err(|_| std::fmt::Error)?;
        let stdout = self.stdout().map_err(|_| std::fmt::Error)?;
        let status = self.output.status.to_string();

        write!(f, "{cmd}\nstatus: {status}\n{stdout}\n{stderr}")
    }
}

pub fn cmd(
    cmd: &str,
    args: impl IntoIterator<Item = impl AsRef<OsStr>>,
) -> eyre::Result<CommandResult> {
    let mut cmd = Command::new(cmd);
    cmd.args(args);

    let res = cmd.output()?;
    let res = CommandResult { cmd, output: res };

    if res.output.status.success() {
        Ok(res)
    } else {
        write_err(&res)?;

        Err(eyre!("Exited with exit code {}", res.output.status))
    }
}

pub fn pdf_info(path: impl AsRef<OsStr> + std::fmt::Debug) -> eyre::Result<CommandResult> {
    cmd("pdfinfo", [path])
}

pub fn img2pdf(
    args: impl IntoIterator<Item = impl AsRef<OsStr>> + std::fmt::Debug,
) -> eyre::Result<CommandResult> {
    cmd("img2pdf", args)
}

pub fn pdfunite(
    pdfs: impl IntoIterator<Item = impl AsRef<OsStr>> + std::fmt::Debug,
) -> eyre::Result<CommandResult> {
    cmd("pdfunite", pdfs)
}

pub fn cpdf(params: impl IntoIterator<Item = impl AsRef<OsStr>>) -> eyre::Result<CommandResult> {
    cmd("cpdf", params)
}

pub fn ocrmypdf(
    params: impl IntoIterator<Item = impl AsRef<OsStr>>,
) -> eyre::Result<CommandResult> {
    cmd("ocrmypdf", params)
}

pub fn wkhtmltopdf(
    args: &[impl AsRef<OsStr> + std::fmt::Debug],
    input: impl AsRef<OsStr>,
    output: impl AsRef<OsStr>,
) -> eyre::Result<CommandResult> {
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
    pub static ref ROBOTO_FONT_FILE: PathBuf = {
        let file_path = TEMP_DIR.path().join("font.ttf");

        let mut tmp_file = fs::File::create(file_path.clone()).unwrap();
        let content = include_bytes!("./assets/Roboto-Medium.ttf");

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
    pub bottom_link: &'a str,
}

pub fn deno(args: DenoArgs) -> eyre::Result<CommandResult> {
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
        "-u",
        args.bottom_link,
        "-f",
        font_path,
    ];

    cmd("deno", args)
}
