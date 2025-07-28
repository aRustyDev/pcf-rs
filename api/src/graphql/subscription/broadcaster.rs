use tokio::sync::broadcast;
use std::sync::{Arc, RwLock};
use crate::graphql::subscription::DomainEvent;

/// Event broadcasting system for GraphQL subscriptions
/// 
/// This broadcaster manages real-time event distribution to multiple subscribers
/// using Tokio's broadcast channel for efficient pub/sub pattern.
pub struct EventBroadcaster {
    sender: broadcast::Sender<DomainEvent>,
    capacity: usize,
    subscriber_count: Arc<RwLock<usize>>,
}

impl EventBroadcaster {
    /// Create a new event broadcaster with specified capacity
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            capacity,
            subscriber_count: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Send an event to all subscribers
    /// 
    /// Events are only sent if there are active subscribers to avoid unnecessary work.
    /// Returns the number of subscribers that received the event.
    pub async fn send(&self, event: DomainEvent) -> usize {
        let subscriber_count = *self.subscriber_count.read().unwrap();
        
        if subscriber_count > 0 {
            // Only send if there are active subscribers
            match self.sender.send(event) {
                Ok(count) => {
                    tracing::debug!("Event sent to {} subscribers", count);
                    count
                }
                Err(_) => {
                    tracing::warn!("No active subscribers for event");
                    0
                }
            }
        } else {
            tracing::trace!("No subscribers, event not sent");
            0
        }
    }
    
    /// Subscribe to events
    /// 
    /// Returns a receiver that will receive all future events.
    /// The subscriber count is automatically tracked.
    pub fn subscribe(&self) -> EventSubscription {
        *self.subscriber_count.write().unwrap() += 1;
        let receiver = self.sender.subscribe();
        
        EventSubscription {
            receiver,
            counter: Arc::downgrade(&self.subscriber_count),
        }
    }
    
    /// Get current subscriber count
    pub fn subscriber_count(&self) -> usize {
        *self.subscriber_count.read().unwrap()
    }
    
    /// Get the channel capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    
    /// Check if the broadcaster has any active subscribers
    pub fn has_subscribers(&self) -> bool {
        *self.subscriber_count.read().unwrap() > 0
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new(1000) // Default capacity of 1000 events
    }
}

/// Subscription handle that automatically decrements subscriber count when dropped
pub struct EventSubscription {
    receiver: broadcast::Receiver<DomainEvent>,
    counter: std::sync::Weak<RwLock<usize>>,
}

impl EventSubscription {
    /// Receive the next event
    pub async fn recv(&mut self) -> Result<DomainEvent, broadcast::error::RecvError> {
        self.receiver.recv().await
    }
    
    /// Try to receive an event without blocking
    pub fn try_recv(&mut self) -> Result<DomainEvent, broadcast::error::TryRecvError> {
        self.receiver.try_recv()
    }
    
    /// Get the number of messages in the channel
    pub fn len(&self) -> usize {
        self.receiver.len()
    }
    
    /// Check if the channel is empty
    pub fn is_empty(&self) -> bool {
        self.receiver.is_empty()
    }
}

impl Drop for EventSubscription {
    fn drop(&mut self) {
        if let Some(counter) = self.counter.upgrade() {
            if let Ok(mut count) = counter.write() {
                if *count > 0 {
                    *count -= 1;
                    tracing::debug!("Subscriber disconnected, {} remaining", *count);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Note;
    use tokio::time::{timeout, Duration};
    
    fn create_test_note() -> Note {
        Note::new(
            "Test Note".to_string(),
            "Test content".to_string(),
            "test_user".to_string(),
            vec!["test".to_string()],
        )
    }
    
    #[tokio::test]
    async fn test_event_broadcaster_creation() {
        let broadcaster = EventBroadcaster::new(100);
        assert_eq!(broadcaster.capacity(), 100);
        assert_eq!(broadcaster.subscriber_count(), 0);
        assert!(!broadcaster.has_subscribers());
    }
    
    #[tokio::test]
    async fn test_subscriber_count_tracking() {
        let broadcaster = EventBroadcaster::new(100);
        
        // No subscribers initially
        assert_eq!(broadcaster.subscriber_count(), 0);
        
        // Create subscriptions
        let _sub1 = broadcaster.subscribe();
        assert_eq!(broadcaster.subscriber_count(), 1);
        assert!(broadcaster.has_subscribers());
        
        let _sub2 = broadcaster.subscribe();
        assert_eq!(broadcaster.subscriber_count(), 2);
        
        // Drop first subscription
        drop(_sub1);
        assert_eq!(broadcaster.subscriber_count(), 1);
        
        // Drop second subscription
        drop(_sub2);
        assert_eq!(broadcaster.subscriber_count(), 0);
        assert!(!broadcaster.has_subscribers());
    }
    
    #[tokio::test]
    async fn test_event_broadcasting() {
        let broadcaster = EventBroadcaster::new(100);
        let mut subscription = broadcaster.subscribe();
        
        let test_note = create_test_note();
        let event = DomainEvent::NoteCreated(test_note.clone());
        
        // Send event
        let sent_count = broadcaster.send(event.clone()).await;
        assert_eq!(sent_count, 1);
        
        // Receive event
        let received_event = timeout(Duration::from_millis(100), subscription.recv())
            .await
            .expect("Should receive event within timeout")
            .expect("Should receive event successfully");
        
        match received_event {
            DomainEvent::NoteCreated(note) => {
                assert_eq!(note.title, test_note.title);
                assert_eq!(note.author, test_note.author);
            }
            _ => panic!("Expected NoteCreated event"),
        }
    }
    
    #[tokio::test]
    async fn test_multiple_subscribers() {
        let broadcaster = EventBroadcaster::new(100);
        let mut sub1 = broadcaster.subscribe();
        let mut sub2 = broadcaster.subscribe();
        
        assert_eq!(broadcaster.subscriber_count(), 2);
        
        let test_note = create_test_note();
        let event = DomainEvent::NoteCreated(test_note);
        
        // Send event to both subscribers
        let sent_count = broadcaster.send(event).await;
        assert_eq!(sent_count, 2);
        
        // Both should receive the event
        let event1 = timeout(Duration::from_millis(100), sub1.recv())
            .await
            .expect("Sub1 should receive event")
            .expect("Sub1 should receive successfully");
        
        let event2 = timeout(Duration::from_millis(100), sub2.recv())
            .await
            .expect("Sub2 should receive event")
            .expect("Sub2 should receive successfully");
        
        // Both should be the same event
        matches!(event1, DomainEvent::NoteCreated(_));
        matches!(event2, DomainEvent::NoteCreated(_));
    }
    
    #[tokio::test]
    async fn test_no_send_without_subscribers() {
        let broadcaster = EventBroadcaster::new(100);
        
        let test_note = create_test_note();
        let event = DomainEvent::NoteCreated(test_note);
        
        // Should not send if no subscribers
        let sent_count = broadcaster.send(event).await;
        assert_eq!(sent_count, 0);
    }
    
    #[tokio::test]
    async fn test_event_update_and_delete() {
        let broadcaster = EventBroadcaster::new(100);
        let mut subscription = broadcaster.subscribe();
        
        let old_note = create_test_note();
        let mut new_note = old_note.clone();
        new_note.title = "Updated Title".to_string();
        
        // Test update event
        let update_event = DomainEvent::NoteUpdated {
            old: old_note.clone(),
            new: new_note.clone(),
        };
        
        broadcaster.send(update_event).await;
        
        let received = timeout(Duration::from_millis(100), subscription.recv())
            .await
            .expect("Should receive update event")
            .expect("Should receive successfully");
        
        match received {
            DomainEvent::NoteUpdated { old, new } => {
                assert_eq!(old.title, "Test Note");
                assert_eq!(new.title, "Updated Title");
            }
            _ => panic!("Expected NoteUpdated event"),
        }
        
        // Test delete event
        let delete_event = DomainEvent::NoteDeleted(old_note.id.clone());
        broadcaster.send(delete_event).await;
        
        let received = timeout(Duration::from_millis(100), subscription.recv())
            .await
            .expect("Should receive delete event")
            .expect("Should receive successfully");
        
        match received {
            DomainEvent::NoteDeleted(id) => {
                assert_eq!(id, old_note.id);
            }
            _ => panic!("Expected NoteDeleted event"),
        }
    }
    
    #[tokio::test]
    async fn test_subscription_cleanup_on_drop() {
        let broadcaster = EventBroadcaster::new(100);
        
        {
            let _sub1 = broadcaster.subscribe();
            let _sub2 = broadcaster.subscribe();
            assert_eq!(broadcaster.subscriber_count(), 2);
            
            // Both subscriptions should be dropped when this scope ends
        }
        
        // Small delay to allow drop handlers to execute
        tokio::task::yield_now().await;
        
        assert_eq!(broadcaster.subscriber_count(), 0);
    }
    
    #[tokio::test]
    async fn test_try_recv_functionality() {
        let broadcaster = EventBroadcaster::new(100);
        let mut subscription = broadcaster.subscribe();
        
        // No events yet
        assert!(subscription.try_recv().is_err());
        assert!(subscription.is_empty());
        
        // Send event
        let test_note = create_test_note();
        let event = DomainEvent::NoteCreated(test_note);
        broadcaster.send(event).await;
        
        // Should be able to receive immediately
        assert!(!subscription.is_empty());
        assert_eq!(subscription.len(), 1);
        
        let received = subscription.try_recv().expect("Should receive event immediately");
        matches!(received, DomainEvent::NoteCreated(_));
        
        // Should be empty again
        assert!(subscription.is_empty());
        assert!(subscription.try_recv().is_err());
    }
    
    #[tokio::test]
    async fn test_default_broadcaster() {
        let broadcaster = EventBroadcaster::default();
        assert_eq!(broadcaster.capacity(), 1000);
        assert_eq!(broadcaster.subscriber_count(), 0);
    }
}