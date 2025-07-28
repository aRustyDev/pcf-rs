/// GraphQL Security Patterns - Phase 3 Implementation Examples
///
/// This file demonstrates security patterns required for GraphQL APIs including
/// query depth limiting, complexity calculation, and malicious query prevention.

use async_graphql::{
    parser::types::*,
    validation::{ValidationResult, Visitor, VisitorContext},
    Value, Variables,
};
use std::collections::HashMap;

/// Query Depth Limiter
/// 
/// Prevents deeply nested queries that could cause exponential resource usage
pub struct DepthLimiter {
    max_depth: usize,
    current_depth: usize,
}

impl DepthLimiter {
    pub fn new(max_depth: usize) -> Self {
        Self {
            max_depth,
            current_depth: 0,
        }
    }
}

impl Visitor for DepthLimiter {
    fn enter_field(&mut self, ctx: &mut VisitorContext<'_>, field: &Field) -> Result<(), String> {
        self.current_depth += 1;
        
        if self.current_depth > self.max_depth {
            ctx.report_error(
                vec![field.pos],
                format!(
                    "Query depth {} exceeds maximum allowed depth of {}",
                    self.current_depth, self.max_depth
                ),
            );
        }
        
        Ok(())
    }
    
    fn exit_field(&mut self, _ctx: &mut VisitorContext<'_>, _field: &Field) {
        self.current_depth -= 1;
    }
}

/// Query Complexity Calculator
/// 
/// Assigns costs to fields and calculates total query complexity
pub struct ComplexityCalculator {
    max_complexity: usize,
    current_complexity: usize,
    field_costs: HashMap<String, ComplexityCost>,
}

#[derive(Debug, Clone)]
pub enum ComplexityCost {
    Fixed(usize),
    Multiplier(usize), // Cost multiplied by limit argument
    Dynamic(Box<dyn Fn(&Field) -> usize + Send + Sync>),
}

impl ComplexityCalculator {
    pub fn new(max_complexity: usize) -> Self {
        let mut field_costs = HashMap::new();
        
        // Define costs for different fields
        field_costs.insert("note".to_string(), ComplexityCost::Fixed(1));
        field_costs.insert("notes".to_string(), ComplexityCost::Multiplier(1));
        field_costs.insert("searchNotes".to_string(), ComplexityCost::Fixed(50));
        field_costs.insert("content".to_string(), ComplexityCost::Fixed(1)); // Large field
        
        Self {
            max_complexity,
            current_complexity: 0,
            field_costs,
        }
    }
    
    fn calculate_field_cost(&self, field: &Field) -> usize {
        let field_name = field.name.node.as_str();
        
        match self.field_costs.get(field_name) {
            Some(ComplexityCost::Fixed(cost)) => *cost,
            Some(ComplexityCost::Multiplier(base_cost)) => {
                // Look for limit argument
                let limit = field.arguments.iter()
                    .find(|(name, _)| name.node == "limit")
                    .and_then(|(_, value)| {
                        if let Value::Number(n) = &value.node {
                            n.as_u64()
                        } else {
                            None
                        }
                    })
                    .unwrap_or(10) as usize;
                
                base_cost * limit
            }
            Some(ComplexityCost::Dynamic(calculator)) => calculator(field),
            None => 1, // Default cost
        }
    }
}

impl Visitor for ComplexityCalculator {
    fn enter_field(&mut self, ctx: &mut VisitorContext<'_>, field: &Field) -> Result<(), String> {
        let cost = self.calculate_field_cost(field);
        self.current_complexity += cost;
        
        if self.current_complexity > self.max_complexity {
            ctx.report_error(
                vec![field.pos],
                format!(
                    "Query complexity {} exceeds maximum allowed complexity of {}",
                    self.current_complexity, self.max_complexity
                ),
            );
        }
        
        Ok(())
    }
}

/// Alias Limiter
/// 
/// Prevents alias bombing attacks where many aliases are used for the same field
pub struct AliasLimiter {
    max_aliases_per_field: usize,
    field_alias_count: HashMap<String, usize>,
}

impl AliasLimiter {
    pub fn new(max_aliases_per_field: usize) -> Self {
        Self {
            max_aliases_per_field,
            field_alias_count: HashMap::new(),
        }
    }
}

impl Visitor for AliasLimiter {
    fn enter_field(&mut self, ctx: &mut VisitorContext<'_>, field: &Field) -> Result<(), String> {
        let field_name = field.name.node.as_str();
        let count = self.field_alias_count.entry(field_name.to_string()).or_insert(0);
        *count += 1;
        
        if *count > self.max_aliases_per_field {
            ctx.report_error(
                vec![field.pos],
                format!(
                    "Field '{}' has {} aliases, exceeding the maximum of {}",
                    field_name, count, self.max_aliases_per_field
                ),
            );
        }
        
        Ok(())
    }
}

/// Query Whitelist Validator
/// 
/// Only allows pre-approved queries in production
pub struct QueryWhitelist {
    allowed_queries: HashMap<String, String>,
}

impl QueryWhitelist {
    pub fn new() -> Self {
        let mut allowed_queries = HashMap::new();
        
        // Add allowed queries
        allowed_queries.insert(
            "GetNotes".to_string(),
            "query GetNotes($limit: Int) { notes(limit: $limit) { edges { node { id title } } } }".to_string()
        );
        
        allowed_queries.insert(
            "GetNoteById".to_string(),
            "query GetNoteById($id: ID!) { note(id: $id) { id title content author } }".to_string()
        );
        
        Self { allowed_queries }
    }
    
    pub fn is_allowed(&self, operation_name: Option<&str>, query: &str) -> bool {
        if let Some(name) = operation_name {
            self.allowed_queries.get(name)
                .map(|allowed| normalize_query(allowed) == normalize_query(query))
                .unwrap_or(false)
        } else {
            false
        }
    }
}

fn normalize_query(query: &str) -> String {
    // Simple normalization - in production use proper AST comparison
    query.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Rate Limiter for GraphQL Operations
/// 
/// Tracks and limits operations per time window
pub struct RateLimiter {
    limits: RateLimits,
    operation_counts: tokio::sync::Mutex<HashMap<String, OperationCount>>,
}

#[derive(Debug, Clone)]
pub struct RateLimits {
    pub queries_per_minute: u32,
    pub mutations_per_minute: u32,
    pub subscriptions_per_minute: u32,
    pub search_queries_per_minute: u32,
}

#[derive(Debug)]
struct OperationCount {
    count: u32,
    window_start: tokio::time::Instant,
}

impl RateLimiter {
    pub fn new(limits: RateLimits) -> Self {
        Self {
            limits,
            operation_counts: tokio::sync::Mutex::new(HashMap::new()),
        }
    }
    
    pub async fn check_rate_limit(
        &self,
        client_id: &str,
        operation_type: &str,
        operation_name: Option<&str>,
    ) -> Result<(), String> {
        let mut counts = self.operation_counts.lock().await;
        let now = tokio::time::Instant::now();
        let window = std::time::Duration::from_secs(60);
        
        let key = format!("{}:{}:{}", client_id, operation_type, operation_name.unwrap_or("unnamed"));
        
        let entry = counts.entry(key.clone()).or_insert(OperationCount {
            count: 0,
            window_start: now,
        });
        
        // Reset window if expired
        if now.duration_since(entry.window_start) > window {
            entry.count = 0;
            entry.window_start = now;
        }
        
        entry.count += 1;
        
        let limit = match (operation_type, operation_name) {
            ("Query", Some(name)) if name.contains("search") => self.limits.search_queries_per_minute,
            ("Query", _) => self.limits.queries_per_minute,
            ("Mutation", _) => self.limits.mutations_per_minute,
            ("Subscription", _) => self.limits.subscriptions_per_minute,
            _ => return Ok(()),
        };
        
        if entry.count > limit {
            Err(format!(
                "Rate limit exceeded: {} operations per minute allowed for {}",
                limit, operation_type
            ))
        } else {
            Ok(())
        }
    }
}

/// Introspection Guard
/// 
/// Prevents introspection queries in production
pub struct IntrospectionGuard {
    allow_introspection: bool,
}

impl IntrospectionGuard {
    pub fn new(allow_introspection: bool) -> Self {
        Self { allow_introspection }
    }
}

impl Visitor for IntrospectionGuard {
    fn enter_field(&mut self, ctx: &mut VisitorContext<'_>, field: &Field) -> Result<(), String> {
        if !self.allow_introspection && field.name.node.starts_with("__") {
            ctx.report_error(
                vec![field.pos],
                "Introspection is not allowed in production".to_string(),
            );
        }
        
        Ok(())
    }
}

/// Security Configuration for GraphQL
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub max_depth: usize,
    pub max_complexity: usize,
    pub max_aliases_per_field: usize,
    pub allow_introspection: bool,
    pub enable_query_whitelist: bool,
    pub rate_limits: RateLimits,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_depth: 15,
            max_complexity: 1000,
            max_aliases_per_field: 10,
            allow_introspection: cfg!(debug_assertions),
            enable_query_whitelist: false,
            rate_limits: RateLimits {
                queries_per_minute: 1000,
                mutations_per_minute: 100,
                subscriptions_per_minute: 10,
                search_queries_per_minute: 10,
            },
        }
    }
}

impl SecurityConfig {
    pub fn production() -> Self {
        Self {
            max_depth: 10,
            max_complexity: 500,
            max_aliases_per_field: 5,
            allow_introspection: false,
            enable_query_whitelist: true,
            rate_limits: RateLimits {
                queries_per_minute: 100,
                mutations_per_minute: 20,
                subscriptions_per_minute: 5,
                search_queries_per_minute: 5,
            },
        }
    }
    
    pub fn development() -> Self {
        Self {
            max_depth: 20,
            max_complexity: 2000,
            max_aliases_per_field: 20,
            allow_introspection: true,
            enable_query_whitelist: false,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_depth_calculation() {
        let query = r#"
            query {
                notes {
                    edges {
                        node {
                            author {
                                notes {
                                    edges {
                                        node {
                                            title
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        "#;
        
        // This query has depth of 7
        // Limiter with max_depth of 5 should reject it
        let mut limiter = DepthLimiter::new(5);
        // Test would fail at depth 6
    }
    
    #[test]
    fn test_complexity_calculation() {
        // Query requesting 100 notes, each with content field
        // Base cost: 1 * 100 = 100
        // Content fields: 1 * 100 = 100
        // Total: 200
        
        let calculator = ComplexityCalculator::new(150);
        // This should exceed the limit
    }
    
    #[test]
    fn test_rate_limiting() {
        use tokio::runtime::Runtime;
        
        let rt = Runtime::new().unwrap();
        let limiter = RateLimiter::new(RateLimits {
            queries_per_minute: 5,
            mutations_per_minute: 2,
            subscriptions_per_minute: 1,
            search_queries_per_minute: 1,
        });
        
        rt.block_on(async {
            let client = "test_client";
            
            // Should allow first 5 queries
            for i in 0..5 {
                assert!(limiter.check_rate_limit(client, "Query", None).await.is_ok());
            }
            
            // 6th query should fail
            assert!(limiter.check_rate_limit(client, "Query", None).await.is_err());
        });
    }
}

/// Example Integration with async-graphql
pub mod integration {
    use super::*;
    use async_graphql::{extensions::Extension, ServerError};
    
    /// Security Extension for async-graphql
    pub struct SecurityExtension {
        config: SecurityConfig,
        rate_limiter: RateLimiter,
    }
    
    impl SecurityExtension {
        pub fn new(config: SecurityConfig) -> Self {
            let rate_limiter = RateLimiter::new(config.rate_limits.clone());
            Self { config, rate_limiter }
        }
    }
    
    #[async_trait::async_trait]
    impl Extension for SecurityExtension {
        async fn request(
            &self,
            ctx: &async_graphql::extensions::ExtensionContext<'_>,
            next: async_graphql::extensions::NextRequest<'_>,
        ) -> async_graphql::Response {
            // Extract client ID from headers or session
            let client_id = ctx.data::<String>()
                .map(|s| s.as_str())
                .unwrap_or("anonymous");
            
            let operation_type = ctx.operation.node.ty.to_string();
            let operation_name = ctx.operation.node.name.as_ref().map(|n| n.node.as_str());
            
            // Check rate limit
            if let Err(err) = self.rate_limiter
                .check_rate_limit(client_id, &operation_type, operation_name)
                .await
            {
                return async_graphql::Response::from_errors(vec![
                    ServerError::new(err, None)
                ]);
            }
            
            // Continue with request
            next.run(ctx).await
        }
    }
}