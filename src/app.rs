use std::fmt::Display;
use std::path::Path;

use tempdir::TempDir;

type Temp = Box<dyn AsRef<Path> + Sync + Send>;

pub struct App {
    tmp_img: Temp,
    tmp_html: Temp,
    tmp_label: Temp,
    export_path: String,
}

const PRESERVE_TMP_AFTER_COMPLETE: bool = true;

impl App {
    fn generate_tmp(name: &str) -> eyre::Result<Temp> {
        let name = format!("tmp_medpac_{}", name);

        let tmp: Temp = if PRESERVE_TMP_AFTER_COMPLETE {
            Box::new(TempDir::new(&name)?.into_path())
        } else {
            Box::new(TempDir::new(&name)?)
        };

        Ok(tmp)
    }

    pub fn new(export_path: &str) -> eyre::Result<Self> {
        Ok(Self {
            tmp_img: Self::generate_tmp("img")?,
            tmp_html: Self::generate_tmp("html")?,
            tmp_label: Self::generate_tmp("label")?,
            export_path: export_path.trim_end_matches('/').into(),
        })
    }

    fn tmp_file(tmp: impl AsRef<Path>, file: impl Display) -> String {
        let tmp = tmp
            .as_ref()
            .to_path_buf()
            .into_os_string()
            .into_string()
            .unwrap();

        format!("{}/{}", tmp, file)
    }

    pub fn tmp_img(&self, file: impl Display) -> String {
        Self::tmp_file(self.tmp_img.as_ref(), file)
    }

    pub fn tmp_html(&self, file: impl Display) -> String {
        Self::tmp_file(self.tmp_html.as_ref(), file)
    }

    pub fn tmp_label(&self, file: impl Display) -> String {
        Self::tmp_file(self.tmp_label.as_ref(), file)
    }

    pub fn export_path(&self) -> &str {
        &self.export_path
    }
}
