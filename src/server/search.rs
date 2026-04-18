use crate::server::{
    Conversation, ConvoCard, Databases, Message, SearchResponse, models::msg_prefix,
};
use anyhow::Result;
use lmdb::{Cursor, Transaction};

#[derive(Clone)]
pub struct IndexEntry {
    pub convo: Conversation,
    pub message_snippets: Vec<(String, String)>, // (msg_uuid, text)
}

pub struct SearchIndex {
    pub entries: Vec<IndexEntry>,
}

impl SearchIndex {
    pub fn build(dbs: &Databases) -> Result<Self> {
        let convo_txn = dbs.convos.env.begin_ro_txn()?;
        let msg_txn = dbs.messages.env.begin_ro_txn()?;

        let mut convo_cursor = convo_txn.open_ro_cursor(*dbs.convos.db)?;
        let mut entries = Vec::new();

        for (_, value) in convo_cursor.iter() {
            let convo: Conversation = bitcode::decode(value)?;

            let prefix = msg_prefix(&convo.uuid);
            let mut msg_cursor = msg_txn.open_ro_cursor(*dbs.messages.db)?;
            let mut snippets: Vec<(String, String)> = Vec::new();

            for (key, val) in msg_cursor.iter_from(prefix.as_bytes()) {
                let key_str = std::str::from_utf8(key).unwrap_or("");
                if !key_str.starts_with(&prefix) {
                    break;
                }
                let msg: Message = bitcode::decode(val)?;
                snippets.push((msg.uuid, msg.text));
            }

            entries.push(IndexEntry {
                convo,
                message_snippets: snippets,
            });
        }

        tracing::info!("search index built: {} conversations", entries.len());
        Ok(Self { entries })
    }

    pub fn search(&self, query: &str, limit: usize) -> SearchResponse {
        if query.is_empty() {
            let mut convos: Vec<ConvoCard> = self
                .entries
                .iter()
                .map(|e| ConvoCard {
                    uuid: e.convo.uuid.clone(),
                    name: e.convo.name.clone(),
                    created_at: e.convo.created_at.clone(),
                    updated_at: e.convo.updated_at.clone(),
                    message_count: e.convo.message_count,
                    snippet: String::new(),
                    match_count: 0,
                })
                .collect();
            convos.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            let total = convos.len();
            return SearchResponse {
                convos: convos.into_iter().take(limit).collect(),
                total,
            };
        }

        let q = query.to_ascii_lowercase();
        let mut scored: Vec<(usize, u32, String)> = Vec::new();

        for (idx, entry) in self.entries.iter().enumerate() {
            let mut match_count: u32 = 0;
            let mut best_snippet = String::new();

            if entry.convo.name.to_ascii_lowercase().contains(&q) {
                match_count += 1;
            }

            for (_, text) in &entry.message_snippets {
                if text.to_ascii_lowercase().contains(&q) {
                    match_count += 1;
                    if best_snippet.is_empty() {
                        best_snippet = make_snippet(text, &q, 120);
                    }
                }
            }

            if match_count > 0 {
                scored.push((idx, match_count, best_snippet));
            }
        }

        scored.sort_by(|a, b| b.1.cmp(&a.1));

        let total = scored.len();
        let convos = scored
            .into_iter()
            .take(limit)
            .map(|(idx, match_count, snippet)| {
                let e = &self.entries[idx];
                ConvoCard {
                    uuid: e.convo.uuid.clone(),
                    name: e.convo.name.clone(),
                    created_at: e.convo.created_at.clone(),
                    updated_at: e.convo.updated_at.clone(),
                    message_count: e.convo.message_count,
                    snippet,
                    match_count: match_count as usize,
                }
            })
            .collect();

        SearchResponse { convos, total }
    }
}

fn floor_char(s: &str, mut i: usize) -> usize {
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn make_snippet(text: &str, needle: &str, width: usize) -> String {
    let lc = text.to_ascii_lowercase();
    let pos = lc.find(needle).unwrap_or(0);
    let start = floor_char(text, pos.saturating_sub(width / 4));
    let end = floor_char(text, (start + width).min(text.len()));
    let mut s = String::new();
    if start > 0 {
        s.push('\u{2026}');
    }
    s.push_str(&text[start..end]);
    if end < text.len() {
        s.push('\u{2026}');
    }
    s
}

// use crate::server::{
//     Conversation, ConvoCard, Databases, Message, SearchResponse, models::msg_prefix,
// };
// use anyhow::Result;
// use lmdb::{Cursor, Transaction};
// use nucleo::{Config, Matcher, Utf32Str};
// use std::panic;

// /// One entry in the in-memory index per conversation
// #[derive(Clone)]
// pub struct IndexEntry {
//     pub convo: Conversation,
//     /// all message texts concatenated, lowercased
//     pub full_text: String,
//     /// individual message texts (same order as stored)
//     pub message_snippets: Vec<(String, String)>, // (msg_uuid, text)
// }

// pub static C_SCORE: u16 = 98;

// pub struct SearchIndex {
//     pub entries: Vec<IndexEntry>,
// }

// impl SearchIndex {
//     pub fn build(dbs: &Databases) -> Result<Self> {
//         let convo_txn = dbs.convos.env.begin_ro_txn()?;
//         let msg_txn = dbs.messages.env.begin_ro_txn()?;

//         let mut convo_cursor = convo_txn.open_ro_cursor(*dbs.convos.db)?;
//         let mut entries = Vec::new();

//         for (_, value) in convo_cursor.iter() {
//             let convo: Conversation = bitcode::decode(value)?;

//             let prefix = msg_prefix(&convo.uuid);
//             let mut msg_cursor = msg_txn.open_ro_cursor(*dbs.messages.db)?;
//             let mut snippets: Vec<(String, String)> = Vec::new();

//             for (key, val) in msg_cursor.iter_from(prefix.as_bytes()) {
//                 let key_str = std::str::from_utf8(key).unwrap_or("");
//                 if !key_str.starts_with(&prefix) {
//                     break;
//                 }
//                 let msg: Message = bitcode::decode(val)?;
//                 snippets.push((msg.uuid, msg.text));
//             }

//             let full_text = format!(
//                 "{} {}",
//                 convo.name.to_ascii_lowercase(),
//                 snippets
//                     .iter()
//                     .map(|(_, t)| t.as_str())
//                     .collect::<Vec<_>>()
//                     .join(" ")
//                     .to_ascii_lowercase()
//             );

//             entries.push(IndexEntry {
//                 convo,
//                 full_text,
//                 message_snippets: snippets,
//             });
//         }

//         tracing::info!("search index built: {} conversations", entries.len());
//         Ok(Self { entries })
//     }

//     pub fn search(&self, query: &str, limit: usize) -> SearchResponse {
//         if query.is_empty() {
//             // No query: return all sorted by updated_at desc
//             let mut convos: Vec<ConvoCard> = self
//                 .entries
//                 .iter()
//                 .map(|e| ConvoCard {
//                     uuid: e.convo.uuid.clone(),
//                     name: e.convo.name.clone(),
//                     created_at: e.convo.created_at.clone(),
//                     updated_at: e.convo.updated_at.clone(),
//                     message_count: e.convo.message_count,
//                     snippet: String::new(),
//                     match_count: 0,
//                 })
//                 .collect();
//             convos.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
//             let total = convos.len();
//             return SearchResponse {
//                 convos: convos.into_iter().take(limit).collect(),
//                 total,
//             };
//         }

//         let q = query.to_ascii_lowercase();

//         let result = panic::catch_unwind(|| {
//             let mut matcher = Matcher::new(Config::DEFAULT);
//             let mut needle_buf = Vec::new();
//             let needle = Utf32Str::new(&q, &mut needle_buf);

//             let mut scored: Vec<(usize, u32, String)> = Vec::new(); // (idx, total_score, snippet)

//             for (idx, entry) in self.entries.iter().enumerate() {
//                 let mut haystack_buf = Vec::new();
//                 let haystack = Utf32Str::new(&entry.full_text, &mut haystack_buf);

//                 if entry.full_text.len() < q.len() {
//                     continue;
//                 }

//                 if let Some(_) = matcher.fuzzy_match(haystack, needle) {
//                     // Count individual message matches for match_count
//                     let mut match_count: u32 = 0;
//                     let mut best_snippet = String::new();

//                     // Check title match
//                     let mut name_buf = Vec::new();
//                     let name_lc = entry.convo.name.to_ascii_lowercase();
//                     if name_lc.len() >= q.len() {
//                         let name_hay = Utf32Str::new(&name_lc, &mut name_buf);
//                         if matcher.fuzzy_match(name_hay, needle).is_some() {
//                             match_count += 1;
//                         }
//                     }

//                     // Check each message
//                     for (_, text) in &entry.message_snippets {
//                         if text.len() < q.len() {
//                             continue;
//                         }
//                         let lc = text.to_ascii_lowercase();
//                         let mut hbuf = Vec::new();
//                         let hay = Utf32Str::new(&lc, &mut hbuf);
//                         if matcher
//                             .fuzzy_match(hay, needle)
//                             .filter(|s| *s >= C_SCORE)
//                             .is_some()
//                         {
//                             match_count += 1;
//                             if best_snippet.is_empty() {
//                                 best_snippet = make_snippet(text, &q, 120);
//                             }
//                         }
//                     }

//                     if match_count > 0 {
//                         scored.push((idx, match_count, best_snippet));
//                     }
//                 }
//             }

//             scored
//         });

//         let mut scored = match result {
//             Ok(s) => s,
//             Err(_) => {
//                 tracing::error!("nucleo panicked for query: {:?}", query);
//                 return SearchResponse {
//                     convos: vec![],
//                     total: 0,
//                 };
//             }
//         };

//         // Sort by match count desc, then updated_at desc
//         scored.sort_by(|a, b| b.1.cmp(&a.1));

//         let total = scored.len();
//         let convos = scored
//             .into_iter()
//             .take(limit)
//             .map(|(idx, match_count, snippet)| {
//                 let e = &self.entries[idx];
//                 ConvoCard {
//                     uuid: e.convo.uuid.clone(),
//                     name: e.convo.name.clone(),
//                     created_at: e.convo.created_at.clone(),
//                     updated_at: e.convo.updated_at.clone(),
//                     message_count: e.convo.message_count,
//                     snippet,
//                     match_count: match_count as usize,
//                 }
//             })
//             .collect();

//         SearchResponse { convos, total }
//     }
// }

// fn floor_char(s: &str, mut i: usize) -> usize {
//     while i > 0 && !s.is_char_boundary(i) {
//         i -= 1;
//     }
//     i
// }

// /// Extract ~`width` chars of context around first occurrence of needle in text
// fn make_snippet(text: &str, needle: &str, width: usize) -> String {
//     let lc = text.to_ascii_lowercase();
//     let pos = lc.find(needle).unwrap_or(0);
//     let start = floor_char(text, pos.saturating_sub(width / 4));
//     let end = floor_char(text, (start + width).min(text.len()));
//     let mut s = String::new();
//     if start > 0 {
//         s.push_str("\u{2026}");
//     }
//     s.push_str(&text[start..end]);
//     if end < text.len() {
//         s.push_str("\u{2026}");
//     }
//     s
// }
