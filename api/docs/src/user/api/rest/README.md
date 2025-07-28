# REST API Endpoints

Comprehensive reference for all REST API endpoints provided by the PCF API, including authentication, request/response formats, and examples.

<!-- toc -->

## Overview

The PCF API provides a RESTful interface alongside GraphQL for clients that prefer traditional REST patterns. All endpoints follow REST conventions and return JSON responses.

## Base URL

```
Production: https://api.pcf.example.com/v1
Staging:    https://api-staging.pcf.example.com/v1
Local:      http://localhost:8080/v1
```

## Authentication

All API requests require authentication using Bearer tokens:

```bash
curl -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  https://api.pcf.example.com/v1/user/me
```

## Common Headers

```http
Authorization: Bearer <token>
Content-Type: application/json
Accept: application/json
X-Request-ID: <unique-request-id>
X-Client-Version: 1.0.0
```

## Response Format

### Success Response

```json
{
  "data": {
    "id": "123",
    "type": "user",
    "attributes": {
      "name": "John Doe",
      "email": "john@example.com"
    }
  },
  "meta": {
    "request_id": "req_123abc",
    "timestamp": "2024-01-15T10:30:00Z"
  }
}
```

### Error Response

```json
{
  "errors": [
    {
      "id": "err_123",
      "status": "400",
      "code": "INVALID_INPUT",
      "title": "Invalid Request",
      "detail": "The email field is required",
      "source": {
        "pointer": "/data/attributes/email"
      }
    }
  ],
  "meta": {
    "request_id": "req_123abc",
    "timestamp": "2024-01-15T10:30:00Z"
  }
}
```

## Authentication Endpoints

### POST /auth/login

Authenticate a user and receive access tokens.

**Request:**
```json
{
  "email": "user@example.com",
  "password": "secure_password"
}
```

**Response:**
```json
{
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIs...",
    "refresh_token": "eyJhbGciOiJIUzI1NiIs...",
    "token_type": "Bearer",
    "expires_in": 3600,
    "user": {
      "id": "user_123",
      "email": "user@example.com",
      "name": "John Doe"
    }
  }
}
```

### POST /auth/refresh

Refresh an access token using a refresh token.

**Request:**
```json
{
  "refresh_token": "eyJhbGciOiJIUzI1NiIs..."
}
```

**Response:**
```json
{
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIs...",
    "token_type": "Bearer",
    "expires_in": 3600
  }
}
```

### POST /auth/logout

Invalidate the current access token.

**Request:**
```bash
curl -X POST https://api.pcf.example.com/v1/auth/logout \
  -H "Authorization: Bearer YOUR_TOKEN"
```

**Response:**
```json
{
  "data": {
    "message": "Successfully logged out"
  }
}
```

### POST /auth/register

Register a new user account.

**Request:**
```json
{
  "email": "newuser@example.com",
  "password": "secure_password",
  "name": "Jane Doe",
  "organization": "ACME Corp"
}
```

**Response:**
```json
{
  "data": {
    "id": "user_456",
    "email": "newuser@example.com",
    "name": "Jane Doe",
    "created_at": "2024-01-15T10:30:00Z"
  }
}
```

## User Endpoints

### GET /user/me

Get the authenticated user's profile.

**Response:**
```json
{
  "data": {
    "id": "user_123",
    "type": "user",
    "attributes": {
      "email": "user@example.com",
      "name": "John Doe",
      "organization": "ACME Corp",
      "role": "admin",
      "created_at": "2023-01-01T00:00:00Z",
      "updated_at": "2024-01-15T10:30:00Z"
    }
  }
}
```

### PATCH /user/me

Update the authenticated user's profile.

**Request:**
```json
{
  "name": "John Smith",
  "organization": "New Corp"
}
```

**Response:**
```json
{
  "data": {
    "id": "user_123",
    "type": "user",
    "attributes": {
      "email": "user@example.com",
      "name": "John Smith",
      "organization": "New Corp",
      "updated_at": "2024-01-15T10:35:00Z"
    }
  }
}
```

### DELETE /user/me

Delete the authenticated user's account.

**Response:**
```json
{
  "data": {
    "message": "Account successfully deleted"
  }
}
```

### POST /user/me/change-password

Change the authenticated user's password.

**Request:**
```json
{
  "current_password": "old_password",
  "new_password": "new_secure_password"
}
```

**Response:**
```json
{
  "data": {
    "message": "Password successfully changed"
  }
}
```

## Resource Endpoints

### GET /resources

List all resources with pagination and filtering.

**Query Parameters:**
- `page[number]` - Page number (default: 1)
- `page[size]` - Items per page (default: 20, max: 100)
- `filter[status]` - Filter by status (active, inactive, pending)
- `filter[created_after]` - Filter by creation date
- `sort` - Sort field (`created_at`, `-created_at`, `name`, `-name`)

**Example:**
```bash
curl "https://api.pcf.example.com/v1/resources?page[number]=2&page[size]=50&filter[status]=active&sort=-created_at" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

**Response:**
```json
{
  "data": [
    {
      "id": "res_123",
      "type": "resource",
      "attributes": {
        "name": "Resource One",
        "status": "active",
        "created_at": "2024-01-15T10:00:00Z"
      }
    },
    {
      "id": "res_124",
      "type": "resource",
      "attributes": {
        "name": "Resource Two",
        "status": "active",
        "created_at": "2024-01-14T15:00:00Z"
      }
    }
  ],
  "meta": {
    "pagination": {
      "current_page": 2,
      "per_page": 50,
      "total_pages": 10,
      "total_count": 487
    }
  },
  "links": {
    "self": "https://api.pcf.example.com/v1/resources?page[number]=2&page[size]=50",
    "first": "https://api.pcf.example.com/v1/resources?page[number]=1&page[size]=50",
    "prev": "https://api.pcf.example.com/v1/resources?page[number]=1&page[size]=50",
    "next": "https://api.pcf.example.com/v1/resources?page[number]=3&page[size]=50",
    "last": "https://api.pcf.example.com/v1/resources?page[number]=10&page[size]=50"
  }
}
```

### GET /resources/:id

Get a specific resource by ID.

**Response:**
```json
{
  "data": {
    "id": "res_123",
    "type": "resource",
    "attributes": {
      "name": "Resource One",
      "description": "Detailed description of the resource",
      "status": "active",
      "metadata": {
        "key1": "value1",
        "key2": "value2"
      },
      "created_at": "2024-01-15T10:00:00Z",
      "updated_at": "2024-01-15T12:00:00Z"
    },
    "relationships": {
      "owner": {
        "data": {
          "id": "user_123",
          "type": "user"
        }
      },
      "tags": {
        "data": [
          { "id": "tag_1", "type": "tag" },
          { "id": "tag_2", "type": "tag" }
        ]
      }
    }
  }
}
```

### POST /resources

Create a new resource.

**Request:**
```json
{
  "name": "New Resource",
  "description": "Description of the new resource",
  "metadata": {
    "key1": "value1",
    "key2": "value2"
  },
  "tags": ["tag1", "tag2"]
}
```

**Response:**
```json
{
  "data": {
    "id": "res_125",
    "type": "resource",
    "attributes": {
      "name": "New Resource",
      "description": "Description of the new resource",
      "status": "active",
      "metadata": {
        "key1": "value1",
        "key2": "value2"
      },
      "created_at": "2024-01-15T13:00:00Z"
    }
  }
}
```

### PATCH /resources/:id

Update a resource.

**Request:**
```json
{
  "name": "Updated Resource Name",
  "metadata": {
    "key1": "new_value1",
    "key3": "value3"
  }
}
```

**Response:**
```json
{
  "data": {
    "id": "res_123",
    "type": "resource",
    "attributes": {
      "name": "Updated Resource Name",
      "metadata": {
        "key1": "new_value1",
        "key2": "value2",
        "key3": "value3"
      },
      "updated_at": "2024-01-15T14:00:00Z"
    }
  }
}
```

### DELETE /resources/:id

Delete a resource.

**Response:**
```json
{
  "data": {
    "message": "Resource successfully deleted"
  }
}
```

## Batch Operations

### POST /batch

Perform multiple operations in a single request.

**Request:**
```json
{
  "operations": [
    {
      "method": "POST",
      "path": "/resources",
      "body": {
        "name": "Resource 1"
      }
    },
    {
      "method": "PATCH",
      "path": "/resources/res_123",
      "body": {
        "status": "inactive"
      }
    },
    {
      "method": "DELETE",
      "path": "/resources/res_124"
    }
  ]
}
```

**Response:**
```json
{
  "data": {
    "results": [
      {
        "status": 201,
        "body": {
          "data": {
            "id": "res_126",
            "type": "resource",
            "attributes": {
              "name": "Resource 1"
            }
          }
        }
      },
      {
        "status": 200,
        "body": {
          "data": {
            "id": "res_123",
            "type": "resource",
            "attributes": {
              "status": "inactive"
            }
          }
        }
      },
      {
        "status": 204,
        "body": null
      }
    ]
  }
}
```

## Search Endpoints

### POST /search

Perform a full-text search across resources.

**Request:**
```json
{
  "query": "search term",
  "filters": {
    "type": ["resource", "document"],
    "status": "active",
    "created_after": "2024-01-01T00:00:00Z"
  },
  "facets": ["type", "status", "tags"],
  "page": {
    "number": 1,
    "size": 20
  }
}
```

**Response:**
```json
{
  "data": {
    "results": [
      {
        "id": "res_123",
        "type": "resource",
        "score": 0.95,
        "highlight": {
          "name": "<em>search term</em> in title",
          "description": "Content with <em>search term</em>"
        },
        "attributes": {
          "name": "search term in title",
          "status": "active"
        }
      }
    ],
    "facets": {
      "type": [
        { "value": "resource", "count": 45 },
        { "value": "document", "count": 23 }
      ],
      "status": [
        { "value": "active", "count": 50 },
        { "value": "inactive", "count": 18 }
      ]
    },
    "meta": {
      "total_count": 68,
      "query_time_ms": 45
    }
  }
}
```

## File Upload

### POST /upload

Upload files using multipart/form-data.

**Request:**
```bash
curl -X POST https://api.pcf.example.com/v1/upload \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -F "file=@document.pdf" \
  -F "metadata={\"description\":\"Important document\"}"
```

**Response:**
```json
{
  "data": {
    "id": "file_789",
    "type": "file",
    "attributes": {
      "filename": "document.pdf",
      "size": 1048576,
      "mime_type": "application/pdf",
      "url": "https://storage.pcf.example.com/files/file_789",
      "metadata": {
        "description": "Important document"
      },
      "created_at": "2024-01-15T15:00:00Z"
    }
  }
}
```

### GET /files/:id

Retrieve file metadata.

**Response:**
```json
{
  "data": {
    "id": "file_789",
    "type": "file",
    "attributes": {
      "filename": "document.pdf",
      "size": 1048576,
      "mime_type": "application/pdf",
      "url": "https://storage.pcf.example.com/files/file_789",
      "expires_at": "2024-01-15T16:00:00Z"
    }
  }
}
```

## Webhooks

### GET /webhooks

List configured webhooks.

**Response:**
```json
{
  "data": [
    {
      "id": "webhook_123",
      "type": "webhook",
      "attributes": {
        "url": "https://example.com/webhook",
        "events": ["resource.created", "resource.updated"],
        "active": true,
        "secret": "whsec_...",
        "created_at": "2024-01-10T00:00:00Z"
      }
    }
  ]
}
```

### POST /webhooks

Create a new webhook.

**Request:**
```json
{
  "url": "https://example.com/webhook",
  "events": ["resource.created", "resource.updated", "resource.deleted"],
  "active": true
}
```

**Response:**
```json
{
  "data": {
    "id": "webhook_124",
    "type": "webhook",
    "attributes": {
      "url": "https://example.com/webhook",
      "events": ["resource.created", "resource.updated", "resource.deleted"],
      "active": true,
      "secret": "whsec_1234567890abcdef",
      "created_at": "2024-01-15T16:00:00Z"
    }
  }
}
```

## Health & Status

### GET /health

Check API health status.

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T16:00:00Z",
  "version": "1.0.0",
  "uptime_seconds": 86400
}
```

### GET /health/ready

Check if the API is ready to handle requests.

**Response:**
```json
{
  "status": "ready",
  "checks": {
    "database": {
      "status": "healthy",
      "response_time_ms": 5
    },
    "cache": {
      "status": "healthy",
      "response_time_ms": 1
    },
    "external_api": {
      "status": "healthy",
      "response_time_ms": 150
    }
  }
}
```

## Rate Limiting

All endpoints are subject to rate limiting. Rate limit information is included in response headers:

```http
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1705329600
X-RateLimit-Reset-After: 3600
```

When rate limited, the API returns:

```json
{
  "errors": [
    {
      "status": "429",
      "code": "RATE_LIMITED",
      "title": "Too Many Requests",
      "detail": "Rate limit exceeded. Retry after 3600 seconds.",
      "meta": {
        "retry_after": 3600
      }
    }
  ]
}
```

## Pagination

List endpoints support pagination using the `page` parameter:

```bash
# Page-based pagination
GET /resources?page[number]=2&page[size]=50

# Cursor-based pagination (for large datasets)
GET /resources?page[cursor]=eyJpZCI6MTIzfQ&page[size]=50
```

Pagination metadata is included in the response:

```json
{
  "meta": {
    "pagination": {
      "current_page": 2,
      "per_page": 50,
      "total_pages": 10,
      "total_count": 487,
      "has_next_page": true,
      "has_prev_page": true
    }
  }
}
```

## Filtering & Sorting

### Filtering

Use the `filter` parameter to filter results:

```bash
# Single filter
GET /resources?filter[status]=active

# Multiple filters
GET /resources?filter[status]=active&filter[type]=document

# Date range filters
GET /resources?filter[created_after]=2024-01-01&filter[created_before]=2024-01-31

# Array filters
GET /resources?filter[tags][]=tag1&filter[tags][]=tag2
```

### Sorting

Use the `sort` parameter to sort results:

```bash
# Sort ascending
GET /resources?sort=created_at

# Sort descending
GET /resources?sort=-created_at

# Multiple sort fields
GET /resources?sort=-created_at,name
```

## Field Selection

Use sparse fieldsets to request only specific fields:

```bash
# Request only specific fields
GET /resources?fields[resource]=id,name,status

# Include related resources with specific fields
GET /resources?include=owner&fields[resource]=id,name&fields[user]=id,email
```

## Including Related Resources

Use the `include` parameter to include related resources:

```bash
# Include single relationship
GET /resources/123?include=owner

# Include multiple relationships
GET /resources/123?include=owner,tags,comments

# Include nested relationships
GET /resources/123?include=comments.author
```

## Error Codes

| HTTP Status | Error Code | Description |
|-------------|------------|-------------|
| 400 | `INVALID_REQUEST` | The request is malformed |
| 401 | `UNAUTHORIZED` | Authentication required |
| 403 | `FORBIDDEN` | Access denied |
| 404 | `NOT_FOUND` | Resource not found |
| 409 | `CONFLICT` | Resource conflict |
| 422 | `VALIDATION_ERROR` | Validation failed |
| 429 | `RATE_LIMITED` | Too many requests |
| 500 | `INTERNAL_ERROR` | Server error |
| 503 | `SERVICE_UNAVAILABLE` | Service temporarily unavailable |

## SDK Examples

### JavaScript/TypeScript

```typescript
import { PCFClient } from '@pcf/sdk';

const client = new PCFClient({
  apiKey: 'YOUR_API_KEY',
  baseURL: 'https://api.pcf.example.com/v1'
});

// Get resources
const resources = await client.resources.list({
  page: { number: 1, size: 20 },
  filter: { status: 'active' },
  sort: '-created_at'
});

// Create resource
const newResource = await client.resources.create({
  name: 'New Resource',
  description: 'Description'
});
```

### Python

```python
from pcf_sdk import PCFClient

client = PCFClient(
    api_key='YOUR_API_KEY',
    base_url='https://api.pcf.example.com/v1'
)

# Get resources
resources = client.resources.list(
    page={'number': 1, 'size': 20},
    filter={'status': 'active'},
    sort='-created_at'
)

# Create resource
new_resource = client.resources.create(
    name='New Resource',
    description='Description'
)
```

### Go

```go
import "github.com/pcf/sdk-go"

client := pcf.NewClient(
    pcf.WithAPIKey("YOUR_API_KEY"),
    pcf.WithBaseURL("https://api.pcf.example.com/v1"),
)

// Get resources
resources, err := client.Resources.List(ctx, &pcf.ListOptions{
    Page: &pcf.PageOptions{Number: 1, Size: 20},
    Filter: map[string]string{"status": "active"},
    Sort: "-created_at",
})

// Create resource
newResource, err := client.Resources.Create(ctx, &pcf.Resource{
    Name:        "New Resource",
    Description: "Description",
})
```

## Best Practices

1. **Use HTTPS** - Always use HTTPS in production
2. **Include Request IDs** - Add `X-Request-ID` header for tracing
3. **Handle Rate Limits** - Implement exponential backoff
4. **Paginate Large Results** - Use pagination for large datasets
5. **Cache Responses** - Cache GET requests when appropriate
6. **Validate Input** - Validate data client-side before sending
7. **Use Compression** - Enable gzip compression with `Accept-Encoding: gzip`
8. **Version Your Client** - Include `X-Client-Version` header
9. **Handle Errors Gracefully** - Parse error responses properly
10. **Use Field Selection** - Request only needed fields to reduce payload size
