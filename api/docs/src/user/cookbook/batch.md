# Batch Operations Cookbook

Practical guide to performing efficient batch operations with the PCF API, including bulk creates, updates, deletes, and imports.

<!-- toc -->

## Overview

Batch operations allow you to perform multiple API operations in a single request, significantly improving performance and reducing network overhead. The PCF API supports various batch operation patterns for different use cases.

## Benefits of Batch Operations

- **Performance**: Reduce network round trips
- **Atomicity**: All-or-nothing execution options
- **Efficiency**: Lower server processing overhead
- **Consistency**: Maintain data integrity
- **Cost**: Reduced API call quotas

## REST Batch Operations

### Basic Batch Request

```bash
# Batch multiple operations
curl -X POST https://api.pcf.example.com/v1/batch \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "operations": [
      {
        "id": "op1",
        "method": "POST",
        "path": "/resources",
        "body": {
          "name": "Resource 1",
          "type": "document"
        }
      },
      {
        "id": "op2",
        "method": "POST",
        "path": "/resources",
        "body": {
          "name": "Resource 2",
          "type": "image"
        }
      },
      {
        "id": "op3",
        "method": "PATCH",
        "path": "/resources/existing_123",
        "body": {
          "status": "archived"
        }
      }
    ],
    "options": {
      "atomic": true,
      "continue_on_error": false
    }
  }'
```

### Batch Response

```json
{
  "results": [
    {
      "id": "op1",
      "status": 201,
      "body": {
        "data": {
          "id": "res_456",
          "name": "Resource 1",
          "type": "document",
          "created_at": "2024-01-15T10:00:00Z"
        }
      }
    },
    {
      "id": "op2",
      "status": 201,
      "body": {
        "data": {
          "id": "res_457",
          "name": "Resource 2",
          "type": "image",
          "created_at": "2024-01-15T10:00:01Z"
        }
      }
    },
    {
      "id": "op3",
      "status": 200,
      "body": {
        "data": {
          "id": "existing_123",
          "status": "archived",
          "updated_at": "2024-01-15T10:00:02Z"
        }
      }
    }
  ],
  "meta": {
    "total_operations": 3,
    "successful": 3,
    "failed": 0,
    "duration_ms": 150
  }
}
```

### Batch Options

```typescript
interface BatchOptions {
  // Execute all operations in a transaction
  atomic?: boolean;
  
  // Continue processing after errors
  continue_on_error?: boolean;
  
  // Maximum parallel operations
  parallel?: number;
  
  // Timeout for entire batch
  timeout_ms?: number;
  
  // Return only errors
  suppress_success?: boolean;
}
```

## GraphQL Batch Operations

### Multiple Mutations

```graphql
mutation BatchCreateResources {
  r1: createResource(input: {
    name: "Resource 1"
    type: DOCUMENT
    metadata: { key: "value1" }
  }) {
    resource {
      id
      name
    }
  }
  
  r2: createResource(input: {
    name: "Resource 2"
    type: IMAGE
    metadata: { key: "value2" }
  }) {
    resource {
      id
      name
    }
  }
  
  r3: updateResource(id: "existing_123", input: {
    status: ARCHIVED
  }) {
    resource {
      id
      status
    }
  }
}
```

### Batch Query

```graphql
query BatchGetResources {
  res1: resource(id: "res_123") {
    ...ResourceFields
  }
  
  res2: resource(id: "res_456") {
    ...ResourceFields
  }
  
  res3: resource(id: "res_789") {
    ...ResourceFields
  }
}

fragment ResourceFields on Resource {
  id
  name
  type
  status
  metadata
  createdAt
  updatedAt
}
```

### DataLoader Pattern

```javascript
import DataLoader from 'dataloader';

// Batch loading function
const batchLoadResources = async (ids) => {
  const query = `
    query BatchLoadResources($ids: [ID!]!) {
      resources(ids: $ids) {
        id
        name
        type
        status
      }
    }
  `;
  
  const { data } = await graphqlClient.request(query, { ids });
  
  // Map results back to requested order
  const resourceMap = new Map(data.resources.map(r => [r.id, r]));
  return ids.map(id => resourceMap.get(id) || null);
};

// Create DataLoader instance
const resourceLoader = new DataLoader(batchLoadResources, {
  maxBatchSize: 100,
  cache: true,
});

// Usage - these will be batched
const [res1, res2, res3] = await Promise.all([
  resourceLoader.load('res_123'),
  resourceLoader.load('res_456'),
  resourceLoader.load('res_789'),
]);
```

## Bulk Import

### CSV Import

```javascript
// Prepare CSV file
const csvContent = `
name,type,status,metadata
"Resource 1",document,active,"{""key1"":""value1""}"
"Resource 2",image,active,"{""key2"":""value2""}"
"Resource 3",video,draft,"{""key3"":""value3""}"
`;

// Upload and import
const formData = new FormData();
formData.append('file', new Blob([csvContent], { type: 'text/csv' }), 'resources.csv');
formData.append('options', JSON.stringify({
  mapping: {
    name: 'name',
    type: 'type',
    status: 'status',
    metadata: { type: 'json', field: 'metadata' }
  },
  validation: 'strict',
  dry_run: false,
  chunk_size: 1000
}));

const response = await fetch('https://api.pcf.example.com/v1/import/resources', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
  },
  body: formData,
});

const result = await response.json();
// {
//   "job_id": "import_job_123",
//   "status": "processing",
//   "total_rows": 3,
//   "processed": 0
// }
```

### JSON Bulk Import

```javascript
const bulkData = {
  resources: [
    { name: "Resource 1", type: "document", metadata: { key1: "value1" } },
    { name: "Resource 2", type: "image", metadata: { key2: "value2" } },
    { name: "Resource 3", type: "video", metadata: { key3: "value3" } },
    // ... up to 10,000 items
  ],
  options: {
    upsert: true,
    upsert_key: "name",
    validate_before_import: true,
    notification_webhook: "https://webhook.example.com/import-complete"
  }
};

const response = await fetch('https://api.pcf.example.com/v1/bulk/resources', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json',
  },
  body: JSON.stringify(bulkData),
});
```

## Batch Update Patterns

### Update Multiple by IDs

```javascript
// Update specific resources
const batchUpdate = {
  operations: [
    {
      method: "PATCH",
      path: "/resources/res_123",
      body: { status: "published" }
    },
    {
      method: "PATCH",
      path: "/resources/res_456",
      body: { status: "published" }
    },
    {
      method: "PATCH",
      path: "/resources/res_789",
      body: { status: "published" }
    }
  ]
};
```

### Update by Query

```javascript
// Update all matching resources
const bulkUpdate = await fetch('https://api.pcf.example.com/v1/bulk-update/resources', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    filter: {
      status: "draft",
      created_before: "2024-01-01T00:00:00Z"
    },
    update: {
      status: "archived",
      metadata: {
        archived_at: new Date().toISOString(),
        archived_by: "system"
      }
    },
    options: {
      dry_run: false,
      limit: 1000
    }
  }),
});

// Response
// {
//   "updated_count": 247,
//   "duration_ms": 1234,
//   "details": {
//     "successful": 247,
//     "failed": 0,
//     "skipped": 0
//   }
// }
```

## Batch Delete Operations

### Delete Multiple Resources

```javascript
// Delete by IDs
const batchDelete = await fetch('https://api.pcf.example.com/v1/batch', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    operations: [
      { method: "DELETE", path: "/resources/res_123" },
      { method: "DELETE", path: "/resources/res_456" },
      { method: "DELETE", path: "/resources/res_789" }
    ],
    options: {
      atomic: false,  // Continue even if some fail
      soft_delete: true  // Mark as deleted instead of removing
    }
  }),
});
```

### Bulk Delete by Filter

```graphql
mutation BulkDeleteResources($filter: ResourceFilter!) {
  bulkDeleteResources(filter: $filter) {
    deletedCount
    deletedIds
    errors {
      id
      message
    }
  }
}

# Variables
{
  "filter": {
    "status": "archived",
    "updatedBefore": "2023-01-01T00:00:00Z"
  }
}
```

## Streaming Operations

### Server-Sent Events for Progress

```javascript
// Monitor bulk operation progress
const eventSource = new EventSource(
  `https://api.pcf.example.com/v1/jobs/import_job_123/progress`,
  {
    headers: {
      'Authorization': `Bearer ${token}`,
    },
  }
);

eventSource.addEventListener('progress', (event) => {
  const data = JSON.parse(event.data);
  console.log(`Progress: ${data.processed}/${data.total} (${data.percentage}%)`);
});

eventSource.addEventListener('complete', (event) => {
  const data = JSON.parse(event.data);
  console.log('Import complete:', data);
  eventSource.close();
});

eventSource.addEventListener('error', (event) => {
  console.error('Import error:', event);
  eventSource.close();
});
```

### Chunked Processing

```javascript
// Process large dataset in chunks
async function* processInChunks(items, chunkSize = 100) {
  for (let i = 0; i < items.length; i += chunkSize) {
    const chunk = items.slice(i, i + chunkSize);
    const result = await processBatch(chunk);
    yield {
      processed: i + chunk.length,
      total: items.length,
      results: result
    };
  }
}

// Usage
const items = Array.from({ length: 10000 }, (_, i) => ({ id: i, data: '...' }));

for await (const progress of processInChunks(items, 500)) {
  console.log(`Processed ${progress.processed}/${progress.total}`);
  updateProgressBar(progress.processed / progress.total);
}
```

## Error Handling

### Partial Failures

```javascript
// Handle mixed results
const handleBatchResponse = (response) => {
  const { results } = response;
  
  const successful = results.filter(r => r.status >= 200 && r.status < 300);
  const failed = results.filter(r => r.status >= 400);
  
  if (failed.length > 0) {
    console.error('Failed operations:', failed);
    
    // Retry failed operations
    const retryOperations = failed.map(f => ({
      id: `retry_${f.id}`,
      ...f.originalOperation
    }));
    
    return retryBatch(retryOperations);
  }
  
  return successful;
};
```

### Validation Errors

```json
{
  "results": [
    {
      "id": "op1",
      "status": 400,
      "error": {
        "code": "VALIDATION_ERROR",
        "message": "Validation failed",
        "details": [
          {
            "field": "name",
            "message": "Name is required"
          },
          {
            "field": "type",
            "message": "Invalid type value"
          }
        ]
      }
    }
  ]
}
```

## Performance Optimization

### Parallel Processing

```javascript
// Process batches in parallel with concurrency limit
import pLimit from 'p-limit';

const limit = pLimit(5); // Max 5 concurrent requests

const processBatches = async (batches) => {
  const promises = batches.map((batch, index) => 
    limit(() => 
      processBatch(batch)
        .then(result => ({ index, result, success: true }))
        .catch(error => ({ index, error, success: false }))
    )
  );
  
  return Promise.all(promises);
};
```

### Request Compression

```javascript
// Enable gzip compression for large batches
const compressedRequest = await fetch('https://api.pcf.example.com/v1/batch', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json',
    'Content-Encoding': 'gzip',
    'Accept-Encoding': 'gzip',
  },
  body: await gzipCompress(JSON.stringify(largeBatchData)),
});
```

### Batch Size Optimization

```javascript
// Dynamic batch sizing based on performance
class AdaptiveBatcher {
  constructor(initialSize = 100) {
    this.batchSize = initialSize;
    this.minSize = 10;
    this.maxSize = 1000;
    this.targetDuration = 1000; // 1 second target
  }
  
  async processBatch(items) {
    const start = Date.now();
    const batch = items.slice(0, this.batchSize);
    
    const result = await sendBatch(batch);
    const duration = Date.now() - start;
    
    // Adjust batch size based on performance
    if (duration < this.targetDuration * 0.8) {
      this.batchSize = Math.min(this.maxSize, Math.floor(this.batchSize * 1.2));
    } else if (duration > this.targetDuration * 1.2) {
      this.batchSize = Math.max(this.minSize, Math.floor(this.batchSize * 0.8));
    }
    
    return {
      result,
      nextBatchSize: this.batchSize,
      duration
    };
  }
}
```

## Monitoring Batch Operations

### Job Status Endpoint

```javascript
// Check batch job status
const checkJobStatus = async (jobId) => {
  const response = await fetch(`https://api.pcf.example.com/v1/jobs/${jobId}`, {
    headers: {
      'Authorization': `Bearer ${token}`,
    },
  });
  
  return response.json();
  // {
  //   "id": "job_123",
  //   "type": "bulk_import",
  //   "status": "processing",
  //   "progress": {
  //     "total": 10000,
  //     "processed": 7500,
  //     "successful": 7450,
  //     "failed": 50,
  //     "percentage": 75
  //   },
  //   "started_at": "2024-01-15T10:00:00Z",
  //   "estimated_completion": "2024-01-15T10:05:00Z"
  // }
};
```

### Webhook Notifications

```javascript
// Configure webhook for batch completion
const startBatchWithWebhook = async (operations) => {
  return fetch('https://api.pcf.example.com/v1/batch', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      operations,
      options: {
        webhook: {
          url: 'https://webhook.example.com/batch-complete',
          events: ['completed', 'failed'],
          headers: {
            'X-Webhook-Secret': 'secret_key'
          }
        }
      }
    }),
  });
};

// Webhook payload
// {
//   "event": "batch.completed",
//   "job_id": "job_123",
//   "summary": {
//     "total_operations": 1000,
//     "successful": 995,
//     "failed": 5,
//     "duration_ms": 5432
//   },
//   "timestamp": "2024-01-15T10:05:32Z"
// }
```

## Best Practices

### 1. Batch Size Guidelines

| Operation Type | Recommended Size | Maximum Size |
|----------------|-----------------|---------------|
| Create | 100-500 | 1,000 |
| Update | 100-500 | 1,000 |
| Delete | 500-1,000 | 5,000 |
| Query | 50-100 | 500 |

### 2. Error Recovery

```javascript
// Implement exponential backoff for retries
const retryWithBackoff = async (operation, maxRetries = 3) => {
  let lastError;
  
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await operation();
    } catch (error) {
      lastError = error;
      
      if (i < maxRetries - 1) {
        const delay = Math.min(1000 * Math.pow(2, i), 10000);
        await new Promise(resolve => setTimeout(resolve, delay));
      }
    }
  }
  
  throw lastError;
};
```

### 3. Progress Tracking

```javascript
// Track and report progress
class BatchProgressTracker {
  constructor(total, onProgress) {
    this.total = total;
    this.processed = 0;
    this.successful = 0;
    this.failed = 0;
    this.onProgress = onProgress;
    this.startTime = Date.now();
  }
  
  update(successful, failed) {
    this.successful += successful;
    this.failed += failed;
    this.processed = this.successful + this.failed;
    
    const progress = {
      processed: this.processed,
      total: this.total,
      successful: this.successful,
      failed: this.failed,
      percentage: (this.processed / this.total) * 100,
      estimatedTimeRemaining: this.estimateTimeRemaining(),
    };
    
    this.onProgress(progress);
  }
  
  estimateTimeRemaining() {
    const elapsed = Date.now() - this.startTime;
    const rate = this.processed / elapsed;
    const remaining = this.total - this.processed;
    return remaining / rate;
  }
}
```

## Summary

Key batch operation guidelines:
1. Choose appropriate batch sizes
2. Handle partial failures gracefully
3. Implement progress monitoring
4. Use compression for large payloads
5. Consider atomic transactions when needed
6. Implement retry logic with backoff
7. Monitor performance and adjust
8. Use webhooks for async notifications
9. Validate data before bulk operations
10. Test with production-like data volumes
