use regex::Regex;
use std::sync::OnceLock;

/// Sanitization patterns for removing sensitive data from logs
pub struct SanitizationPatterns {
    email: Regex,
    credit_card: Regex,
    api_key: Regex,
    bearer_token: Regex,
    password_field: Regex,
    ipv4_address: Regex,
    user_path: Regex,
}

static PATTERNS: OnceLock<SanitizationPatterns> = OnceLock::new();

/// Get compiled regex patterns for sanitization
pub fn get_patterns() -> &'static SanitizationPatterns {
    PATTERNS.get_or_init(|| SanitizationPatterns {
        // Email addresses - keep domain visible
        email: Regex::new(r"\b([a-zA-Z0-9._%+-]+)@([a-zA-Z0-9.-]+\.[a-zA-Z]{2,})\b").unwrap(),
        
        // Credit card numbers - any 13-19 digit sequence
        credit_card: Regex::new(r"\b\d{13,19}\b").unwrap(),
        
        // API keys - common patterns
        api_key: Regex::new(r"\b(sk_|pk_|api_|key_)[a-zA-Z0-9_]{15,}\b").unwrap(),
        
        // Bearer tokens
        bearer_token: Regex::new(r"Bearer\s+[a-zA-Z0-9\-_\.]+").unwrap(),
        
        // Password fields in various formats
        password_field: Regex::new(r"(?i)(password|passwd|pwd)\s*[:=]\s*\S+").unwrap(),
        
        // IPv4 addresses - show subnet only
        ipv4_address: Regex::new(r"\b(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})\b").unwrap(),
        
        // User home directories
        user_path: Regex::new(r"/(?:home|Users)/([^/]+)").unwrap(),
    })
}

/// Sanitize log message by removing sensitive data
/// 
/// This function applies multiple regex patterns to remove:
/// - Email addresses (shows ***@domain.com)
/// - Credit card numbers (shows [REDACTED])
/// - API keys and tokens (shows [REDACTED])
/// - Password fields (shows field=[REDACTED])
/// - IP addresses (shows subnet only like 192.168.x.x)
/// - User paths (shows /[USER]/)
pub fn sanitize_log_message(message: &str) -> String {
    let patterns = get_patterns();
    let mut result = message.to_string();
    
    // Replace emails with ***@domain
    result = patterns.email.replace_all(&result, "***@$2").to_string();
    
    // Replace credit cards
    result = patterns.credit_card.replace_all(&result, "[REDACTED]").to_string();
    
    // Replace API keys
    result = patterns.api_key.replace_all(&result, "[REDACTED]").to_string();
    
    // Replace bearer tokens
    result = patterns.bearer_token.replace_all(&result, "Bearer [REDACTED]").to_string();
    
    // Replace password fields
    result = patterns.password_field.replace_all(&result, "$1=[REDACTED]").to_string();
    
    // Replace IP addresses with subnet
    result = patterns.ipv4_address.replace_all(&result, "$1.$2.x.x").to_string();
    
    // Replace user paths
    result = patterns.user_path.replace_all(&result, "/[USER]").to_string();
    
    result
}