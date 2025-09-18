use itertools::Itertools;

use crate::structs::Record;

pub struct TocItem<'a> {
    pub pages: u8,
    pub record: &'a Record,
}

pub struct Toc<'a> {
    pub items: Vec<TocItem<'a>>,
}

impl<'a> Toc<'a> {
    pub fn new() -> Self {
        Self::new_from(Vec::new())
    }

    pub fn new_from(toc_items: impl IntoIterator<Item = TocItem<'a>>) -> Self {
        Self {
            items: toc_items.into_iter().collect(),
        }
    }

    pub fn append(&mut self, toc_items: impl IntoIterator<Item = TocItem<'a>>) {
        self.items.extend(toc_items);
    }

    pub fn generate_html(&self, shift: u8) -> String {
        let mut current_page = shift;
        let content = self
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                current_page += item.pages;
                format!(
                    r#"
                        <tr>
                            <td>{index}</td>
                            <td>{date}</td>
                            <td style="width: 100%">
                                {place}
                                <ul><li>{tags}</li></ul>
                                {doctor}
                            </td>
                            <td style="text-align: right">{page}</td>
                        </tr>
                    "#,
                    index = index + 1,
                    date = item.record.date,
                    tags = item.record.tags.join("</li><li>"),
                    place = item
                        .record
                        .place
                        .as_ref()
                        .map(|place| format!("<div class='small-font'>{place}</div>"))
                        .unwrap_or_default(),
                    doctor = item
                        .record
                        .doctor
                        .as_ref()
                        .map(|doctor| format!("<div class='small-font'>{doctor}</div>"))
                        .unwrap_or_default(),
                    page = current_page - item.pages + 1,
                )
            })
            .join("");

        format!(
            r#"
            <table class="table table-striped table-sm">
                <tr class="thead-dark">
                    <th style="text-align: left">#</th>
                    <th style="text-align: left">date</th>
                    <th style="width: 100%; text-align: left">info</th>
                    <th style="text-align: right">#</th>
                </tr>
                {content}
            </table>
            "#
        )
    }
}
