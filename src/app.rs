use std::path::{Path, PathBuf};

use tempdir::TempDir;

type Temp = Box<dyn AsRef<Path> + Sync + Send>;

pub struct App {
    tmp_img: Temp,
    tmp_html: Temp,
    tmp_label: Temp,
}

impl App {
    fn generate_tmp(name: &str) -> eyre::Result<Temp> {
        let name = format!("tmp_medpack_{name}");
        let preserve = std::env::var("PRESERVE_TMP").is_ok();

        let tmp: Temp = if preserve {
            Box::new(TempDir::new(&name)?.into_path())
        } else {
            Box::new(TempDir::new(&name)?)
        };

        Ok(tmp)
    }

    pub fn new() -> eyre::Result<Self> {
        Ok(Self {
            tmp_img: Self::generate_tmp("img")?,
            tmp_html: Self::generate_tmp("html")?,
            tmp_label: Self::generate_tmp("label")?,
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
