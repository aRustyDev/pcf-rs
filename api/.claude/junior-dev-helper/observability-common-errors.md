# Common Observability Errors Guide

## Metrics Errors

### Error: "Metrics endpoint returns empty response"

**Symptom:**
```bash
curl http://localhost:9090/metrics
# Returns: empty or 404
```

**Causes and Solutions:**

1. **Recorder not initialized**
   ```rust
   // BAD - Handle dropped immediately
   pub fn init_metrics() {
       let builder = PrometheusBuilder::new();
       builder.install_recorder(); // Handle dropped!
   }
   
   // GOOD - Return and store handle
   pub fn init_metrics() -> PrometheusHandle {
       let builder = PrometheusBuilder::new();
       builder.install_recorder().unwrap()
   }
   
   // In main.rs
   let metrics_handle = init_metrics();
   // Store in app state to prevent drop
   ```

2. **Wrong endpoint path**
   ```rust
   // Check your route definition
   .route("/metrics", get(metrics_handler)) // Not "/prometheus"
   ```

3. **Metrics not being recorded**
   ```rust
   // Add debug logging
   counter!("test_metric").increment(1);
   println!("Recorded test metric"); // Verify this runs
   ```

### Error: "Prometheus rejects metrics - invalid format"

**Symptom:**
```bash
curl http://localhost:9090/metrics | promtool check metrics
# Error: expected label name, got "INVALID"
```

**Causes and Solutions:**

1. **Invalid metric names**
   ```rust
   // BAD - Invalid characters
   counter!("my-metric-name"); // Hyphens not allowed!
   counter!("metric.with.dots"); // Dots not allowed!
   
   // GOOD - Use underscores
   counter!("my_metric_name");
   counter!("metric_with_underscores");
   ```

2. **Invalid label names**
   ```rust
   // BAD
   counter!("requests", "status-code" => "200"); // Hyphen!
   
   // GOOD
   counter!("requests", "status_code" => "200");
   ```

### Error: "Cardinality explosion - Prometheus OOM"

**Symptom:**
- Prometheus memory usage growing rapidly
- Queries becoming slow
- Alert: "Too many series"

**Causes and Solutions:**

1. **Unbounded labels**
   ```rust
   // BAD - Unique ID as label
   counter!("user_actions", 
       "user_id" => user_id,  // Could be millions!
       "session_id" => session_id  // Always unique!
   );
   
   // GOOD - Use bounded categories
   counter!("user_actions",
       "user_type" => user.account_type(), // "free", "premium"
       "action" => action.category()  // Limited set
   );
   ```

2. **Not implementing cardinality limits**
   ```rust
   // Implement limiter
   static OPERATION_NAMES: Lazy<Mutex<HashSet<String>>> = 
       Lazy::new(|| Mutex::new(HashSet::new()));
   
   fn get_operation_label(name: &str) -> &'static str {
       let mut names = OPERATION_NAMES.lock().unwrap();
       
       if names.contains(name) {
           return name;
       }
       
       if names.len() < 50 {
           names.insert(name.to_string());
           // Leak to get 'static lifetime
           Box::leak(name.to_string().into_boxed_str())
       } else {
           "other"
       }
   }
   ```

## Logging Errors

### Error: "Logs show as plain text instead of JSON"

**Symptom:**
```
2024-01-15 10:30:45 INFO my_app: Starting server
```
Instead of:
```json
{"timestamp":"2024-01-15T10:30:45Z","level":"INFO","message":"Starting server"}
```

**Causes and Solutions:**

1. **Wrong formatter configuration**
   ```rust
   // Check if production flag is set correctly
   let is_production = std::env::var("ENVIRONMENT")
       .map(|e| e == "production")
       .unwrap_or(false);
   
   if is_production {
       tracing_subscriber::fmt()
           .json()  // This is important!
           .init();
   } else {
       tracing_subscriber::fmt()
           .pretty()
           .init();
   }
   ```

2. **Multiple subscribers initialized**
   ```rust
   // BAD - Initializing twice
   tracing_subscriber::fmt().init();
   tracing_subscriber::fmt().json().init(); // Error!
   
   // GOOD - Initialize once
   static INIT: std::sync::Once = std::sync::Once::new();
   INIT.call_once(|| {
       tracing_subscriber::fmt().json().init();
   });
   ```

### Error: "Sensitive data appearing in logs"

**Symptom:**
```json
{"level":"INFO","message":"User login","email":"user@example.com","password":"secret123"}
```

**Causes and Solutions:**

1. **Not using skip attribute**
   ```rust
   // BAD
   #[instrument]
   fn login(email: &str, password: &str) {
       info!("Login attempt");
   }
   
   // GOOD - Skip sensitive parameters
   #[instrument(skip(password))]
   fn login(email: &str, password: &str) {
       info!("Login attempt");
   }
   ```

2. **Logging full structs**
   ```rust
   // BAD
   #[derive(Debug)]
   struct User {
       id: String,
       email: String,
       password_hash: String,
   }
   
   info!("User data: {:?}", user); // Logs everything!
   
   // GOOD - Implement custom Debug
   impl fmt::Debug for User {
       fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
           f.debug_struct("User")
               .field("id", &self.id)
               .field("email", &"<REDACTED>")
               .finish()
       }
   }
   ```

### Error: "No trace_id in logs"

**Symptom:**
Logs missing trace correlation:
```json
{"level":"INFO","message":"Processing request"}
```

**Causes and Solutions:**

1. **Not creating spans**
   ```rust
   // BAD - No span
   async fn handle_request() {
       info!("Processing request");
   }
   
   // GOOD - Create span with trace_id
   #[instrument(fields(trace_id = %Uuid::new_v4()))]
   async fn handle_request() {
       info!("Processing request");
   }
   ```

2. **Span not entered**
   ```rust
   // BAD - Span created but not entered
   let span = info_span!("operation");
   info!("This log has no trace_id");
   
   // GOOD - Enter the span
   let span = info_span!("operation");
   let _enter = span.enter();
   info!("This log has trace_id");
   ```

## Tracing Errors

### Error: "Spans not appearing in Jaeger/Backend"

**Symptom:**
- Application runs but no traces in UI
- No errors in logs

**Causes and Solutions:**

1. **Wrong OTLP endpoint**
   ```rust
   // Check the endpoint
   println!("OTLP endpoint: {}", config.otlp_endpoint);
   
   // Common endpoints:
   // - Jaeger: "http://localhost:4317"
   // - Local collector: "http://localhost:4318/v1/traces"
   // - OTEL collector: "grpc://localhost:4317"
   ```

2. **Sampling set to 0**
   ```rust
   // BAD - Nothing sampled
   .with_sampler(Sampler::TraceIdRatioBased(0.0))
   
   // GOOD - Sample 10%
   .with_sampler(Sampler::TraceIdRatioBased(0.1))
   
   // For debugging - sample everything
   .with_sampler(Sampler::AlwaysOn)
   ```

3. **Spans not being exported**
   ```rust
   // Force flush on shutdown
   fn shutdown_tracing() {
       opentelemetry::global::shutdown_tracer_provider();
   }
   
   // Or manually flush
   if let Some(provider) = 
       opentelemetry::global::tracer_provider()
           .as_any()
           .downcast_ref::<TracerProvider>() 
   {
       let _ = provider.force_flush();
   }
   ```

### Error: "Broken span hierarchy - no parent/child relationships"

**Symptom:**
All spans appear at the same level, no nesting

**Causes and Solutions:**

1. **Not propagating context**
   ```rust
   // BAD - New root span every time
   async fn operation_a() {
       let span = info_span!("op_a");
       operation_b().await;
   }
   
   // GOOD - Propagate context
   async fn operation_a() {
       let span = info_span!("op_a");
       operation_b().instrument(span.clone()).await;
   }
   ```

2. **Spawning tasks without context**
   ```rust
   // BAD - Loses trace context
   tokio::spawn(async {
       do_work().await;
   });
   
   // GOOD - Preserve context
   let span = Span::current();
   tokio::spawn(
       async move {
           do_work().await;
       }
       .instrument(span)
   );
   ```

### Error: "Trace context not propagating between services"

**Symptom:**
Each service creates new trace instead of continuing

**Causes and Solutions:**

1. **Not injecting headers**
   ```rust
   // BAD - No trace headers
   client.get(url).send().await
   
   // GOOD - Inject trace context
   let mut headers = HeaderMap::new();
   opentelemetry::global::get_text_map_propagator(|propagator| {
       propagator.inject_context(
           &Span::current().context(),
           &mut HeaderInjector(&mut headers)
       );
   });
   
   client.get(url).headers(headers).send().await
   ```

2. **Not extracting headers**
   ```rust
   // In receiving service
   let parent_context = opentelemetry::global::get_text_map_propagator(|prop| {
       prop.extract(&HeaderExtractor(req.headers()))
   });
   
   let span = info_span!("handle_request");
   span.set_parent(parent_context);
   ```

## Performance Issues

### Error: "High CPU usage from observability"

**Causes and Solutions:**

1. **Too many spans**
   ```rust
   // BAD - Span for every item
   for item in items {
       let span = info_span!("process_item");
       let _enter = span.enter();
       process(item);
   }
   
   // GOOD - One span for batch
   let span = info_span!("process_batch", count = items.len());
   let _enter = span.enter();
   for item in items {
       process(item);
   }
   ```

2. **Synchronous export**
   ```rust
   // BAD - Blocks on every span
   .install_simple()
   
   // GOOD - Batch export
   .install_batch(opentelemetry::runtime::Tokio)
   ```

### Error: "Memory leak in observability"

**Causes and Solutions:**

1. **Unbounded metric labels**
   ```rust
   // Monitor cardinality
   let metrics = prometheus_handle.gather();
   for family in metrics {
       if family.get_metric().len() > 10000 {
           warn!("High cardinality metric: {}", family.get_name());
       }
   }
   ```

2. **Span attributes too large**
   ```rust
   // BAD - Huge attribute
   span.set_attribute("response_body", &large_json);
   
   // GOOD - Limit size
   span.set_attribute(
       "response_preview",
       &large_json.chars().take(100).collect::<String>()
   );
   ```

## Debug Commands

### Check Metrics
```bash
# Count total metrics
curl -s localhost:9090/metrics | grep -v '^#' | wc -l

# Find high cardinality metrics
curl -s localhost:9090/metrics | grep -v '^#' | cut -d'{' -f1 | sort | uniq -c | sort -rn | head

# Validate format
curl -s localhost:9090/metrics | promtool check metrics
```

### Check Logs
```bash
# Verify JSON format
tail -f app.log | jq .

# Check for sensitive data
tail -1000 app.log | grep -E "(password|token|secret)"

# Count log levels
tail -1000 app.log | jq -r .level | sort | uniq -c
```

### Check Traces
```bash
# Test OTLP connection
grpcurl -plaintext localhost:4317 list

# Check Jaeger
curl http://localhost:16686/api/services

# Verify trace propagation
curl -H "traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01" \
     http://localhost:8080/test
```

## Prevention Checklist

Before deploying:
1. ✅ Test metrics endpoint manually
2. ✅ Verify no sensitive data in logs
3. ✅ Check cardinality limits work
4. ✅ Confirm traces appear in backend
5. ✅ Load test to check overhead
6. ✅ Set up alerts for high cardinality
7. ✅ Document metric/span names
8. ✅ Review sampling rates

Remember: It's better to have less telemetry that works than lots that doesn't!