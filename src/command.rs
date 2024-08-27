use std::ffi::OsStr;
use std::process::Command;

use eyre::{eyre, Ok};

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

pub fn deno(
    args: impl IntoIterator<Item = impl AsRef<OsStr>> + std::fmt::Debug,
) -> eyre::Result<String> {
    let mut new_args: Vec<&OsStr> = vec!["run", "--allow-read", "--allow-write"]
        .into_iter()
        .map(AsRef::as_ref)
        .collect();

    // make borrow checker happy
    let args: Vec<_> = args.into_iter().collect();

    new_args.extend(args.iter().map(|item| item.as_ref()));

    cmd("deno", new_args)
}
