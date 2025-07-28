use async_graphql::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Note schema for GraphQL
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject, PartialEq)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub author: String,
    #[graphql(name = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[graphql(name = "updatedAt")]
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

impl Note {
    /// Create a new note with generated ID
    pub fn new(title: String, content: String, author: String, tags: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            id: format!("notes:{}", uuid::Uuid::new_v4()),
            title,
            content,
            author,
            created_at: now,
            updated_at: now,
            tags,
        }
    }
    
    /// Create note with specific ID (for tests)
    pub fn with_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }
    
    /// Update note content and timestamp
    pub fn update(&mut self, title: Option<String>, content: Option<String>, tags: Option<Vec<String>>) {
        if let Some(title) = title {
            self.title = title;
        }
        if let Some(content) = content {
            self.content = content;
        }
        if let Some(tags) = tags {
            self.tags = tags;
        }
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_creation() {
        let note = Note::new(
            "Test Note".to_string(),
            "Test content".to_string(),
            "test_user".to_string(),
            vec!["test".to_string()],
        );
        
        assert_eq!(note.title, "Test Note");
        assert_eq!(note.content, "Test content");
        assert_eq!(note.author, "test_user");
        assert_eq!(note.tags, vec!["test".to_string()]);
        assert!(note.id.starts_with("notes:"));
    }
    
    #[test]
    fn test_note_update() {
        let mut note = Note::new(
            "Original".to_string(),
            "Original content".to_string(),
            "test_user".to_string(),
            vec![],
        );
        
        let original_created = note.created_at;
        let original_updated = note.updated_at;
        
        // Small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        note.update(
            Some("Updated".to_string()),
            Some("Updated content".to_string()),
            Some(vec!["updated".to_string()]),
        );
        
        assert_eq!(note.title, "Updated");
        assert_eq!(note.content, "Updated content");
        assert_eq!(note.tags, vec!["updated".to_string()]);
        assert_eq!(note.created_at, original_created);
        assert!(note.updated_at > original_updated);
    }
}