use std::ffi::OsStr;
use std::process::Command;

use eyre::Ok;

pub fn cmd(cmd: &str, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> eyre::Result<String> {
    let res = Command::new(cmd).args(args).output()?;
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
