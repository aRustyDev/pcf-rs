use async_graphql::*;
use async_graphql::connection::Connection;
use base64::{Engine as _, engine::general_purpose};
use crate::services::database::DatabaseService;
use crate::schema::Note;
use std::sync::Arc;

/// Cursor-based pagination following Relay specification
pub async fn query_notes_paginated(
    database: Arc<dyn DatabaseService>,
    limit: i32,
    after: Option<String>,
    before: Option<String>,
) -> Result<Connection<String, Note>> {
    let mut connection = Connection::new(false, false);
    
    // Decode cursors to get IDs
    let after_id = after.as_ref().and_then(|c| decode_cursor(c));
    let before_id = before.as_ref().and_then(|c| decode_cursor(c));
    
    // Build database query with cursor constraints
    let mut filter = std::collections::HashMap::new();
    
    // Add cursor-based filtering
    if let Some(after_id) = &after_id {
        // For simplicity, we'll use a basic comparison
        // In a real implementation, this would use proper cursor-based pagination
        filter.insert("id_gt".to_string(), serde_json::Value::String(after_id.clone()));
    }
    if let Some(before_id) = &before_id {
        filter.insert("id_lt".to_string(), serde_json::Value::String(before_id.clone()));
    }
    
    let query = crate::services::database::Query {
        filter,
        limit: Some((limit + 1) as usize), // Fetch one extra to check for next page
        offset: None,
        sort: Some({
            let mut sort = std::collections::HashMap::new();
            sort.insert("id".to_string(), crate::services::database::SortOrder::Asc);
            sort
        }),
    };
    
    let results = database
        .query("notes", query)
        .await
        .map_err(|e| Error::new(format!("Database error: {}", e)))?;
    
    let has_next_page = results.len() > limit as usize;
    let notes: Result<Vec<Note>, _> = results
        .into_iter()
        .take(limit as usize) // Take only the requested amount
        .map(|data| {
            serde_json::from_value(data)
                .map_err(|e| Error::new(format!("Failed to deserialize note: {}", e)))
        })
        .collect();
    
    let notes = notes?;
    
    // Build edges with cursors
    for note in notes {
        let cursor = encode_cursor(&note.id);
        connection.edges.push(async_graphql::connection::Edge::new(cursor.clone(), note));
    }
    
    // Set page info using the correct Connection API
    connection.has_next_page = has_next_page;
    connection.has_previous_page = after.is_some();
    
    Ok(connection)
}

/// Encode a note ID as a base64 cursor
pub fn encode_cursor(id: &str) -> String {
    general_purpose::STANDARD.encode(id)
}

/// Decode a base64 cursor to get the note ID
pub fn decode_cursor(cursor: &str) -> Option<String> {
    general_purpose::STANDARD
        .decode(cursor)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::database::MockDatabase;
    
    #[test]
    fn test_cursor_encoding() {
        let id = "notes:test123";
        let cursor = encode_cursor(id);
        let decoded = decode_cursor(&cursor);
        
        assert_eq!(decoded.unwrap(), id);
    }
    
    #[test] 
    fn test_invalid_cursor_decoding() {
        let invalid_cursor = "invalid!!!base64";
        let decoded = decode_cursor(invalid_cursor);
        
        assert!(decoded.is_none());
    }
    
    #[tokio::test]
    async fn test_pagination_basic_functionality() {
        let database = Arc::new(MockDatabase::new());
        let result = query_notes_paginated(database, 10, None, None).await;
        
        // Should now work since we implemented it
        assert!(result.is_ok());
        let connection = result.unwrap();
        
        // MockDatabase returns empty results, so should have no edges
        assert_eq!(connection.edges.len(), 0);
        assert!(!connection.has_next_page);
        assert!(!connection.has_previous_page);
    }
}