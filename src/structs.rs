use std::fmt::Display;
use std::path::Path;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use tempdir::TempDir;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TextEntity {
    Bold { text: String },
    Code { text: String },
    CustomEmoji { text: String, document_id: String },
    Hashtag { text: String },
    Italic { text: String },
    Link { text: String },
    Phone { text: String },
    Plain { text: String },
    Pre { text: String, language: String },
    Strikethrough { text: String },
    TextLink { text: String, href: String },
}

impl TextEntity {
    pub fn to_html(&self) -> String {
        let wrap = |text: &str, tag: &str| -> String {
            let text = text.replace('\n', "<br />");
            format!("<{tag}>{text}</{tag}>")
        };

        match self {
            TextEntity::Bold { text } => wrap(text, "b"),
            TextEntity::Code { text } => wrap(text, "code"),
            TextEntity::CustomEmoji { text, .. } => wrap(text, "span"),
            TextEntity::Hashtag { text } => wrap(text, "bold"),
            TextEntity::Italic { text } => wrap(text, "i"),
            TextEntity::Link { text } => wrap(text, "span"),
            TextEntity::Phone { text } => wrap(text, "span"),
            TextEntity::Plain { text } => wrap(text, "span"),
            TextEntity::Pre { text, language: _ } => wrap(text, "pre"),
            TextEntity::Strikethrough { text } => wrap(text, "s"),
            TextEntity::TextLink { text, href: _ } => wrap(text, "span"),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Export {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub messages: Vec<Message>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: i64,
    #[serde(rename = "type")]
    pub type_field: String,

    pub contact_information: Option<serde_json::Value>,
    pub date: NaiveDateTime,
    pub date_unixtime: String,
    pub from: Option<String>,
    pub from_id: Option<String>,
    pub forwarded_from: Option<String>,
    pub reply_to_message_id: Option<i64>,
    pub text_entities: Vec<TextEntity>,
    pub edited: Option<String>,
    pub edited_unixtime: Option<String>,
    pub file: Option<String>,
    pub thumbnail: Option<String>,
    pub mime_type: Option<String>,
    pub photo: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub media_type: Option<String>,
    pub duration_seconds: Option<i64>,
}

impl Message {
    pub fn is_text_empty(&self) -> bool {
        self.text_entities.is_empty()
    }

    pub fn is_photo(&self) -> bool {
        self.photo.is_some() || self.mime_type == Some("image/jpeg".into())
    }

    pub fn unwrap_photo(&self) -> &str {
        if let Some(photo) = self.photo.as_ref() {
            photo
        } else if self.mime_type == Some("image/jpeg".into()) {
            self.unwrap_file()
        } else {
            panic!("File should exist")
        }
    }

    pub fn unwrap_file(&self) -> &str {
        self.file.as_ref().unwrap()
    }

    pub fn is_pdf(&self) -> bool {
        self.mime_type == Some("application/pdf".into())
    }

    pub fn has_record(&self) -> bool {
        let descrition = self.get_record();

        descrition.is_ok()
    }

    pub fn get_record(&self) -> eyre::Result<Record> {
        let Some(TextEntity::Pre { text, .. }) = self.text_entities.first() else {
            return Err(eyre::eyre!("No entry"));
        };

        // let reg = Regex::new(r"(?m)^date: (\S*)$")?;
        // let text = reg.replace(text, r#"date: "$1""#);

        let entry = serde_yaml::from_str(text);

        match entry {
            Ok(expr) => Ok(expr),
            Err(err) => Err(err.into()),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub date: String,
    pub tags: Vec<String>,
    pub person: String,
    #[serde(default)]
    pub messages: Vec<Message>,
}

pub struct TocItem<'a> {
    pub pages: u8,
    pub record: &'a Record,
}

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
