use serde::{Deserialize, Serialize};
use garde::Validate;
use chrono::{DateTime, Utc};
use surrealdb::sql::{Thing, Id};
use std::fmt;

/// Custom validation error for model validation
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Validation failed: {0}")]
    InvalidInput(String),
    #[error("Invalid ID format: {0}")]
    InvalidId(String),
}

/// Note identifier type that wraps SurrealDB Thing
#[derive(Debug, Clone, PartialEq)]
pub struct NoteId(Thing);

impl NoteId {
    /// Create a new Note ID with a ULID
    pub fn new() -> Self {
        Self(Thing::from(("notes", Id::ulid())))
    }
    
    /// Create a Note ID from a string representation (e.g., "notes:abc123")
    pub fn from_string(s: &str) -> Result<Self, ValidationError> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(ValidationError::InvalidId("ID must be in format 'collection:id'".to_string()));
        }
        
        if parts[0] != "notes" {
            return Err(ValidationError::InvalidId("Collection must be 'notes'".to_string()));
        }
        
        // Parse the ID part - it could be various types
        let id = if parts[1].chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            Id::String(parts[1].to_string())
        } else {
            return Err(ValidationError::InvalidId("Invalid ID characters".to_string()));
        };
        
        Ok(Self(Thing::from((parts[0], id))))
    }
    
    /// Get the collection name (always "notes" for NoteId)
    pub fn collection(&self) -> &str {
        &self.0.tb
    }
    
    /// Get the ID part as a string
    pub fn id(&self) -> String {
        self.0.id.to_string()
    }
    
    /// Get the underlying Thing for database operations
    pub fn as_thing(&self) -> &Thing {
        &self.0
    }
}

impl Default for NoteId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for NoteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.0.tb, self.0.id)
    }
}

impl From<Thing> for NoteId {
    fn from(thing: Thing) -> Self {
        Self(thing)
    }
}

impl From<NoteId> for Thing {
    fn from(note_id: NoteId) -> Self {
        note_id.0
    }
}

impl TryFrom<String> for NoteId {
    type Error = ValidationError;
    
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_string(&s)
    }
}

impl TryFrom<&str> for NoteId {
    type Error = ValidationError;
    
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_string(s)
    }
}

// Serde implementations for API compatibility
impl Serialize for NoteId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for NoteId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_string(&s).map_err(serde::de::Error::custom)
    }
}

/// Custom validation function to prevent script tags
fn no_script_tags(value: &str, _: &()) -> garde::Result {
    if value.to_lowercase().contains("<script") || value.to_lowercase().contains("</script>") {
        return Err(garde::Error::new("Script tags are not allowed"));
    }
    Ok(())
}

/// Custom validation function to check for valid author format
fn valid_author(value: &str, _: &()) -> garde::Result {
    if !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(garde::Error::new("Author must contain only alphanumeric characters, underscores, and hyphens"));
    }
    Ok(())
}

/// Note model with comprehensive validation
#[derive(Debug, Clone, Serialize, Deserialize, Validate, PartialEq)]
pub struct Note {
    /// Optional ID - None for new notes, Some for existing notes
    #[serde(skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub id: Option<NoteId>,
    
    /// Note title with length and content validation
    #[garde(length(min = 1, max = 200))]
    #[garde(custom(no_script_tags))]
    pub title: String,
    
    /// Note content with length validation
    #[garde(length(min = 1, max = 10000))]
    pub content: String,
    
    /// Author identifier with format validation
    #[garde(length(min = 1, max = 100))]
    #[garde(custom(valid_author))]
    pub author: String,
    
    /// Creation timestamp (skipped from validation)
    #[garde(skip)]
    pub created_at: DateTime<Utc>,
    
    /// Last update timestamp (skipped from validation)
    #[garde(skip)]
    pub updated_at: DateTime<Utc>,
    
    /// Optional tags with count and individual length limits
    #[garde(length(max = 10))]
    #[garde(inner(length(min = 1, max = 50)))]
    pub tags: Vec<String>,
}

/// Builder pattern for creating Notes with validation
pub struct NoteBuilder {
    title: Option<String>,
    content: Option<String>,
    author: Option<String>,
    tags: Vec<String>,
}

impl NoteBuilder {
    /// Create a new NoteBuilder
    /// 
    /// # Example
    /// ```
    /// use pcf_api::services::database::models::NoteBuilder;
    /// 
    /// let note = NoteBuilder::new()
    ///     .title("My Note")
    ///     .content("Some content")
    ///     .author("alice")
    ///     .tag("important")
    ///     .build()
    ///     .expect("Valid note");
    /// 
    /// assert_eq!(note.title, "My Note");
    /// assert_eq!(note.author, "alice");
    /// ```
    pub fn new() -> Self {
        Self {
            title: None,
            content: None,
            author: None,
            tags: Vec::new(),
        }
    }
    
    /// Set the note title
    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }
    
    /// Set the note content
    pub fn content<S: Into<String>>(mut self, content: S) -> Self {
        self.content = Some(content.into());
        self
    }
    
    /// Set the note author
    pub fn author<S: Into<String>>(mut self, author: S) -> Self {
        self.author = Some(author.into());
        self
    }
    
    /// Add a tag to the note
    pub fn tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.tags.push(tag.into());
        self
    }
    
    /// Add multiple tags to the note
    pub fn tags<I, S>(mut self, tags: I) -> Self 
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags.extend(tags.into_iter().map(|s| s.into()));
        self
    }
    
    /// Build the Note, validating all fields
    pub fn build(self) -> Result<Note, ValidationError> {
        let title = self.title.ok_or_else(|| {
            ValidationError::InvalidInput("Title is required".to_string())
        })?;
        
        let content = self.content.ok_or_else(|| {
            ValidationError::InvalidInput("Content is required".to_string())
        })?;
        
        let author = self.author.ok_or_else(|| {
            ValidationError::InvalidInput("Author is required".to_string())
        })?;
        
        let note = Note::new(title, content, author, self.tags);
        
        // Validate the built note
        note.validate_model().map_err(|report| {
            ValidationError::InvalidInput(format!("Validation failed: {}", report))
        })?;
        
        Ok(note)
    }
}

impl Default for NoteBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Note {
    /// Create a new Note with current timestamps
    /// 
    /// # Example
    /// ```
    /// use pcf_api::services::database::models::Note;
    /// 
    /// let note = Note::new(
    ///     "Shopping List".to_string(),
    ///     "Milk, Eggs, Bread".to_string(),
    ///     "alice".to_string(),
    ///     vec!["personal".to_string()],
    /// );
    /// 
    /// assert_eq!(note.title, "Shopping List");
    /// assert_eq!(note.author, "alice");
    /// assert!(note.id.is_none()); // New notes don't have IDs yet
    /// ```
    pub fn new(title: String, content: String, author: String, tags: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            title,
            content,
            author,
            created_at: now,
            updated_at: now,
            tags,
        }
    }
    
    /// Create a Note with a specific ID (for database retrieval)
    pub fn with_id(
        id: NoteId,
        title: String,
        content: String,
        author: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        tags: Vec<String>,
    ) -> Self {
        Self {
            id: Some(id),
            title,
            content,
            author,
            created_at,
            updated_at,
            tags,
        }
    }
    
    /// Update the note's content and refresh the updated_at timestamp
    /// 
    /// # Example
    /// ```
    /// use pcf_api::services::database::models::Note;
    /// 
    /// let mut note = Note::new(
    ///     "Title".to_string(),
    ///     "Original content".to_string(),
    ///     "alice".to_string(),
    ///     vec![],
    /// );
    /// 
    /// let old_updated = note.updated_at;
    /// note.update_content("New content".to_string());
    /// 
    /// assert_eq!(note.content, "New content");
    /// assert!(note.updated_at > old_updated);
    /// ```
    pub fn update_content(&mut self, new_content: String) {
        self.content = new_content;
        self.updated_at = Utc::now();
    }
    
    /// Update the note's title and refresh the updated_at timestamp
    pub fn update_title(&mut self, new_title: String) {
        self.title = new_title;
        self.updated_at = Utc::now();
    }
    
    /// Add a tag if it doesn't already exist and doesn't exceed limits
    pub fn add_tag(&mut self, tag: String) -> Result<(), ValidationError> {
        if self.tags.len() >= 10 {
            return Err(ValidationError::InvalidInput("Maximum 10 tags allowed".to_string()));
        }
        
        if tag.len() == 0 || tag.len() > 50 {
            return Err(ValidationError::InvalidInput("Tag length must be 1-50 characters".to_string()));
        }
        
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = Utc::now();
        }
        
        Ok(())
    }
    
    /// Remove a tag if it exists
    pub fn remove_tag(&mut self, tag: &str) -> bool {
        if let Some(pos) = self.tags.iter().position(|x| x == tag) {
            self.tags.remove(pos);
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }
    
    /// Validate the note using Garde
    pub fn validate_model(&self) -> Result<(), garde::Report> {
        self.validate()
    }
}

/// Schema conversion utilities for database operations
pub mod schema {
    use super::*;
    use serde_json::Value;
    
    /// Convert a Note to a JSON Value for database storage
    pub fn note_to_value(note: &Note) -> Result<Value, serde_json::Error> {
        serde_json::to_value(note)
    }
    
    /// Convert a JSON Value from database to a Note
    pub fn value_to_note(value: Value) -> Result<Note, serde_json::Error> {
        serde_json::from_value(value)
    }
    
    /// Extract fields from a Note for partial updates
    pub fn note_update_fields(note: &Note) -> serde_json::Map<String, Value> {
        let mut map = serde_json::Map::new();
        
        map.insert("title".to_string(), Value::String(note.title.clone()));
        map.insert("content".to_string(), Value::String(note.content.clone()));
        map.insert("author".to_string(), Value::String(note.author.clone()));
        map.insert("updated_at".to_string(), Value::String(note.updated_at.to_rfc3339()));
        map.insert("tags".to_string(), Value::Array(
            note.tags.iter().map(|tag| Value::String(tag.clone())).collect()
        ));
        
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_note_validation_valid() {
        let valid_note = Note::new(
            "Test Note".to_string(),
            "Test content".to_string(),
            "user123".to_string(),
            vec!["tag1".to_string(), "tag2".to_string()],
        );
        
        assert!(valid_note.validate_model().is_ok());
    }
    
    #[test]
    fn test_note_validation_empty_title() {
        let invalid_note = Note::new(
            "".to_string(), // Empty title
            "Test content".to_string(),
            "user123".to_string(),
            vec![],
        );
        
        let validation_result = invalid_note.validate_model();
        assert!(validation_result.is_err());
        
        let errors = validation_result.unwrap_err();
        assert!(errors.to_string().contains("title"));
    }
    
    #[test]
    fn test_note_validation_long_title() {
        let long_title = "a".repeat(201); // Too long
        let invalid_note = Note::new(
            long_title,
            "Test content".to_string(),
            "user123".to_string(),
            vec![],
        );
        
        let validation_result = invalid_note.validate_model();
        assert!(validation_result.is_err());
    }
    
    #[test]
    fn test_note_validation_script_tags() {
        let malicious_title = "Test <script>alert('xss')</script>";
        let invalid_note = Note::new(
            malicious_title.to_string(),
            "Test content".to_string(),
            "user123".to_string(),
            vec![],
        );
        
        let validation_result = invalid_note.validate_model();
        assert!(validation_result.is_err());
        
        let errors = validation_result.unwrap_err();
        assert!(errors.to_string().contains("Script tags"));
    }
    
    #[test]
    fn test_note_validation_invalid_author() {
        let invalid_note = Note::new(
            "Test Note".to_string(),
            "Test content".to_string(),
            "user@domain.com".to_string(), // Contains invalid characters
            vec![],
        );
        
        let validation_result = invalid_note.validate_model();
        assert!(validation_result.is_err());
    }
    
    #[test]
    fn test_note_validation_too_many_tags() {
        let many_tags: Vec<String> = (0..11).map(|i| format!("tag{}", i)).collect();
        let invalid_note = Note::new(
            "Test Note".to_string(),
            "Test content".to_string(),
            "user123".to_string(),
            many_tags,
        );
        
        let validation_result = invalid_note.validate_model();
        assert!(validation_result.is_err());
    }
    
    #[test]
    fn test_note_id_creation() {
        let id = NoteId::new();
        assert_eq!(id.collection(), "notes");
        assert!(!id.id().is_empty());
        
        // Test string representation
        let id_string = id.to_string();
        assert!(id_string.starts_with("notes:"));
    }
    
    #[test]
    fn test_note_id_from_string_valid() {
        let id = NoteId::from_string("notes:abc123").unwrap();
        assert_eq!(id.collection(), "notes");
        assert_eq!(id.id(), "abc123");
        assert_eq!(id.to_string(), "notes:abc123");
    }
    
    #[test]
    fn test_note_id_from_string_invalid() {
        // Invalid format
        assert!(NoteId::from_string("invalid").is_err());
        
        // Wrong collection
        assert!(NoteId::from_string("users:123").is_err());
        
        // Invalid characters
        assert!(NoteId::from_string("notes:abc@123").is_err());
    }
    
    #[test]
    fn test_note_id_round_trip() {
        let original = NoteId::from_string("notes:test123").unwrap();
        let as_string = original.to_string();
        let parsed = NoteId::from_string(&as_string).unwrap();
        
        assert_eq!(original, parsed);
    }
    
    #[test]
    fn test_note_id_thing_conversion() {
        let thing = Thing::from(("notes", "abc123"));
        let id = NoteId::from(thing.clone());
        assert_eq!(id.to_string(), "notes:abc123");
        
        let back_to_thing: Thing = id.into();
        assert_eq!(back_to_thing.tb, "notes");
    }
    
    #[test]
    fn test_note_id_serde() {
        let id = NoteId::from_string("notes:test123").unwrap();
        
        // Test serialization
        let serialized = serde_json::to_string(&id).unwrap();
        assert_eq!(serialized, "\"notes:test123\"");
        
        // Test deserialization
        let deserialized: NoteId = serde_json::from_str(&serialized).unwrap();
        assert_eq!(id, deserialized);
    }
    
    #[test]
    fn test_note_operations() {
        let mut note = Note::new(
            "Original Title".to_string(),
            "Original content".to_string(),
            "user123".to_string(),
            vec!["initial".to_string()],
        );
        
        let original_updated = note.updated_at;
        
        // Update content
        note.update_content("New content".to_string());
        assert_eq!(note.content, "New content");
        assert!(note.updated_at > original_updated);
        
        // Update title
        let after_content_update = note.updated_at;
        note.update_title("New Title".to_string());
        assert_eq!(note.title, "New Title");
        assert!(note.updated_at > after_content_update);
        
        // Add tag
        assert!(note.add_tag("new_tag".to_string()).is_ok());
        assert!(note.tags.contains(&"new_tag".to_string()));
        
        // Remove tag
        assert!(note.remove_tag("initial"));
        assert!(!note.tags.contains(&"initial".to_string()));
        
        // Test tag limits - note already has "new_tag", so we can add 9 more
        for i in 0..9 {
            assert!(note.add_tag(format!("tag{}", i)).is_ok());
        }
        
        // Should fail on 11th tag (note now has 10 tags)
        assert!(note.add_tag("too_many".to_string()).is_err());
    }
    
    #[test]
    fn test_schema_conversions() {
        let note = Note::new(
            "Test Note".to_string(),
            "Test content".to_string(),
            "user123".to_string(),
            vec!["tag1".to_string()],
        );
        
        // Convert to value and back
        let value = schema::note_to_value(&note).unwrap();
        let converted_note = schema::value_to_note(value).unwrap();
        
        assert_eq!(note.title, converted_note.title);
        assert_eq!(note.content, converted_note.content);
        assert_eq!(note.author, converted_note.author);
        assert_eq!(note.tags, converted_note.tags);
        
        // Test update fields
        let update_fields = schema::note_update_fields(&note);
        assert!(update_fields.contains_key("title"));
        assert!(update_fields.contains_key("content"));
        assert!(update_fields.contains_key("updated_at"));
    }
    
    #[test]
    fn test_note_builder_success() {
        let note = NoteBuilder::new()
            .title("Test Note")
            .content("Test content")
            .author("user123")
            .tag("tag1")
            .tag("tag2")
            .build()
            .unwrap();
        
        assert_eq!(note.title, "Test Note");
        assert_eq!(note.content, "Test content");
        assert_eq!(note.author, "user123");
        assert_eq!(note.tags, vec!["tag1", "tag2"]);
        assert!(note.id.is_none());
    }
    
    #[test]
    fn test_note_builder_missing_fields() {
        // Missing title
        let result = NoteBuilder::new()
            .content("Content")
            .author("user123")
            .build();
        assert!(result.is_err());
        
        // Missing content
        let result = NoteBuilder::new()
            .title("Title")
            .author("user123")
            .build();
        assert!(result.is_err());
        
        // Missing author
        let result = NoteBuilder::new()
            .title("Title")
            .content("Content")
            .build();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_note_builder_validation_failure() {
        // Invalid title (script tag)
        let result = NoteBuilder::new()
            .title("Test <script>alert('xss')</script>")
            .content("Content")
            .author("user123")
            .build();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_note_builder_multiple_tags() {
        let note = NoteBuilder::new()
            .title("Test")
            .content("Content")
            .author("user123")
            .tags(vec!["tag1", "tag2", "tag3"])
            .build()
            .unwrap();
        
        assert_eq!(note.tags, vec!["tag1", "tag2", "tag3"]);
    }
    
    #[test]
    fn test_note_id_try_from() {
        // Test TryFrom<String>
        let id: Result<NoteId, _> = "notes:abc123".to_string().try_into();
        assert!(id.is_ok());
        assert_eq!(id.unwrap().to_string(), "notes:abc123");
        
        // Test TryFrom<&str>
        let id: Result<NoteId, _> = "notes:def456".try_into();
        assert!(id.is_ok());
        assert_eq!(id.unwrap().to_string(), "notes:def456");
        
        // Test invalid format
        let id: Result<NoteId, _> = "invalid".try_into();
        assert!(id.is_err());
    }
}