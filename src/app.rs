use std::path::{Path, PathBuf};

use tempdir::TempDir;

use crate::Cli;

type Temp = Box<dyn AsRef<Path> + Sync + Send>;

pub struct App {
    tmp_img: Temp,
    tmp_html: Temp,
    tmp_label: Temp,
    pub process_ocr: bool,
}

impl App {
    fn generate_tmp(name: &str, preserve: bool) -> eyre::Result<Temp> {
        let name = format!("tmp_medpack_{name}");

        let tmp: Temp = if preserve {
            Box::new(TempDir::new(&name)?.into_path())
        } else {
            Box::new(TempDir::new(&name)?)
        };

        Ok(tmp)
    }

    pub fn new(cli: Cli) -> eyre::Result<Self> {
        Ok(Self {
            tmp_img: Self::generate_tmp("img", cli.preserve_tmp)?,
            tmp_html: Self::generate_tmp("html", cli.preserve_tmp)?,
            tmp_label: Self::generate_tmp("label", cli.preserve_tmp)?,
            process_ocr: !cli.no_ocr,
        })
    }

    fn tmp_file(tmp: impl AsRef<Path>, file: impl AsRef<Path>) -> PathBuf {
        let mut tmp = tmp.as_ref().to_path_buf();

        tmp.push(file);

        tmp
    }

    pub fn tmp_img(&self, file: impl AsRef<Path>) -> PathBuf {
        Self::tmp_file(self.tmp_img.as_ref(), file)
    }

    pub fn tmp_html(&self, file: impl AsRef<Path>) -> PathBuf {
        Self::tmp_file(self.tmp_html.as_ref(), file)
    }

    pub fn tmp_label(&self, file: impl AsRef<Path>) -> PathBuf {
        Self::tmp_file(self.tmp_label.as_ref(), file)
    }
}
