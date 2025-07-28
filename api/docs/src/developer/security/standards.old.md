# Security Standards

Comprehensive security standards and guidelines for the PCF API, covering authentication, authorization, data protection, and secure development practices.

<!-- toc -->

## Overview

Security is a fundamental requirement for the PCF API. This document defines the security standards, implementation guidelines, and best practices that must be followed throughout the codebase. These standards are based on industry best practices including OWASP guidelines, zero-trust principles, and defense-in-depth strategies.

## Core Security Principles

### 1. Defense in Depth

Implement multiple layers of security controls:

```rust
// Example: Multiple validation layers
pub async fn create_note(
    ctx: &Context<'_>,
    input: CreateNoteInput,
) -> Result<Note> {
    // Layer 1: Authentication
    let session = ctx.require_auth()?;
    
    // Layer 2: Input validation
    input.validate()?;
    
    // Layer 3: Authorization
    if !session.can_create_notes() {
        return Err(AppError::Forbidden);
    }
    
    // Layer 4: Business logic validation
    validate_note_content(&input)?;
    
    // Layer 5: Database constraints
    let note = database.create_note(input).await?;
    
    Ok(note)
}
```

### 2. Principle of Least Privilege

Grant minimum necessary permissions:

```rust
// Role-based access control
#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    Guest,      // Read public content only
    User,       // CRUD own content
    Moderator,  // Manage all content
    Admin,      // Full system access
}

impl Session {
    pub fn can_read(&self, resource: &Resource) -> bool {
        match (self.role, resource.visibility) {
            (_, Visibility::Public) => true,
            (Role::Guest, _) => false,
            (_, Visibility::Private) => resource.owner == self.user_id,
            (Role::Moderator | Role::Admin, _) => true,
            _ => false,
        }
    }
}
```

### 3. Zero Trust

Never trust, always verify:

```rust
// Verify authentication on every request
pub async fn graphql_handler(
    headers: HeaderMap,
    State(schema): State<AppSchema>,
    Json(request): Json<Request>,
) -> impl IntoResponse {
    // Extract and verify auth token
    let token = extract_token(&headers);
    let session = verify_token(token).await?;
    
    // Create fresh context for each request
    let context = GraphQLContext {
        session: Some(session),
        request_id: Uuid::new_v4(),
        // Never reuse context between requests
    };
    
    let response = schema.execute(request.data(context)).await;
    Json(response)
}
```

### 4. Fail Secure

Default to secure state on failure:

```rust
// Secure error handling
impl From<DatabaseError> for AppError {
    fn from(err: DatabaseError) -> Self {
        // Log detailed error internally
        error!("Database error: {:?}", err);
        
        // Return generic error to client
        match err {
            DatabaseError::NotFound(_) => AppError::NotFound,
            _ => AppError::InternalError, // Don't leak details
        }
    }
}

// Secure defaults
impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_introspection: false,  // Secure default
            max_query_depth: 10,          // Restrictive default
            max_query_complexity: 500,    // Conservative limit
            enable_playground: false,     // Production default
        }
    }
}
```

## Authentication Standards

### Session Management

Implement secure session handling:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub role: Role,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub ip_address: IpAddr,
    pub user_agent: String,
}

impl Session {
    /// Create new session with security constraints
    pub fn new(user: &User, request: &Request) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id: user.id,
            role: user.role,
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(24), // 24h max
            ip_address: extract_ip(request),
            user_agent: extract_user_agent(request),
        }
    }
    
    /// Validate session is still valid
    pub fn is_valid(&self) -> bool {
        self.expires_at > Utc::now()
    }
    
    /// Refresh session with sliding expiration
    pub fn refresh(&mut self) {
        if self.is_valid() {
            self.expires_at = Utc::now() + Duration::hours(1); // Sliding window
        }
    }
}
```

### Token Security

Secure token handling for future JWT implementation:

```rust
/// Token validation with multiple checks
pub async fn verify_token(token: &str) -> Result<Session, AuthError> {
    // Check token format
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AuthError::InvalidToken);
    }
    
    // Verify signature
    let claims = verify_jwt_signature(token)?;
    
    // Check expiration
    if claims.exp < Utc::now().timestamp() {
        return Err(AuthError::TokenExpired);
    }
    
    // Check issuer
    if claims.iss != expected_issuer() {
        return Err(AuthError::InvalidIssuer);
    }
    
    // Verify session still exists
    let session = get_session(&claims.session_id).await?;
    
    // Check for token reuse
    if session.last_used_token == token {
        return Err(AuthError::TokenReuse);
    }
    
    Ok(session)
}
```

### Password Security

Implement secure password handling:

```rust
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

/// Hash password with Argon2id
pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    // Validate password strength
    validate_password_strength(password)?;
    
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| PasswordError::HashingFailed(e.to_string()))?;
        
    Ok(password_hash.to_string())
}

/// Validate password meets security requirements
fn validate_password_strength(password: &str) -> Result<(), PasswordError> {
    if password.len() < 12 {
        return Err(PasswordError::TooShort);
    }
    
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_digit(10));
    let has_special = password.chars().any(|c| !c.is_alphanumeric());
    
    if !has_uppercase || !has_lowercase || !has_digit || !has_special {
        return Err(PasswordError::TooWeak);
    }
    
    // Check against common passwords
    if is_common_password(password) {
        return Err(PasswordError::TooCommon);
    }
    
    Ok(())
}
```

## Authorization Standards

### Role-Based Access Control (RBAC)

Implement granular permissions:

```rust
/// Permission system
#[derive(Debug, Clone, PartialEq)]
pub struct Permission {
    pub resource: ResourceType,
    pub action: Action,
    pub scope: Scope,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceType {
    Note,
    User,
    Project,
    System,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Create,
    Read,
    Update,
    Delete,
    List,
    Admin,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Scope {
    Own,      // Own resources only
    Team,     // Team resources
    Global,   // All resources
}

/// Check permission
impl Session {
    pub fn has_permission(&self, permission: &Permission) -> bool {
        let role_permissions = match self.role {
            Role::Admin => vec![
                Permission {
                    resource: ResourceType::System,
                    action: Action::Admin,
                    scope: Scope::Global,
                },
                // ... all permissions
            ],
            Role::User => vec![
                Permission {
                    resource: ResourceType::Note,
                    action: Action::Create,
                    scope: Scope::Own,
                },
                // ... limited permissions
            ],
            _ => vec![],
        };
        
        role_permissions.contains(permission)
    }
}
```

### Resource-Level Security

Implement row-level security:

```rust
/// Secure query filtering
pub async fn list_notes(
    ctx: &Context<'_>,
    filter: Option<NoteFilter>,
) -> Result<Vec<Note>> {
    let session = ctx.require_auth()?;
    let db = ctx.database();
    
    // Build secure query based on permissions
    let mut query = Query::new("notes");
    
    match session.role {
        Role::Admin => {
            // No filtering, can see all
        }
        Role::User => {
            // Only own notes or public
            query = query.filter(
                Or(
                    Eq("owner_id", session.user_id),
                    Eq("visibility", "public")
                )
            );
        }
        Role::Guest => {
            // Only public notes
            query = query.filter(Eq("visibility", "public"));
        }
    }
    
    // Apply user filter on top
    if let Some(filter) = filter {
        query = apply_filter(query, filter);
    }
    
    db.query(query).await
}
```

## Input Validation Standards

### Comprehensive Validation

Validate all inputs at multiple levels:

```rust
use garde::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateNoteInput {
    #[garde(length(min = 1, max = 200))]
    #[garde(custom(no_script_tags))]
    pub title: String,
    
    #[garde(length(min = 1, max = 10000))]
    #[garde(custom(no_script_tags))]
    #[garde(custom(no_sql_injection))]
    pub content: String,
    
    #[garde(length(max = 10))]
    pub tags: Vec<String>,
    
    #[garde(custom(valid_visibility))]
    pub visibility: String,
}

/// Prevent XSS attacks
fn no_script_tags(value: &str, _: &()) -> garde::Result {
    if value.to_lowercase().contains("<script") {
        return Err(garde::Error::new("Script tags not allowed"));
    }
    Ok(())
}

/// Prevent SQL injection
fn no_sql_injection(value: &str, _: &()) -> garde::Result {
    let dangerous_patterns = [
        "'; DROP TABLE",
        "1=1",
        "' OR '",
        "UNION SELECT",
        "--",
        "/*",
        "*/",
    ];
    
    for pattern in &dangerous_patterns {
        if value.to_uppercase().contains(pattern) {
            return Err(garde::Error::new("Potentially dangerous input"));
        }
    }
    
    Ok(())
}
```

### Type Safety

Leverage Rust's type system:

```rust
// Use strong types instead of strings
#[derive(Debug, Clone)]
pub struct UserId(Uuid);

#[derive(Debug, Clone)]
pub struct ProjectId(Uuid);

#[derive(Debug, Clone)]
pub struct Email(String);

impl Email {
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        
        if !email_regex.is_match(s) {
            return Err(ValidationError::InvalidEmail);
        }
        
        Ok(Email(s.to_lowercase()))
    }
}

// Prevents mixing up IDs
fn assign_to_project(user: UserId, project: ProjectId) -> Result<()> {
    // Type system prevents passing ProjectId as UserId
}
```

## API Security Standards

### GraphQL Security

Implement comprehensive GraphQL security:

```rust
/// Query depth limiting
pub struct DepthLimit {
    max_depth: usize,
}

impl Extension for DepthLimit {
    async fn parse_query(
        &self,
        ctx: &ExtensionContext<'_>,
        query: &ExecutableDocument,
        variables: &Variables,
    ) -> Result<()> {
        let depth = calculate_query_depth(query);
        
        if depth > self.max_depth {
            return Err(Error::new(format!(
                "Query depth {} exceeds maximum allowed depth of {}",
                depth, self.max_depth
            )));
        }
        
        Ok(())
    }
}

/// Query complexity analysis
pub struct ComplexityLimit {
    max_complexity: usize,
}

impl ComplexityLimit {
    fn calculate_field_complexity(&self, field: &Field) -> usize {
        let mut complexity = 1;
        
        // Lists multiply complexity
        if field.ty.is_list() {
            let first = field.arguments.get("first")
                .and_then(|v| v.as_i64())
                .unwrap_or(10) as usize;
                
            complexity *= first.min(100); // Cap multiplier
        }
        
        // Nested selections add complexity
        for selection in &field.selection_set {
            complexity += self.calculate_selection_complexity(selection);
        }
        
        complexity
    }
}
```

### Rate Limiting

Implement request rate limiting:

```rust
/// Rate limiter configuration
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
    pub block_duration: Duration,
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(limiter): State<Arc<RateLimiter>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let client_id = extract_client_id(&headers);
    
    match limiter.check_rate_limit(&client_id).await {
        Ok(_) => next.run(request).await,
        Err(RateLimitError::Exceeded { retry_after }) => {
            Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .header("Retry-After", retry_after.as_secs().to_string())
                .body("Rate limit exceeded".into())
                .unwrap()
        }
    }
}

/// Distributed rate limiter using Redis
pub struct RateLimiter {
    redis: Arc<RedisClient>,
    config: RateLimitConfig,
}

impl RateLimiter {
    pub async fn check_rate_limit(&self, client_id: &str) -> Result<(), RateLimitError> {
        let key = format!("rate_limit:{}", client_id);
        let now = Utc::now().timestamp();
        let window_start = now - 60; // 1-minute window
        
        // Remove old entries
        self.redis.zremrangebyscore(&key, 0, window_start).await?;
        
        // Count requests in current window
        let count: i64 = self.redis.zcard(&key).await?;
        
        if count >= self.config.requests_per_minute as i64 {
            // Check if client is blocked
            let block_key = format!("rate_limit:block:{}", client_id);
            if self.redis.exists(&block_key).await? {
                let ttl = self.redis.ttl(&block_key).await?;
                return Err(RateLimitError::Exceeded {
                    retry_after: Duration::from_secs(ttl as u64),
                });
            }
            
            // Block client
            self.redis.setex(
                &block_key,
                self.config.block_duration.as_secs() as i64,
                "1"
            ).await?;
            
            return Err(RateLimitError::Exceeded {
                retry_after: self.config.block_duration,
            });
        }
        
        // Add current request
        self.redis.zadd(&key, now, now).await?;
        self.redis.expire(&key, 60).await?;
        
        Ok(())
    }
}
```

## Data Security Standards

### Encryption at Rest

Implement field-level encryption:

```rust
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, NewAead};

/// Encrypt sensitive fields
pub struct FieldEncryption {
    cipher: Aes256Gcm,
}

impl FieldEncryption {
    pub fn new(key: &[u8; 32]) -> Self {
        let key = Key::from_slice(key);
        let cipher = Aes256Gcm::new(key);
        
        Self { cipher }
    }
    
    pub fn encrypt(&self, plaintext: &str) -> Result<String, EncryptionError> {
        let nonce = generate_nonce();
        let ciphertext = self.cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;
            
        // Combine nonce and ciphertext
        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);
        
        Ok(base64::encode(&result))
    }
    
    pub fn decrypt(&self, ciphertext: &str) -> Result<String, EncryptionError> {
        let data = base64::decode(ciphertext)
            .map_err(|e| EncryptionError::InvalidCiphertext(e.to_string()))?;
            
        if data.len() < 12 {
            return Err(EncryptionError::InvalidCiphertext("Too short".into()));
        }
        
        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        let plaintext = self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;
            
        String::from_utf8(plaintext)
            .map_err(|e| EncryptionError::InvalidPlaintext(e.to_string()))
    }
}

/// Encrypted field wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encrypted<T> {
    #[serde(rename = "value")]
    ciphertext: String,
    #[serde(skip)]
    _phantom: PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned> Encrypted<T> {
    pub fn encrypt(value: &T, encryption: &FieldEncryption) -> Result<Self, EncryptionError> {
        let json = serde_json::to_string(value)?;
        let ciphertext = encryption.encrypt(&json)?;
        
        Ok(Self {
            ciphertext,
            _phantom: PhantomData,
        })
    }
    
    pub fn decrypt(&self, encryption: &FieldEncryption) -> Result<T, EncryptionError> {
        let json = encryption.decrypt(&self.ciphertext)?;
        Ok(serde_json::from_str(&json)?)
    }
}
```

### Data Masking

Implement PII masking:

```rust
/// Mask sensitive data for display
pub trait Maskable {
    fn mask(&self) -> String;
}

impl Maskable for Email {
    fn mask(&self) -> String {
        let parts: Vec<&str> = self.0.split('@').collect();
        if parts.len() != 2 {
            return "***".to_string();
        }
        
        let local = parts[0];
        let domain = parts[1];
        
        if local.len() <= 2 {
            format!("***@{}", domain)
        } else {
            format!("{}***@{}", &local[..2], domain)
        }
    }
}

impl Maskable for CreditCard {
    fn mask(&self) -> String {
        let digits = &self.0;
        if digits.len() < 8 {
            return "****".to_string();
        }
        
        format!("****{}", &digits[digits.len()-4..])
    }
}

impl Maskable for PhoneNumber {
    fn mask(&self) -> String {
        let phone = &self.0;
        if phone.len() < 4 {
            return "****".to_string();
        }
        
        format!("***{}", &phone[phone.len()-4..])
    }
}
```

## Logging Security Standards

### Secure Logging Practices

Implement comprehensive log sanitization:

```rust
/// Sanitization patterns
pub struct LogSanitizer {
    patterns: Vec<(Regex, &'static str)>,
}

impl LogSanitizer {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                // Email addresses
                (Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap(), "***@***.***"),
                
                // Credit card numbers
                (Regex::new(r"\b\d{13,19}\b").unwrap(), "[CREDIT_CARD]"),
                
                // API keys
                (Regex::new(r"\b(sk_|pk_|api_|key_)[a-zA-Z0-9]{20,}\b").unwrap(), "[API_KEY]"),
                
                // JWT tokens
                (Regex::new(r"Bearer\s+[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+").unwrap(), "Bearer [JWT]"),
                
                // Passwords in URLs
                (Regex::new(r"(password|pwd|pass)=[^&\s]+").unwrap(), "$1=[REDACTED]"),
                
                // Session IDs
                (Regex::new(r"session_id=[a-f0-9\-]{32,}").unwrap(), "session_id=[SESSION]"),
            ],
        }
    }
    
    pub fn sanitize(&self, message: &str) -> String {
        let mut result = message.to_string();
        
        for (pattern, replacement) in &self.patterns {
            result = pattern.replace_all(&result, *replacement).to_string();
        }
        
        result
    }
}

/// Secure logging macros
#[macro_export]
macro_rules! log_secure {
    ($level:expr, $($arg:tt)*) => {
        let message = format!($($arg)*);
        let sanitized = LOG_SANITIZER.sanitize(&message);
        log::log!($level, "{}", sanitized);
    };
}
```

### Audit Logging

Implement security audit trail:

```rust
#[derive(Debug, Serialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub user_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub ip_address: IpAddr,
    pub user_agent: String,
    pub action: AuditAction,
    pub resource: AuditResource,
    pub result: AuditResult,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Serialize)]
pub enum AuditAction {
    Login,
    Logout,
    Create,
    Read,
    Update,
    Delete,
    Export,
    Admin,
}

#[derive(Debug, Serialize)]
pub enum AuditResult {
    Success,
    Failure { reason: String },
    Blocked { reason: String },
}

/// Audit logger
pub struct AuditLogger {
    writer: Arc<dyn AuditWriter>,
}

impl AuditLogger {
    pub async fn log(&self, event: AuditEvent) {
        // Never fail the operation due to audit logging
        if let Err(e) = self.writer.write(event).await {
            error!("Failed to write audit log: {}", e);
        }
    }
    
    pub async fn log_authentication(
        &self,
        user_id: Option<Uuid>,
        success: bool,
        metadata: HashMap<String, Value>,
    ) {
        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            user_id,
            action: AuditAction::Login,
            result: if success {
                AuditResult::Success
            } else {
                AuditResult::Failure {
                    reason: "Invalid credentials".to_string(),
                }
            },
            metadata,
            // ... other fields
        };
        
        self.log(event).await;
    }
}
```

## Network Security Standards

### HTTPS/TLS Configuration

Enforce secure communications:

```rust
/// TLS configuration
pub struct TlsConfig {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub min_tls_version: TlsVersion,
    pub cipher_suites: Vec<CipherSuite>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: PathBuf::from("/etc/certs/server.crt"),
            key_path: PathBuf::from("/etc/certs/server.key"),
            min_tls_version: TlsVersion::V1_3,
            cipher_suites: vec![
                CipherSuite::TLS13_AES_256_GCM_SHA384,
                CipherSuite::TLS13_AES_128_GCM_SHA256,
                CipherSuite::TLS13_CHACHA20_POLY1305_SHA256,
            ],
        }
    }
}

/// Create secure server
pub async fn create_tls_server(config: TlsConfig) -> Result<Server> {
    let tls_config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(
            load_certs(&config.cert_path)?,
            load_private_key(&config.key_path)?,
        )?;
        
    let acceptor = TlsAcceptor::from(Arc::new(tls_config));
    
    Server::bind_tls(&config.address, acceptor)
        .serve(app)
        .await
}
```

### CORS Configuration

Implement secure CORS policy:

```rust
/// Production CORS configuration
pub fn create_cors_layer(config: &CorsConfig) -> CorsLayer {
    CorsLayer::new()
        // Specific allowed origins
        .allow_origin(
            config.allowed_origins
                .iter()
                .map(|origin| origin.parse::<HeaderValue>().unwrap())
                .collect::<Vec<_>>()
        )
        // Specific allowed methods
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        // Specific allowed headers
        .allow_headers([
            AUTHORIZATION,
            CONTENT_TYPE,
            HeaderName::from_static("x-trace-id"),
        ])
        // Credentials support
        .allow_credentials(config.allow_credentials)
        // Cache preflight
        .max_age(Duration::from_secs(86400)) // 24 hours
}

/// CORS configuration
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CorsConfig {
    #[garde(length(min = 1))]
    pub allowed_origins: Vec<String>,
    
    pub allow_credentials: bool,
    
    #[garde(custom(validate_origins))]
    _validator: (),
}

fn validate_origins(_: &(), context: &CorsConfig) -> garde::Result {
    for origin in &context.allowed_origins {
        if origin == "*" && context.allow_credentials {
            return Err(garde::Error::new(
                "Cannot use wildcard origin with credentials"
            ));
        }
    }
    Ok(())
}
```

### Security Headers

Implement comprehensive security headers:

```rust
/// Security headers middleware
pub fn security_headers_middleware() -> impl Layer<Route> {
    ServiceBuilder::new()
        // Prevent MIME type sniffing
        .layer(SetResponseHeaderLayer::overriding(
            X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        // Prevent clickjacking
        .layer(SetResponseHeaderLayer::overriding(
            X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        // Enable XSS protection
        .layer(SetResponseHeaderLayer::overriding(
            X_XSS_PROTECTION,
            HeaderValue::from_static("1; mode=block"),
        ))
        // Content Security Policy
        .layer(SetResponseHeaderLayer::overriding(
            CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; \
                 script-src 'self' 'unsafe-inline' 'unsafe-eval'; \
                 style-src 'self' 'unsafe-inline'; \
                 img-src 'self' data: https:; \
                 font-src 'self'; \
                 connect-src 'self'; \
                 frame-ancestors 'none'; \
                 base-uri 'self'; \
                 form-action 'self';"
            ),
        ))
        // Strict Transport Security
        .layer(SetResponseHeaderLayer::overriding(
            STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"),
        ))
        // Referrer Policy
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        // Permissions Policy
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static(
                "accelerometer=(), camera=(), geolocation=(), \
                 gyroscope=(), magnetometer=(), microphone=(), \
                 payment=(), usb=()"
            ),
        ))
}
```

## Development Security Standards

### Secure Development Practices

Implement security in the development lifecycle:

```rust
/// Security linting configuration
#[cfg(debug_assertions)]
pub fn check_security_issues() {
    // Check for hardcoded secrets
    let secret_patterns = [
        r"password\s*=\s*['\"][^'\"]+['\"]",
        r"api_key\s*=\s*['\"][^'\"]+['\"]",
        r"secret\s*=\s*['\"][^'\"]+['\"]",
    ];
    
    for pattern in &secret_patterns {
        let regex = Regex::new(pattern).unwrap();
        // Scan source files
    }
    
    // Check for unsafe code
    #[deny(unsafe_code)]
    fn _ensure_no_unsafe() {}
    
    // Check for common vulnerabilities
    #[forbid(deprecated)]
    fn _ensure_no_deprecated() {}
}

/// Dependency security checks
pub fn check_dependencies() -> Result<(), SecurityError> {
    // Parse Cargo.lock
    let lockfile = Lockfile::load("Cargo.lock")?;
    
    // Check against vulnerability database
    let db = VulnerabilityDatabase::fetch()?;
    
    for package in &lockfile.packages {
        if let Some(vulns) = db.vulnerabilities_for(&package.name, &package.version) {
            for vuln in vulns {
                match vuln.severity {
                    Severity::Critical | Severity::High => {
                        return Err(SecurityError::VulnerableDepe
```

### Security Testing

Implement comprehensive security tests:

```rust
#[cfg(test)]
mod security_tests {
    use super::*;
    
    #[test]
    fn test_sql_injection_prevention() {
        let malicious_inputs = vec![
            "'; DROP TABLE users; --",
            "1' OR '1'='1",
            "admin'--",
            "1' UNION SELECT * FROM users--",
        ];
        
        for input in malicious_inputs {
            let result = validate_input(input);
            assert!(result.is_err(), "Failed to block: {}", input);
        }
    }
    
    #[test]
    fn test_xss_prevention() {
        let xss_attempts = vec![
            "<script>alert('XSS')</script>",
            "<img src=x onerror=alert('XSS')>",
            "<svg onload=alert('XSS')>",
            "javascript:alert('XSS')",
        ];
        
        for input in xss_attempts {
            let sanitized = sanitize_html(input);
            assert!(!sanitized.contains("<script"), "Failed to sanitize: {}", input);
            assert!(!sanitized.contains("onerror"), "Failed to sanitize: {}", input);
            assert!(!sanitized.contains("javascript:"), "Failed to sanitize: {}", input);
        }
    }
    
    #[tokio::test]
    async fn test_rate_limiting() {
        let limiter = create_test_rate_limiter();
        let client_id = "test_client";
        
        // Should allow initial requests
        for _ in 0..10 {
            assert!(limiter.check_rate_limit(client_id).await.is_ok());
        }
        
        // Should block after limit
        assert!(limiter.check_rate_limit(client_id).await.is_err());
    }
    
    #[test]
    fn test_password_hashing() {
        let password = "SecurePassword123!";
        let hash = hash_password(password).unwrap();
        
        // Hash should be different each time (due to salt)
        let hash2 = hash_password(password).unwrap();
        assert_ne!(hash, hash2);
        
        // But verification should work
        assert!(verify_password(password, &hash).is_ok());
        assert!(verify_password(password, &hash2).is_ok());
        
        // Wrong password should fail
        assert!(verify_password("WrongPassword", &hash).is_err());
    }
}
```

## Incident Response

### Security Incident Handling

Implement incident response procedures:

```rust
#[derive(Debug, Clone)]
pub struct SecurityIncident {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub severity: IncidentSeverity,
    pub category: IncidentCategory,
    pub description: String,
    pub affected_resources: Vec<String>,
    pub source_ip: Option<IpAddr>,
    pub user_id: Option<Uuid>,
    pub actions_taken: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum IncidentSeverity {
    Critical,  // Immediate response required
    High,      // Response within 1 hour
    Medium,    // Response within 24 hours
    Low,       // Track and analyze
}

#[derive(Debug, Clone)]
pub enum IncidentCategory {
    BruteForce,
    DataBreach,
    Malware,
    UnauthorizedAccess,
    DenialOfService,
    PolicyViolation,
}

pub struct IncidentResponder {
    notifier: Arc<dyn IncidentNotifier>,
    logger: Arc<AuditLogger>,
}

impl IncidentResponder {
    pub async fn handle_incident(&self, incident: SecurityIncident) {
        // Log the incident
        self.logger.log_incident(&incident).await;
        
        // Take immediate action based on severity
        match incident.severity {
            IncidentSeverity::Critical => {
                // Block source IP
                if let Some(ip) = incident.source_ip {
                    self.block_ip(ip).await;
                }
                
                // Disable affected accounts
                if let Some(user_id) = incident.user_id {
                    self.disable_user(user_id).await;
                }
                
                // Notify security team immediately
                self.notifier.send_critical_alert(&incident).await;
            }
            IncidentSeverity::High => {
                // Rate limit source
                if let Some(ip) = incident.source_ip {
                    self.rate_limit_ip(ip).await;
                }
                
                // Notify security team
                self.notifier.send_high_priority_alert(&incident).await;
            }
            _ => {
                // Log for analysis
                self.notifier.send_notification(&incident).await;
            }
        }
    }
}
```

## Security Checklist

### Pre-Deployment Security Checklist

- [ ] **Authentication & Authorization**
  - [ ] All endpoints require authentication (except public ones)
  - [ ] Role-based access control implemented
  - [ ] Session management secure
  - [ ] Password policy enforced

- [ ] **Input Validation**
  - [ ] All inputs validated at API boundary
  - [ ] SQL injection prevention
  - [ ] XSS prevention
  - [ ] File upload restrictions

- [ ] **API Security**
  - [ ] Rate limiting enabled
  - [ ] Query depth/complexity limits set
  - [ ] CORS properly configured
  - [ ] API versioning implemented

- [ ] **Data Security**
  - [ ] Sensitive data encrypted at rest
  - [ ] PII properly masked in logs
  - [ ] Secure data transmission (HTTPS)
  - [ ] Data retention policies implemented

- [ ] **Infrastructure Security**
  - [ ] Security headers configured
  - [ ] TLS 1.3 minimum
  - [ ] Secrets management system used
  - [ ] Regular security updates

- [ ] **Monitoring & Logging**
  - [ ] Security events logged
  - [ ] Audit trail complete
  - [ ] Anomaly detection configured
  - [ ] Incident response plan tested

## Compliance Requirements

### GDPR Compliance

```rust
/// GDPR compliance helpers
pub mod gdpr {
    /// Right to erasure (Right to be forgotten)
    pub async fn delete_user_data(user_id: Uuid) -> Result<()> {
        // Delete from primary database
        db.delete_user(user_id).await?;
        
        // Delete from backups (mark for deletion)
        backup_service.mark_for_deletion(user_id).await?;
        
        // Delete from logs (anonymize)
        log_service.anonymize_user_logs(user_id).await?;
        
        // Delete from analytics
        analytics_service.delete_user_data(user_id).await?;
        
        // Audit the deletion
        audit_logger.log_data_deletion(user_id).await?;
        
        Ok(())
    }
    
    /// Right to data portability
    pub async fn export_user_data(user_id: Uuid) -> Result<UserDataExport> {
        let user_data = db.get_user(user_id).await?;
        let notes = db.get_user_notes(user_id).await?;
        let activities = db.get_user_activities(user_id).await?;
        
        Ok(UserDataExport {
            user: user_data,
            notes,
            activities,
            exported_at: Utc::now(),
            format: ExportFormat::Json,
        })
    }
}
```

## Summary

These security standards provide comprehensive guidelines for building and maintaining a secure PCF API. Key principles include:

1. **Defense in Depth**: Multiple layers of security controls
2. **Least Privilege**: Minimal necessary permissions
3. **Zero Trust**: Verify everything, trust nothing
4. **Fail Secure**: Default to secure state on failure

Regular security audits, penetration testing, and keeping up with security best practices are essential for maintaining a secure system. Security is not a one-time implementation but an ongoing process that requires constant vigilance and improvement.