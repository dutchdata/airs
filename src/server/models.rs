use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

// ── Raw JSON shapes (for import) ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RawExport {
    pub conversations: Option<Vec<RawConversation>>,
    // top-level may be array directly
}

#[derive(Debug, Deserialize)]
pub struct RawConversation {
    pub uuid: String,
    pub name: String,
    #[serde(default)]
    pub summary: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub chat_messages: Vec<RawMessage>,
}

#[derive(Debug, Deserialize)]
pub struct RawMessage {
    pub uuid: String,
    #[serde(default)]
    pub text: String,
    pub sender: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub parent_message_uuid: Option<String>,
    #[serde(default)]
    pub content: Vec<RawContent>,
}

#[derive(Debug, Deserialize)]
pub struct RawContent {
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub text: String,
}

// ── Stored types (bitcode) ────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
pub struct Conversation {
    pub uuid: String,
    pub name: String,
    pub summary: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
pub struct Message {
    pub uuid: String,
    pub convo_uuid: String,
    pub text: String,
    pub sender: String, // "human" | "assistant"
    pub created_at: String,
    pub updated_at: String,
    pub parent_message_uuid: Option<String>,
}

// ── API response shapes ───────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConvoCard {
    pub uuid: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: usize,
    pub snippet: String,    // excerpt around match
    pub match_count: usize, // number of fuzzy hits in this convo
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub convos: Vec<ConvoCard>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreadResponse {
    pub convo: Conversation,
    pub messages: Vec<Message>,
}

// ── LMDB key helpers ──────────────────────────────────────────────────────────

pub fn convo_key(uuid: &str) -> String {
    format!("convo::{uuid}")
}

pub fn msg_key(convo_uuid: &str, msg_uuid: &str) -> String {
    format!("msg::{convo_uuid}::{msg_uuid}")
}

pub fn msg_prefix(convo_uuid: &str) -> String {
    format!("msg::{convo_uuid}::")
}
