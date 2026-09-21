use itertools::Itertools;
use serde::{Deserialize, Serialize};

use super::TextEntity;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RichMessage {
    pub rtl: bool,
    pub part: bool,
    pub blocks: Vec<RichBlock>,
}

impl RichMessage {
    pub fn body_to_html(&self) -> String {
        let body = self.blocks.iter().skip(1).map(RichBlock::to_html).join("");

        if self.rtl {
            format!(r#"<div dir="rtl">{body}</div>"#)
        } else {
            body
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RichBlock {
    Code {
        text: RichText,
        language: String,
    },
    Table {
        title: Option<RichText>,
        bordered: bool,
        striped: bool,
        compact: bool,
        rows: Vec<RichTableRow>,
    },
    Heading {
        level: u8,
        text: RichText,
    },
    Paragraph {
        text: RichText,
    },
    Quote {
        content: String,
        pullquote: bool,
        text: Option<RichText>,
        caption: Option<RichText>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        blocks: Vec<RichBlock>,
    },
    Footer {
        text: RichText,
    },
    Divider,
    List {
        kind: RichListKind,
        // present only on ordered lists
        #[serde(skip_serializing_if = "Option::is_none")]
        reversed: Option<bool>,
        items: Vec<RichListItem>,
    },
    Details {
        title: RichText,
        open: bool,
        blocks: Vec<RichBlock>,
    },
    ButtonRow {
        alignment: String,
        buttons: Vec<RichButton>,
    },
}

impl RichBlock {
    pub fn to_html(&self) -> String {
        match self {
            RichBlock::Code { text, language } => {
                let text = text.plain_text();
                match language.as_str() {
                    "html" => text,
                    "csv" => TextEntity::csv_to_html(&text),
                    "hidden" => String::new(),
                    _ => format!("<pre>{text}</pre>"),
                }
            },
            RichBlock::Table {
                title,
                bordered,
                striped,
                compact,
                rows,
            } => {
                let mut classes = vec!["table"];
                if *bordered {
                    classes.push("table-bordered");
                }
                if *striped {
                    classes.push("table-striped");
                }
                if *compact {
                    classes.push("table-sm");
                }

                let caption = title
                    .as_ref()
                    .map(|title| title.to_html())
                    .filter(|title| !title.is_empty())
                    .map(|title| {
                        format!("<caption style='caption-side: top'><b>{title}</b></caption>")
                    })
                    .unwrap_or_default();

                let rows = rows
                    .iter()
                    .map(|row| {
                        let cells = row
                            .cells
                            .iter()
                            .map(|cell| {
                                let tag = if cell.header { "th" } else { "td" };
                                format!(
                                    "<{tag} style='text-align: {align}; vertical-align: {valign}'>{text}</{tag}>",
                                    align = cell.align,
                                    valign = cell.vertical_align,
                                    text = cell.text.to_html(),
                                )
                            })
                            .join("");

                        format!("<tr>{cells}</tr>")
                    })
                    .join("");

                format!(
                    "<table class='{classes}' style='width: inherit; font-size: 13px'>{caption}{rows}</table>",
                    classes = classes.join(" ")
                )
            },
            RichBlock::Heading { level, text } => {
                format!("<h{level}>{}</h{level}>", text.to_html())
            },
            RichBlock::Paragraph { text } => {
                let text = text.to_html();
                if text.is_empty() {
                    "<br />".into()
                } else {
                    format!("<p>{text}</p>")
                }
            },
            RichBlock::Quote {
                text,
                caption,
                blocks,
                ..
            } => {
                let body = match text {
                    Some(text) => text.to_html(),
                    None => blocks.iter().map(RichBlock::to_html).join(""),
                };

                let caption = caption
                    .as_ref()
                    .map(|caption| caption.to_html())
                    .filter(|caption| !caption.is_empty())
                    .map(|caption| format!("<footer class='blockquote-footer'>{caption}</footer>"))
                    .unwrap_or_default();

                format!("<blockquote>{body}{caption}</blockquote>")
            },
            RichBlock::Footer { text } => {
                format!(
                    "<footer class='small-font text-muted'>{}</footer>",
                    text.to_html()
                )
            },
            RichBlock::Divider => "<hr />".into(),
            RichBlock::List {
                kind,
                reversed,
                items,
            } => {
                let tag = match kind {
                    RichListKind::Ordered => "ol",
                    RichListKind::Bullet => "ul",
                };
                let reversed = if *reversed == Some(true) {
                    " reversed"
                } else {
                    ""
                };

                let items = items
                    .iter()
                    .map(|item| {
                        let marker = match item.task_state {
                            RichTaskState::None => "",
                            RichTaskState::Unchecked => "☐ ",
                            RichTaskState::Checked => "☑ ",
                        };
                        let value = item
                            .num
                            .as_ref()
                            .map(|num| format!(" value='{num}'"))
                            .unwrap_or_default();
                        let content = match &item.content {
                            RichListItemContent::Text { text } => text.to_html(),
                            RichListItemContent::Blocks { blocks } => {
                                blocks.iter().map(RichBlock::to_html).join("")
                            },
                        };

                        format!("<li{value}>{marker}{content}</li>")
                    })
                    .join("");

                format!("<{tag}{reversed}>{items}</{tag}>")
            },
            // wkhtmltopdf doesn't support <details>, render title + content
            RichBlock::Details { title, blocks, .. } => {
                let body = blocks.iter().map(RichBlock::to_html).join("");

                format!("<div><b>{}</b><div>{body}</div></div>", title.to_html())
            },
            RichBlock::ButtonRow { buttons, .. } => {
                let buttons = buttons
                    .iter()
                    .map(|button| {
                        let RichButtonAction::Url { url } = &button.button;

                        format!("<a href='{url}'>{}</a>", button.text.to_html())
                    })
                    .join(" ");

                format!("<p>{buttons}</p>")
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RichListKind {
    Ordered,
    Bullet,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RichTaskState {
    None,
    Unchecked,
    Checked,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RichListItem {
    pub task_state: RichTaskState,
    #[serde(flatten)]
    pub content: RichListItemContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num: Option<String>,
}

// "content" field discriminates the payload: "text" carries a single
// text node, "blocks" carries nested blocks (incl. nested lists)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "content", rename_all = "snake_case")]
pub enum RichListItemContent {
    Text { text: RichText },
    Blocks { blocks: Vec<RichBlock> },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RichTableRow {
    pub cells: Vec<RichTableCell>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RichTableCell {
    pub text: RichText,
    pub header: bool,
    pub align: String,
    pub vertical_align: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RichButton {
    pub text: RichText,
    pub button: RichButtonAction,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RichButtonAction {
    Url { url: String },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RichText {
    Plain { text: String },
    Empty,
    Concat { text: Vec<RichText> },
    Code { text: Box<RichText> },
    Bold { text: Box<RichText> },
    Italic { text: Box<RichText> },
    Underline { text: Box<RichText> },
    Strikethrough { text: Box<RichText> },
    Spoiler { text: Box<RichText> },
    Subscript { text: Box<RichText> },
    Superscript { text: Box<RichText> },
    Marked { text: Box<RichText> },
    TextLink { href: String, text: Box<RichText> },
}

impl RichText {
    pub fn to_html(&self) -> String {
        let wrap =
            |text: &RichText, tag: &str| -> String { format!("<{tag}>{}</{tag}>", text.to_html()) };

        match self {
            RichText::Plain { text } => text.replace('\n', "<br />"),
            RichText::Empty => String::new(),
            RichText::Concat { text } => text.iter().map(RichText::to_html).join(""),
            RichText::Code { text } => wrap(text, "code"),
            RichText::Bold { text } => wrap(text, "b"),
            RichText::Italic { text } => wrap(text, "i"),
            RichText::Underline { text } => wrap(text, "u"),
            RichText::Strikethrough { text } => wrap(text, "s"),
            RichText::Spoiler { text } => wrap(text, "span"),
            RichText::Subscript { text } => wrap(text, "sub"),
            RichText::Superscript { text } => wrap(text, "sup"),
            RichText::Marked { text } => wrap(text, "mark"),
            RichText::TextLink { href, text } => {
                format!("<a href=\"{href}\">{}</a>", text.to_html())
            },
        }
    }

    pub fn plain_text(&self) -> String {
        match self {
            RichText::Plain { text } => text.clone(),
            RichText::Empty => String::new(),
            RichText::Concat { text } => text.iter().map(RichText::plain_text).join(""),
            RichText::Code { text }
            | RichText::Bold { text }
            | RichText::Italic { text }
            | RichText::Underline { text }
            | RichText::Strikethrough { text }
            | RichText::Spoiler { text }
            | RichText::Subscript { text }
            | RichText::Superscript { text }
            | RichText::Marked { text }
            | RichText::TextLink { text, .. } => text.plain_text(),
        }
    }
}

