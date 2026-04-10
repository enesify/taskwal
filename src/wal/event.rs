use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum WalEvent {
    Add {
        id: String,
        title: String,
        tags: Vec<String>,
        day: String,
    },
    Move {
        id: String,
        to: Column,
    },
    Edit {
        id: String,
        title: String,
    },
    Delete {
        id: String,
    },
    Note {
        id: String,
        text: String,
    },
    Tag {
        id: String,
        action: TagAction,
        tag: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalEntry {
    pub ts: DateTime<Utc>,
    #[serde(flatten)]
    pub event: WalEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Column {
    Todo,
    Doing,
    Done,
}

impl Column {
    /// One step backward on the Todo → Doing → Done pipeline.
    pub fn back_from(self) -> Option<Self> {
        match self {
            Column::Done => Some(Column::Doing),
            Column::Doing => Some(Column::Todo),
            Column::Todo => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagAction {
    Add,
    Remove,
}
