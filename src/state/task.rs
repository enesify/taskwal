use chrono::{DateTime, Utc};
use crate::wal::event::Column;

#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub column: Column,
    pub notes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub done_at: Option<DateTime<Utc>>,
    pub created_day: String,
}

#[derive(Debug, Default, Clone)]
pub struct Board {
    pub todo: Vec<Task>,
    pub doing: Vec<Task>,
    pub done: Vec<Task>,
}

impl Board {
    pub fn all_task_ids(&self) -> impl Iterator<Item = &str> {
        self.todo
            .iter()
            .chain(self.doing.iter())
            .chain(self.done.iter())
            .map(|t| t.id.as_str())
    }

    pub fn find_task(&self, id: &str) -> Option<&Task> {
        self.todo
            .iter()
            .find(|t| t.id == id)
            .or_else(|| self.doing.iter().find(|t| t.id == id))
            .or_else(|| self.done.iter().find(|t| t.id == id))
    }
}
