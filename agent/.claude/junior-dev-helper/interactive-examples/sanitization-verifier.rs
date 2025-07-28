/// Interactive Sanitization Pattern Verifier
/// 
/// This tool helps verify that all log sanitization patterns are working correctly.
/// Run with: cargo run --example sanitization-verifier

use regex::Regex;
use std::io::{self, Write};

struct SanitizationTest {
    name: &'static str,
    input: &'static str,
    expected: &'static str,
    pattern_name: &'static str,
}

fn main() {
    println!("=== Log Sanitization Pattern Verifier ===\n");
    
    let tests = vec![
        SanitizationTest {
            name: "Email Address",
            input: "User john.doe@example.com logged in successfully",
            expected: "User ***@example.com logged in successfully",
            pattern_name: "email",
        },
        SanitizationTest {
            name: "Credit Card",
            input: "Payment processed for card 4111111111111111",
            expected: "Payment processed for card [REDACTED]",
            pattern_name: "credit_card",
        },
        SanitizationTest {
            name: "API Key",
            input: "Authenticated with key sk_test_4242424242424242424242",
            expected: "Authenticated with key [REDACTED]",
            pattern_name: "api_key",
        },
        SanitizationTest {
            name: "Bearer Token",
            input: "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
            expected: "Authorization: Bearer [REDACTED]",
            pattern_name: "bearer_token",
        },
        SanitizationTest {
            name: "Password Field",
            input: "Login attempt with password=mysecretpass123",
            expected: "Login attempt with password=[REDACTED]",
            pattern_name: "password",
        },
        SanitizationTest {
            name: "IP Address",
            input: "Connection from 192.168.1.100 on port 8080",
            expected: "Connection from 192.168.x.x on port 8080",
            pattern_name: "ip_address",
        },
        SanitizationTest {
            name: "User Path",
            input: "Reading config from /home/alice/app/config.toml",
            expected: "Reading config from /[USER]/app/config.toml",
            pattern_name: "user_path",
        },
    ];
    
    // Run automated tests
    println!("Running automated tests...\n");
    let patterns = create_patterns();
    let mut passed = 0;
    let mut failed = 0;
    
    for test in &tests {
        let result = sanitize_with_patterns(&test.input, &patterns);
        if result == test.expected {
            println!("✅ {}: PASSED", test.name);
            passed += 1;
        } else {
            println!("❌ {}: FAILED", test.name);
            println!("   Input:    {}", test.input);
            println!("   Expected: {}", test.expected);
            println!("   Got:      {}", result);
            failed += 1;
        }
    }
    
    println!("\nTest Results: {} passed, {} failed", passed, failed);
    
    // Interactive mode
    println!("\n=== Interactive Mode ===");
    println!("Enter text to test sanitization (or 'quit' to exit):\n");
    
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        
        if input == "quit" || input == "exit" {
            break;
        }
        
        let sanitized = sanitize_with_patterns(input, &patterns);
        println!("Sanitized: {}", sanitized);
        
        // Show what was redacted
        if input != sanitized {
            println!("Changes made:");
            for (name, pattern) in &patterns {
                if pattern.is_match(input) {
                    println!("  - {} pattern matched", name);
                }
            }
        } else {
            println!("(No sensitive data detected)");
        }
        println!();
    }
    
    // Pattern reference
    println!("\n=== Pattern Reference ===");
    print_pattern_reference();
}

fn create_patterns() -> Vec<(&'static str, Regex)> {
    vec![
        ("email", Regex::new(r"\b([a-zA-Z0-9._%+-]+)@([a-zA-Z0-9.-]+\.[a-zA-Z]{2,})\b").unwrap()),
        ("credit_card", Regex::new(r"\b\d{13,19}\b").unwrap()),
        ("api_key", Regex::new(r"\b(sk_|pk_|api_|key_)[a-zA-Z0-9]{20,}\b").unwrap()),
        ("bearer_token", Regex::new(r"Bearer\s+[a-zA-Z0-9\-_\.]+").unwrap()),
        ("password", Regex::new(r"(?i)(password|passwd|pwd)\s*[:=]\s*\S+").unwrap()),
        ("ip_address", Regex::new(r"\b(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})\b").unwrap()),
        ("user_path", Regex::new(r"/(?:home|Users)/([^/]+)").unwrap()),
    ]
}

fn sanitize_with_patterns(input: &str, patterns: &[(&str, Regex)]) -> String {
    let mut result = input.to_string();
    
    for (name, pattern) in patterns {
        result = match *name {
            "email" => pattern.replace_all(&result, "***@$2").to_string(),
            "credit_card" => pattern.replace_all(&result, "[REDACTED]").to_string(),
            "api_key" => pattern.replace_all(&result, "[REDACTED]").to_string(),
            "bearer_token" => pattern.replace_all(&result, "Bearer [REDACTED]").to_string(),
            "password" => pattern.replace_all(&result, "$1=[REDACTED]").to_string(),
            "ip_address" => pattern.replace_all(&result, "$1.$2.x.x").to_string(),
            "user_path" => pattern.replace_all(&result, "/[USER]").to_string(),
            _ => result,
        };
    }
    
    result
}

fn print_pattern_reference() {
    println!("Pattern | Matches | Replacement");
    println!("--------|---------|------------");
    println!("Email   | user@example.com | ***@example.com");
    println!("Credit Card | 4111111111111111 | [REDACTED]");
    println!("API Key | sk_test_abc123... | [REDACTED]");
    println!("Bearer Token | Bearer eyJ... | Bearer [REDACTED]");
    println!("Password | password=secret | password=[REDACTED]");
    println!("IP Address | 192.168.1.1 | 192.168.x.x");
    println!("User Path | /home/alice/... | /[USER]/...");
    
    println!("\nCommon Prefixes for API Keys:");
    println!("- sk_ (secret key)");
    println!("- pk_ (public key)");
    println!("- api_");
    println!("- key_");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_multiple_patterns_in_one_line() {
        let patterns = create_patterns();
        let input = "User alice@example.com paid with 4111111111111111 using key sk_test_123456789012345678901234";
        let expected = "User ***@example.com paid with [REDACTED] using key [REDACTED]";
        
        let result = sanitize_with_patterns(input, &patterns);
        assert_eq!(result, expected);
    }
    
    #[test]
    fn test_case_insensitive_password() {
        let patterns = create_patterns();
        
        let test_cases = vec![
            ("PASSWORD=secret", "PASSWORD=[REDACTED]"),
            ("Password: mypass", "Password=[REDACTED]"),
            ("pwd=12345", "pwd=[REDACTED]"),
        ];
        
        for (input, expected) in test_cases {
            let result = sanitize_with_patterns(input, &patterns);
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }
    
    #[test]
    fn test_no_false_positives() {
        let patterns = create_patterns();
        
        let safe_inputs = vec![
            "Normal log message without sensitive data",
            "Port 8080 is listening",
            "User logged in successfully",
            "Processing payment",
        ];
        
        for input in safe_inputs {
            let result = sanitize_with_patterns(input, &patterns);
            assert_eq!(result, input, "False positive for: {}", input);
        }
    }
}