# Authentication Guide

Comprehensive guide to authentication in the PCF API, covering JWT tokens, OAuth2 flows, API keys, and security best practices.

<!-- toc -->

## Overview

The PCF API uses JWT (JSON Web Tokens) for authentication, supporting multiple authentication methods including password-based login, OAuth2, and API keys. All API requests must include valid authentication credentials.

## Authentication Methods

### 1. JWT Token Authentication

The primary authentication method uses JWT tokens issued by the API after successful login.

#### Login Flow

```mermaid
sequenceDiagram
    participant Client
    participant API
    participant Database
    
    Client->>API: POST /auth/login
    Note right of Client: {email, password}
    API->>Database: Verify credentials
    Database-->>API: User data
    API->>API: Generate JWT tokens
    API-->>Client: {access_token, refresh_token}
    
    Client->>API: GET /resources
    Note right of Client: Authorization: Bearer <access_token>
    API->>API: Validate JWT
    API-->>Client: Protected resource
```

#### Token Structure

```json
// JWT Header
{
  "alg": "RS256",
  "typ": "JWT",
  "kid": "key_123"
}

// JWT Payload
{
  "sub": "user_123",
  "email": "user@example.com",
  "role": "admin",
  "org": "org_456",
  "iat": 1705325400,
  "exp": 1705329000,
  "nbf": 1705325400,
  "jti": "token_abc123"
}
```

#### Using Tokens

```bash
# Login
curl -X POST https://api.pcf.example.com/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "secure_password"
  }'

# Response
{
  "access_token": "eyJhbGciOiJSUzI1NiIs...",
  "refresh_token": "eyJhbGciOiJSUzI1NiIs...",
  "token_type": "Bearer",
  "expires_in": 3600
}

# Use token in requests
curl https://api.pcf.example.com/v1/resources \
  -H "Authorization: Bearer eyJhbGciOiJSUzI1NiIs..."
```

### 2. OAuth2 Authentication

The API supports OAuth2 for third-party integrations.

#### Authorization Code Flow

```mermaid
sequenceDiagram
    participant User
    participant Client
    participant API
    participant OAuth Provider
    
    User->>Client: Login with OAuth
    Client->>API: GET /auth/oauth/authorize
    API-->>Client: Redirect to OAuth Provider
    Client->>OAuth Provider: Authorization request
    User->>OAuth Provider: Grant permission
    OAuth Provider-->>Client: Authorization code
    Client->>API: POST /auth/oauth/token
    Note right of Client: {code, client_id, client_secret}
    API->>OAuth Provider: Validate code
    OAuth Provider-->>API: User info
    API->>API: Create/update user
    API-->>Client: {access_token, refresh_token}
```

#### Supported Providers

- Google
- GitHub
- Microsoft Azure AD
- Custom OIDC providers

#### Configuration

```javascript
// OAuth2 login request
const response = await fetch('https://api.pcf.example.com/v1/auth/oauth/google', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    code: 'authorization_code_from_google',
    redirect_uri: 'https://app.example.com/auth/callback',
  }),
});

const { access_token, refresh_token } = await response.json();
```

### 3. API Key Authentication

For machine-to-machine communication and CI/CD pipelines.

#### Creating API Keys

```bash
# Create an API key
curl -X POST https://api.pcf.example.com/v1/api-keys \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "CI/CD Pipeline",
    "scopes": ["read:resources", "write:resources"],
    "expires_at": "2025-01-01T00:00:00Z"
  }'

# Response
{
  "id": "key_789",
  "key": "pcf_api_key_abc123def456...",
  "name": "CI/CD Pipeline",
  "scopes": ["read:resources", "write:resources"],
  "created_at": "2024-01-15T10:00:00Z",
  "expires_at": "2025-01-01T00:00:00Z"
}
```

#### Using API Keys

```bash
# Use API key in header
curl https://api.pcf.example.com/v1/resources \
  -H "X-API-Key: pcf_api_key_abc123def456..."

# Or use as Bearer token
curl https://api.pcf.example.com/v1/resources \
  -H "Authorization: Bearer pcf_api_key_abc123def456..."
```

## Token Management

### Token Refresh

Access tokens expire after 1 hour. Use refresh tokens to obtain new access tokens without re-authentication.

```javascript
// Refresh token implementation
async function refreshAccessToken(refreshToken) {
  const response = await fetch('https://api.pcf.example.com/v1/auth/refresh', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      refresh_token: refreshToken,
    }),
  });

  if (!response.ok) {
    throw new Error('Failed to refresh token');
  }

  return response.json();
}

// Auto-refresh implementation
class APIClient {
  constructor(accessToken, refreshToken) {
    this.accessToken = accessToken;
    this.refreshToken = refreshToken;
    this.tokenExpiry = Date.now() + 3600 * 1000; // 1 hour
  }

  async request(url, options = {}) {
    // Check if token needs refresh
    if (Date.now() >= this.tokenExpiry - 60000) { // Refresh 1 minute before expiry
      const { access_token } = await refreshAccessToken(this.refreshToken);
      this.accessToken = access_token;
      this.tokenExpiry = Date.now() + 3600 * 1000;
    }

    // Make request with current token
    return fetch(url, {
      ...options,
      headers: {
        ...options.headers,
        'Authorization': `Bearer ${this.accessToken}`,
      },
    });
  }
}
```

### Token Revocation

```bash
# Revoke a specific token
curl -X POST https://api.pcf.example.com/v1/auth/revoke \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "token": "token_to_revoke",
    "token_type": "access_token"
  }'

# Revoke all tokens for current user
curl -X POST https://api.pcf.example.com/v1/auth/revoke-all \
  -H "Authorization: Bearer YOUR_TOKEN"
```

## Multi-Factor Authentication (MFA)

### Enabling MFA

```bash
# Request MFA setup
curl -X POST https://api.pcf.example.com/v1/auth/mfa/setup \
  -H "Authorization: Bearer YOUR_TOKEN"

# Response
{
  "secret": "JBSWY3DPEHPK3PXP",
  "qr_code": "data:image/png;base64,...",
  "backup_codes": [
    "12345678",
    "87654321",
    "11223344"
  ]
}
```

### Login with MFA

```javascript
// Step 1: Initial login
const loginResponse = await fetch('https://api.pcf.example.com/v1/auth/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'user@example.com',
    password: 'secure_password',
  }),
});

const { mfa_token } = await loginResponse.json();

// Step 2: Submit MFA code
const mfaResponse = await fetch('https://api.pcf.example.com/v1/auth/mfa/verify', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    mfa_token,
    code: '123456', // From authenticator app
  }),
});

const { access_token, refresh_token } = await mfaResponse.json();
```

## Session Management

### Session Configuration

```json
{
  "session": {
    "access_token_ttl": 3600,      // 1 hour
    "refresh_token_ttl": 2592000,  // 30 days
    "max_refresh_count": 10,       // Maximum token refreshes
    "idle_timeout": 1800,          // 30 minutes
    "absolute_timeout": 86400      // 24 hours
  }
}
```

### Active Sessions

```bash
# List active sessions
curl https://api.pcf.example.com/v1/auth/sessions \
  -H "Authorization: Bearer YOUR_TOKEN"

# Response
{
  "sessions": [
    {
      "id": "session_123",
      "device": "Chrome on macOS",
      "ip_address": "192.168.1.1",
      "location": "San Francisco, CA",
      "created_at": "2024-01-15T10:00:00Z",
      "last_active": "2024-01-15T15:30:00Z",
      "current": true
    }
  ]
}

# Terminate a session
curl -X DELETE https://api.pcf.example.com/v1/auth/sessions/session_123 \
  -H "Authorization: Bearer YOUR_TOKEN"
```

## Security Best Practices

### 1. Token Storage

```javascript
// ❌ Bad: Store in localStorage
localStorage.setItem('token', accessToken);

// ❌ Bad: Store in sessionStorage
sessionStorage.setItem('token', accessToken);

// ✅ Good: Store in memory or secure cookie
class TokenStore {
  constructor() {
    this.accessToken = null;
    this.refreshToken = null;
  }

  setTokens(accessToken, refreshToken) {
    this.accessToken = accessToken;
    // Store refresh token in httpOnly cookie
    document.cookie = `refresh_token=${refreshToken}; HttpOnly; Secure; SameSite=Strict`;
  }

  getAccessToken() {
    return this.accessToken;
  }
}
```

### 2. Request Security

```javascript
// Add security headers
const secureRequest = async (url, options = {}) => {
  const response = await fetch(url, {
    ...options,
    headers: {
      ...options.headers,
      'X-Request-ID': generateRequestId(),
      'X-Client-Version': '1.0.0',
    },
    credentials: 'include', // Include cookies
  });

  // Check for token expiration
  if (response.status === 401) {
    // Handle token refresh
    await refreshToken();
    return secureRequest(url, options);
  }

  return response;
};
```

### 3. CORS Configuration

```javascript
// Allowed origins
const allowedOrigins = [
  'https://app.pcf.example.com',
  'https://staging.pcf.example.com',
];

// CORS headers
app.use((req, res, next) => {
  const origin = req.headers.origin;
  if (allowedOrigins.includes(origin)) {
    res.setHeader('Access-Control-Allow-Origin', origin);
    res.setHeader('Access-Control-Allow-Credentials', 'true');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Authorization, Content-Type');
  }
  next();
});
```

### 4. Rate Limiting

Authentication endpoints have specific rate limits:

| Endpoint | Rate Limit | Window |
|----------|------------|--------|
| `/auth/login` | 5 requests | 15 minutes |
| `/auth/refresh` | 10 requests | 15 minutes |
| `/auth/register` | 3 requests | 1 hour |
| `/auth/mfa/verify` | 5 attempts | 15 minutes |

### 5. Security Headers

```bash
# Required security headers
Strict-Transport-Security: max-age=31536000; includeSubDomains
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Content-Security-Policy: default-src 'self'
```

## Error Handling

### Authentication Errors

```json
// Invalid credentials
{
  "error": "invalid_credentials",
  "error_description": "The email or password is incorrect",
  "status": 401
}

// Expired token
{
  "error": "token_expired",
  "error_description": "The access token has expired",
  "status": 401
}

// Invalid token
{
  "error": "invalid_token",
  "error_description": "The provided token is invalid",
  "status": 401
}

// Account locked
{
  "error": "account_locked",
  "error_description": "Account is locked due to too many failed attempts",
  "retry_after": 900,
  "status": 423
}
```

### Error Recovery

```javascript
class AuthenticationError extends Error {
  constructor(error, description, status) {
    super(description);
    this.name = 'AuthenticationError';
    this.error = error;
    this.status = status;
  }
}

async function handleAuthError(error) {
  switch (error.error) {
    case 'token_expired':
      // Attempt to refresh token
      return await refreshAccessToken();
      
    case 'invalid_token':
      // Clear stored tokens and redirect to login
      clearTokens();
      redirectToLogin();
      break;
      
    case 'account_locked':
      // Show account locked message
      showError(`Account locked. Try again in ${error.retry_after} seconds`);
      break;
      
    default:
      // Show generic error
      showError('Authentication failed. Please try again.');
  }
}
```

## SSO Integration

### SAML Configuration

```xml
<!-- SAML metadata -->
<EntityDescriptor entityID="https://api.pcf.example.com">
  <SPSSODescriptor>
    <AssertionConsumerService 
      Location="https://api.pcf.example.com/auth/saml/callback"
      Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"/>
  </SPSSODescriptor>
</EntityDescriptor>
```

### OpenID Connect

```javascript
// OIDC configuration
const oidcConfig = {
  issuer: 'https://auth.pcf.example.com',
  authorization_endpoint: 'https://auth.pcf.example.com/authorize',
  token_endpoint: 'https://auth.pcf.example.com/token',
  userinfo_endpoint: 'https://auth.pcf.example.com/userinfo',
  jwks_uri: 'https://auth.pcf.example.com/.well-known/jwks.json',
};
```

## Troubleshooting

### Common Issues

1. **Token not accepted**
   - Verify token hasn't expired
   - Check token format (Bearer prefix)
   - Ensure correct environment

2. **CORS errors**
   - Verify origin is whitelisted
   - Check preflight OPTIONS requests
   - Ensure credentials are included

3. **MFA issues**
   - Sync device time
   - Use backup codes if needed
   - Contact support for reset

### Debug Mode

```bash
# Enable auth debug logging
export PCF_API_AUTH_DEBUG=true

# View auth logs
tail -f /var/log/pcf-api/auth.log
```

## Summary

Key authentication points:
1. Use JWT tokens for stateless authentication
2. Implement token refresh for seamless UX
3. Enable MFA for enhanced security
4. Follow security best practices
5. Handle errors gracefully
6. Monitor authentication metrics
7. Regular security audits
