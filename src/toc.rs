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
                    <td>{}</td>
                    <td>{}</td>
                    <td style="width: 100%"><ul><li>{}</li></ul></td>
                    <td style="text-align: right"> {}</td>
                </tr>
                "#,
                    index + 1,
                    item.record.date,
                    item.record.tags.join("</li><li>"),
                    current_page - item.pages + 1,
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
