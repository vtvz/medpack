use std::path::PathBuf;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

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
    pub edited: Option<NaiveDateTime>,
    pub edited_unixtime: Option<String>,
    pub file: Option<String>,
    pub thumbnail: Option<String>,
    pub mime_type: Option<String>,
    pub photo: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub media_type: Option<String>,
    pub duration_seconds: Option<i64>,

    pub export_path: Option<PathBuf>,
}

impl Message {
    pub fn is_text_empty(&self) -> bool {
        self.text_entities.is_empty()
    }

    pub fn is_photo(&self) -> bool {
        self.photo.is_some() || self.mime_type == Some("image/jpeg".into())
    }

    pub fn unwrap_export_path(&self) -> PathBuf {
        self.export_path.clone().unwrap()
    }

    pub fn unwrap_photo(&self) -> PathBuf {
        if let Some(photo) = self.photo.as_ref() {
            let mut export_path = self.unwrap_export_path();
            export_path.push(photo);

            export_path
        } else if self.mime_type == Some("image/jpeg".into()) {
            self.unwrap_file()
        } else {
            panic!("File should exist")
        }
    }

    pub fn unwrap_file(&self) -> PathBuf {
        let mut export_path = self.unwrap_export_path();

        export_path.push(self.file.as_ref().unwrap());

        export_path
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
