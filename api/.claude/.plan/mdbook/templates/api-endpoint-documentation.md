# API Endpoint Documentation Template

# [Endpoint Name]

[Brief description of what this endpoint does]

## Endpoint Information

| | |
|---|---|
| **URL** | `/api/v1/[endpoint]` |
| **Method** | `POST` \| `GET` \| `PUT` \| `DELETE` |
| **Auth Required** | Yes \| No |
| **Permissions** | `resource:action` |
| **Rate Limit** | 100 requests/minute |
| **Idempotent** | Yes \| No |

## Quick Example

### Request
```bash
curl -X POST https://api.example.com/api/v1/[endpoint] \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "field": "value"
  }'
```

### Response
```json
{
  "success": true,
  "data": {
    "id": "123",
    "field": "value"
  }
}
```

## Request

### Headers

| Header | Type | Required | Description |
|--------|------|----------|-------------|
| `Authorization` | string | Yes | Bearer token for authentication |
| `Content-Type` | string | Yes | Must be `application/json` |
| `X-Request-ID` | string | No | Optional request tracking ID |

### URL Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| `id` | string | Yes | Resource identifier | `123e4567-e89b-12d3-a456-426614174000` |

### Query Parameters

| Parameter | Type | Required | Description | Default | Example |
|-----------|------|----------|-------------|---------|---------|
| `page` | integer | No | Page number for pagination | 1 | `?page=2` |
| `limit` | integer | No | Items per page | 20 | `?limit=50` |
| `sort` | string | No | Sort field and direction | `created_at:desc` | `?sort=name:asc` |

### Request Body

```typescript
interface RequestBody {
  // Required fields
  field1: string;      // Description of field1
  field2: number;      // Description of field2
  
  // Optional fields
  field3?: boolean;    // Description of field3
  field4?: {           // Nested object
    subfield1: string;
    subfield2: number;
  };
}
```

#### Field Validations

- `field1`: 
  - Min length: 3
  - Max length: 255
  - Pattern: `/^[a-zA-Z0-9-_]+$/`
- `field2`:
  - Min value: 0
  - Max value: 1000
- `field4.subfield2`:
  - Must be positive integer

### Example Request Bodies

<details>
<summary>Minimal Request</summary>

```json
{
  "field1": "example",
  "field2": 42
}
```

</details>

<details>
<summary>Full Request</summary>

```json
{
  "field1": "example",
  "field2": 42,
  "field3": true,
  "field4": {
    "subfield1": "nested",
    "subfield2": 10
  }
}
```

</details>

## Response

### Success Response

**Status Code:** `200 OK` | `201 Created` | `204 No Content`

```typescript
interface SuccessResponse {
  success: true;
  data: {
    id: string;
    field1: string;
    field2: number;
    // ... other fields
    created_at: string;  // ISO 8601
    updated_at: string;  // ISO 8601
  };
  meta?: {
    pagination?: {
      page: number;
      limit: number;
      total: number;
      total_pages: number;
    };
  };
}
```

### Error Responses

**Status Code:** `400 Bad Request`

```json
{
  "success": false,
  "error": {
    "code": "INVALID_INPUT",
    "message": "Validation failed",
    "trace_id": "550e8400-e29b-41d4-a716-446655440000",
    "timestamp": "2024-01-01T00:00:00Z",
    "details": {
      "field1": ["Must be at least 3 characters long"],
      "field2": ["Must be a positive number"]
    }
  }
}
```

**Status Code:** `401 Unauthorized`

```json
{
  "success": false,
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Invalid or expired token",
    "trace_id": "550e8400-e29b-41d4-a716-446655440000",
    "timestamp": "2024-01-01T00:00:00Z"
  }
}
```

**Status Code:** `403 Forbidden`

```json
{
  "success": false,
  "error": {
    "code": "FORBIDDEN",
    "message": "Insufficient permissions",
    "trace_id": "550e8400-e29b-41d4-a716-446655440000",
    "timestamp": "2024-01-01T00:00:00Z",
    "required_permission": "resource:write"
  }
}
```

**Status Code:** `404 Not Found`

```json
{
  "success": false,
  "error": {
    "code": "NOT_FOUND",
    "message": "Resource not found",
    "trace_id": "550e8400-e29b-41d4-a716-446655440000",
    "timestamp": "2024-01-01T00:00:00Z"
  }
}
```

**Status Code:** `429 Too Many Requests`

```json
{
  "success": false,
  "error": {
    "code": "RATE_LIMITED",
    "message": "Rate limit exceeded",
    "trace_id": "550e8400-e29b-41d4-a716-446655440000",
    "timestamp": "2024-01-01T00:00:00Z",
    "retry_after": 60
  }
}
```

**Status Code:** `500 Internal Server Error`

```json
{
  "success": false,
  "error": {
    "code": "INTERNAL_ERROR",
    "message": "An unexpected error occurred",
    "trace_id": "550e8400-e29b-41d4-a716-446655440000",
    "timestamp": "2024-01-01T00:00:00Z"
  }
}
```

## Error Codes

| Code | HTTP Status | Description | Recovery |
|------|-------------|-------------|----------|
| `INVALID_INPUT` | 400 | Request validation failed | Fix validation errors |
| `UNAUTHORIZED` | 401 | Missing or invalid auth | Provide valid token |
| `FORBIDDEN` | 403 | Lacks required permission | Check user permissions |
| `NOT_FOUND` | 404 | Resource doesn't exist | Verify resource ID |
| `CONFLICT` | 409 | Resource already exists | Use different identifier |
| `RATE_LIMITED` | 429 | Too many requests | Wait and retry |
| `INTERNAL_ERROR` | 500 | Server error | Retry with backoff |

## Rate Limiting

- **Window**: 1 minute sliding window
- **Limit**: 100 requests per minute
- **Headers**:
  - `X-RateLimit-Limit`: Maximum requests allowed
  - `X-RateLimit-Remaining`: Requests remaining
  - `X-RateLimit-Reset`: Unix timestamp when limit resets

## Pagination

For endpoints returning lists:

### Request
```
GET /api/v1/[endpoint]?page=2&limit=20
```

### Response
```json
{
  "success": true,
  "data": [...],
  "meta": {
    "pagination": {
      "page": 2,
      "limit": 20,
      "total": 453,
      "total_pages": 23,
      "has_previous": true,
      "has_next": true
    }
  },
  "links": {
    "first": "/api/v1/[endpoint]?page=1&limit=20",
    "previous": "/api/v1/[endpoint]?page=1&limit=20",
    "next": "/api/v1/[endpoint]?page=3&limit=20",
    "last": "/api/v1/[endpoint]?page=23&limit=20"
  }
}
```

## Filtering & Sorting

### Filtering
```
GET /api/v1/[endpoint]?filter[status]=active&filter[type]=premium
```

### Sorting
```
GET /api/v1/[endpoint]?sort=-created_at,name
```
- Prefix with `-` for descending order
- Multiple fields separated by comma

## Webhooks

If this endpoint triggers webhooks:

### Webhook Payload
```json
{
  "event": "resource.created",
  "timestamp": "2024-01-01T00:00:00Z",
  "data": {
    // Same as API response data
  }
}
```

### Webhook Events
- `resource.created`
- `resource.updated`
- `resource.deleted`

## Code Examples

### JavaScript/TypeScript
```typescript
const response = await fetch('https://api.example.com/api/v1/[endpoint]', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    field1: 'value',
    field2: 42
  })
});

const data = await response.json();
```

### Python
```python
import requests

response = requests.post(
    'https://api.example.com/api/v1/[endpoint]',
    headers={
        'Authorization': f'Bearer {token}',
        'Content-Type': 'application/json',
    },
    json={
        'field1': 'value',
        'field2': 42
    }
)

data = response.json()
```

### Go
```go
client := &http.Client{}
payload, _ := json.Marshal(map[string]interface{}{
    "field1": "value",
    "field2": 42,
})

req, _ := http.NewRequest("POST", "https://api.example.com/api/v1/[endpoint]", bytes.NewBuffer(payload))
req.Header.Set("Authorization", "Bearer " + token)
req.Header.Set("Content-Type", "application/json")

resp, _ := client.Do(req)
```

## Try It Out

<div class="api-playground" 
     data-endpoint="/api/v1/[endpoint]" 
     data-method="POST"
     data-auth="true">
  <button class="try-api">Open in API Playground</button>
</div>

## Notes & Best Practices

- Always include idempotency keys for non-idempotent operations
- Use pagination for large result sets
- Implement exponential backoff for retries
- Cache responses where appropriate
- Monitor rate limit headers

## Common Issues

### "Invalid JSON"
Ensure Content-Type header is set and body is valid JSON

### "Rate limit exceeded"
Implement rate limit handling and backoff

### "Unauthorized"
Check token expiration and refresh if needed

## Related Endpoints

- [`GET /api/v1/related`](./related.md) - Fetch related resources
- [`PUT /api/v1/[endpoint]/{id}`](./update.md) - Update resource
- [`DELETE /api/v1/[endpoint]/{id}`](./delete.md) - Delete resource

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.1.0 | 2024-01-15 | Added filtering support |
| 1.0.0 | 2024-01-01 | Initial release |

---
*See also: [Authentication](../authentication.md) | [Rate Limiting](../rate-limiting.md) | [Error Handling](../errors.md)*