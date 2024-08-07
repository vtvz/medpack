use std::ffi::OsStr;
use std::process::Command;

use eyre::Ok;

pub fn cmd(cmd: &str, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> eyre::Result<String> {
    // println!("{} {:?}", cmd, args);

    let res = Command::new(cmd).args(args).output()?;
    // println!("{}", String::from_utf8(res.stdout.clone())?);
    match res.status.exit_ok() {
        Result::Ok(()) => Ok(String::from_utf8(res.stdout)?),
        Err(err) => {
            println!("{}", String::from_utf8(res.stdout)?);
            println!("{}", String::from_utf8(res.stderr)?);
            Err(err.into())
        },
        // Ok(_) => Ok(String::from_utf8(res.stdout)?),
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

pub fn cpdf(
    args: impl IntoIterator<Item = impl AsRef<OsStr>> + std::fmt::Debug,
) -> eyre::Result<String> {
    cmd("cpdf", args)
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
