use crate::server::{
    Databases, convo_key,
    models::{Conversation, Message, RawConversation, RawExport},
    msg_key,
};
use anyhow::{Context, Result};
use lmdb::{Cursor, Transaction, WriteFlags};
use std::fs;

pub fn import_if_empty(dbs: &Databases) -> Result<()> {
    // Check if convos db already has data
    let txn = dbs.convos.env.begin_ro_txn()?;
    let mut cursor = txn.open_ro_cursor(*dbs.convos.db)?;
    let has_data = cursor.iter().next().is_some();
    drop(cursor);
    txn.abort();

    if has_data {
        tracing::info!("lmdb already populated, skipping import");
        return Ok(());
    }

    let path = "claude-data/conversations.json";
    if !std::path::Path::new(path).exists() {
        tracing::warn!("no conversations.json found, starting empty");
        return Ok(());
    }

    tracing::info!("importing conversations.json ...");
    let raw = fs::read_to_string(path).context("reading conversations.json")?;

    // The export can be either {"conversations": [...]} or just [...]
    let convos: Vec<RawConversation> = if raw.trim_start().starts_with('[') {
        serde_json::from_str(&raw).context("parsing json array")?
    } else {
        let export: RawExport = serde_json::from_str(&raw).context("parsing json object")?;
        export.conversations.unwrap_or_default()
    };

    let total = convos.len();
    tracing::info!("found {} conversations", total);

    let mut convo_txn = dbs.convos.env.begin_rw_txn()?;
    let mut msg_txn = dbs.messages.env.begin_rw_txn()?;

    for (i, raw_convo) in convos.into_iter().enumerate() {
        let msg_count = raw_convo.chat_messages.len();

        let convo = Conversation {
            uuid: raw_convo.uuid.clone(),
            name: raw_convo.name.clone(),
            summary: raw_convo.summary.clone(),
            created_at: raw_convo.created_at.clone(),
            updated_at: raw_convo.updated_at.clone(),
            message_count: msg_count,
        };

        convo_txn.put(
            *dbs.convos.db,
            &convo_key(&convo.uuid),
            &bitcode::encode(&convo),
            WriteFlags::empty(),
        )?;

        for raw_msg in raw_convo.chat_messages {
            // Prefer top-level text, fall back to content[0].text
            let text = if !raw_msg.text.is_empty() {
                raw_msg.text.clone()
            } else {
                raw_msg
                    .content
                    .iter()
                    .find(|c| c.kind == "text")
                    .map(|c| c.text.clone())
                    .unwrap_or_default()
            };

            let msg = Message {
                uuid: raw_msg.uuid.clone(),
                convo_uuid: raw_convo.uuid.clone(),
                text,
                sender: raw_msg.sender,
                created_at: raw_msg.created_at,
                updated_at: raw_msg.updated_at,
                parent_message_uuid: raw_msg.parent_message_uuid,
            };

            msg_txn.put(
                *dbs.messages.db,
                &msg_key(&raw_convo.uuid, &raw_msg.uuid),
                &bitcode::encode(&msg),
                WriteFlags::empty(),
            )?;
        }

        if (i + 1) % 500 == 0 {
            tracing::info!("imported {}/{}", i + 1, total);
        }
    }

    convo_txn.commit()?;
    msg_txn.commit()?;

    tracing::info!("import complete: {} conversations", total);
    Ok(())
}
