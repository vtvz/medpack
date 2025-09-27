use std::collections::HashMap;

use itertools::Itertools;

use crate::structs::{Export, Message, Record};

pub struct Categorizer {}

impl Categorizer {
    pub fn process_exports(exports: Vec<Export>) -> HashMap<String, Vec<Record>> {
        let messages = Self::messages(exports);

        // I do this for consistency as messages in different topics can interfere with each other
        let grouped_by_topic = Self::group_by_topic(messages);

        Self::person_records(grouped_by_topic)
    }

    pub fn messages(exports: Vec<Export>) -> Vec<Message> {
        exports
            .into_iter()
            // Merge exports to one stream
            .flat_map(|export| export.messages)
            // Remove trash
            .filter(|msg| msg.type_field == "message" && msg.contact_information.is_none())
            // Sort by date and edited to dedupe in right order
            .sorted_by_key(|msg| (msg.id, msg.date, msg.edited.unwrap_or_default()))
            .rev()
            .dedup_by(|a, b| a.id == b.id)
            .collect_vec()
    }

    pub fn group_by_topic(messages: Vec<Message>) -> HashMap<Option<i64>, Vec<Message>> {
        messages
            .into_iter()
            .map(|msg| (msg.reply_to_message_id, msg))
            .into_group_map()
    }

    pub fn person_records(
        grouped_by_topic: HashMap<Option<i64>, Vec<Message>>,
    ) -> HashMap<String, Vec<Record>> {
        grouped_by_topic
            .into_values()
            .flat_map(Self::group_messages)
            .sorted_by_key(|rec| rec.date.clone())
            .rev()
            .map(|rec| (rec.person.clone(), rec))
            .into_group_map()
    }

    fn group_messages(mut msgs: Vec<Message>) -> Vec<Record> {
        msgs.sort_by_key(|msg| msg.id);

        // Group is a collection of related messages
        let mut group: Vec<Message> = vec![];
        let mut records = vec![];
        let mut continue_group = false;

        for msg in msgs {
            // Will this messages be pushed to group
            let to_push;

            // If true will create record with grouped messages
            // `group` variable will be emptied
            let close_prev;

            // Record in msg is an `yaml` block with metadata
            if msg.has_record() {
                to_push = true;
                close_prev = true;

                // Image with record could have following image
                continue_group = msg.is_photo();
            } else if msg.is_photo() && msg.is_text_empty() && continue_group {
                // Image without record can be a part of group
                // True if group continues
                to_push = true;
                close_prev = false;
            } else {
                // Only images can create group.
                // Text and PDFs without Record won't be added to document
                to_push = false;
                close_prev = true;

                continue_group = false;
            }

            if close_prev && !group.is_empty() {
                records.push(Self::group_to_record(group));
                group = vec![];
            }

            if to_push {
                group.push(msg);
            }
        }

        if !group.is_empty() {
            records.push(Self::group_to_record(group));
        }

        records
    }

    fn group_to_record(group: Vec<Message>) -> Record {
        let mut record = group
            .first()
            .expect("Group shouldn't empty")
            .get_record()
            .expect("Group must be pre-parsed");

        record.messages = group;

        record
    }
}
