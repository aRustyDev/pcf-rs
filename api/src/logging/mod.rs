pub mod sanitization;
pub mod subscriber;
pub mod tracing;

pub use sanitization::*;
pub use subscriber::*;
pub use tracing::*;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sanitize_all_patterns() {
        // Test cases from WORK_PLAN.md section 1.4.3
        let test_cases = vec![
            // Email addresses - keep domain visible  
            ("User john.doe@example.com logged in", "User ***@example.com logged in"),
            
            // Credit card numbers - any 13-19 digit sequence
            ("Payment with card 4111111111111111", "Payment with card [REDACTED]"),
            
            // API keys - common patterns
            ("Using key sk_test_1234567890abcdefghij", "Using key [REDACTED]"),
            ("API key: api_key_abcdef123456", "API key: [REDACTED]"),
            
            // Bearer tokens
            ("Authorization: Bearer eyJhbGciOiJIUzI1NiIs", "Authorization: Bearer [REDACTED]"),
            
            // Password fields in various formats
            ("password=secret123", "password=[REDACTED]"),
            ("pwd: mysecret", "pwd=[REDACTED]"),
            ("Password: test123", "Password=[REDACTED]"),
            
            // IPv4 addresses - show subnet only
            ("Connected from 192.168.1.100", "Connected from 192.168.x.x"),
            ("Server at 10.0.0.5 responded", "Server at 10.0.x.x responded"),
            
            // User home directories
            ("Reading /home/john/config", "Reading /[USER]/config"),
            ("File at /Users/jane/Documents", "File at /[USER]/Documents"),
        ];
        
        for (input, expected) in test_cases {
            let result = sanitize_log_message(input);
            assert_eq!(result, expected, "Failed to sanitize: {}", input);
        }
    }
    
    #[test]
    fn test_multiple_patterns_in_one_message() {
        let input = "User john@example.com used card 4111111111111111 from IP 192.168.1.100";
        let expected = "User ***@example.com used card [REDACTED] from IP 192.168.x.x";
        let result = sanitize_log_message(input);
        assert_eq!(result, expected);
    }
    
    #[test]
    fn test_no_sensitive_data_unchanged() {
        let input = "Normal log message with no sensitive data";
        let result = sanitize_log_message(input);
        assert_eq!(result, input);
    }
    
    #[test]
    fn test_trace_id_generation() {
        let trace_id1 = generate_trace_id();
        let trace_id2 = generate_trace_id();
        
        // Trace IDs should be unique
        assert_ne!(trace_id1, trace_id2);
        
        // Should be valid UUID format (36 characters with hyphens)
        assert_eq!(trace_id1.len(), 36);
        assert!(trace_id1.contains('-'));
    }
    
    #[test]
    fn test_tracing_subscriber_initialization() {
        // Test that subscriber initialization doesn't panic for valid configs
        let json_config = crate::config::LoggingConfig {
            level: "info".to_string(),
            format: "json".to_string(),
        };
        
        let _pretty_config = crate::config::LoggingConfig {
            level: "debug".to_string(),
            format: "pretty".to_string(),
        };
        
        let invalid_config = crate::config::LoggingConfig {
            level: "info".to_string(),
            format: "invalid".to_string(),
        };
        
        // The first initialization should succeed (or fail if already initialized)
        let _ = setup_tracing(&json_config);
        
        // Test invalid format returns error
        let result_invalid = setup_tracing(&invalid_config);
        assert!(result_invalid.is_err());
        assert!(result_invalid.unwrap_err().to_string().contains("Unsupported log format"));
    }

    #[test]
    fn test_sanitized_macros() {
        // Test that sanitized macros work correctly
        // Note: These macros call the sanitization function, so we verify they compile and work
        let test_message = "User john@example.com used password=secret123";
        let sanitized = sanitize_log_message(test_message);
        
        // Verify sanitization worked
        assert_eq!(sanitized, "User ***@example.com used password=[REDACTED]");
        
        // The macros themselves can't be easily unit tested since they emit logs,
        // but we can ensure they compile and the underlying sanitization works
        assert!(!sanitized.contains("john"));
        assert!(!sanitized.contains("secret123"));
    }
}