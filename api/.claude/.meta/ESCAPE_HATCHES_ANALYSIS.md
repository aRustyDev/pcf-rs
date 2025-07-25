# Escape Hatches Analysis and Remediation

This document identifies all ambiguities, escape hatches, and weak language in the PCF-RS API specifications that could allow an agent to avoid proper implementation.

## Critical Issues Requiring Immediate Fix

### 1. Weak Modal Verbs
**Problem**: Use of "should" instead of "MUST" throughout specs
**Impact**: Allows implementation to be optional
**Files Affected**:
- SPEC.md: Lines 4, 73, 120
- All .spec files use "should" intermittently

**Fix**: Replace all instances with RFC 2119 keywords:
- MUST / MUST NOT - absolute requirements
- SHOULD / SHOULD NOT - strong recommendations with valid exceptions documented
- MAY - truly optional features

### 2. Undefined Terms
**Problem**: Vague terms without concrete definitions
**Examples**:
- "appropriate" (graphql-schema.md:66)
- "careful" (SPEC.md:210)
- "high-frequency" (logging.md:162)
- "non-critical" (SPEC.md:120)

**Fix**: Create glossary section with exact definitions:
```markdown
## Definitions
- High-frequency: Any operation exceeding 100 requests/second
- Critical failure: Database unreachable, auth service down, or data corruption
- Non-critical failure: Cache miss, metrics endpoint timeout, or log write failure
- Appropriate derives: Must include exactly: Debug, Clone, Serialize, Deserialize, SimpleObject, Validate
```

### 3. Missing Concrete Numbers
**Problem**: Performance and limits without specific values
**Examples**:
- "Careful cardinality control" → "Maximum 1000 unique label combinations"
- "Maximum limits" → Specific numbers for each limit
- "Reasonable timeout" → Exact timeout in milliseconds

**Fix**: Add requirements table:
```markdown
## Concrete Limits and Thresholds
| Requirement | Value | Enforcement |
|------------|-------|-------------|
| Max query depth | 15 | MUST reject with 400 error |
| Max query complexity | 1000 | MUST calculate and reject |
| Cardinality per metric | 1000 | MUST use bucketing beyond this |
| Connection pool size | 100 | MUST NOT exceed |
| Request timeout | 30s | MUST cancel and return 504 |
| GraphQL timeout | 25s | MUST cancel execution |
| Database timeout | 20s | MUST cancel query |
| Cache TTL | 5 min | MUST expire exactly at |
| Retry attempts | 3 | MUST stop after 3rd failure |
| Backoff base | 1s | MUST use exponential: 1s, 2s, 4s |
```

### 4. Incomplete Error Specifications
**Problem**: Error handling with "..." or partial examples
**Examples**:
- health-checks.md:106: "// ... error handling"
- Many "handle appropriately" comments

**Fix**: Provide complete error handling code:
```rust
// MUST implement exactly this error handling pattern:
match check_spicedb().await {
    Ok(_) => ServiceStatus::Healthy,
    Err(e) if e.is_timeout() => ServiceStatus::Degraded,
    Err(e) if e.is_connection() => ServiceStatus::Unhealthy,
    Err(e) => {
        error!("SpiceDB check failed: {}", e);
        ServiceStatus::Unhealthy
    }
}
```

### 5. Optional Features as Escape Hatches
**Problem**: Features that can be disabled or skipped
**Examples**:
- Demo mode can bypass auth (authorization.md:54)
- Tests can be ignored (testing-strategy.md:867)
- Introspection "can be" disabled

**Fix**: Add production guards:
```rust
// MUST include these compile-time checks:
#[cfg(all(not(debug_assertions), feature = "demo"))]
compile_error!("Demo mode MUST NOT be enabled in release builds");

#[cfg(not(debug_assertions))]
const _: () = assert!(!cfg!(feature = "introspection"), 
    "Introspection MUST be disabled in production");
```

### 6. Open Decisions Blocking Implementation
**Problem**: ROADMAP.md leaves critical decisions open
**Example**: "Decision needed on restoration vs fresh start"

**Fix**: Remove all decision points:
- "Implementation MUST be fresh, following this roadmap exactly"
- "Archive is for reference only and MUST NOT be restored"

### 7. Conditional Implementation
**Problem**: Using #[cfg] to make requirements optional
**Examples**:
- Authorization bypass in demo mode
- Different behavior in test vs production

**Fix**: Requirements for all build configurations:
```markdown
## Build Configuration Requirements
1. Demo mode MUST:
   - Log every bypassed authorization with user ID "demo"
   - Add "X-Demo-Mode: true" header to all responses
   - Reject any production data access

2. Production builds MUST:
   - Fail compilation if demo feature enabled
   - Fail compilation if test-utils feature enabled
   - Include runtime check: panic!("Demo mode in production") if detected
```

### 8. Missing Validation Requirements
**Problem**: No verification that requirements are met
**Example**: "80% coverage" without enforcement

**Fix**: Add CI/CD requirements:
```yaml
# MUST include in CI pipeline:
- name: Coverage Check
  run: |
    cargo tarpaulin --out Xml
    coverage=$(cat cobertura.xml | grep -oP 'line-rate="\K[^"]+')
    if (( $(echo "$coverage < 0.85" | bc -l) )); then
      echo "Coverage $coverage is below required 85%"
      exit 1
    fi

- name: Critical Path Coverage
  run: |
    # MUST have 100% coverage for:
    # - src/auth/authorization.rs
    # - src/health/checks.rs
    # - src/database/retry.rs
```

### 9. Escape Clauses in Language
**Problem**: Phrases that allow avoiding requirements
**Examples**:
- "if possible"
- "when appropriate"
- "consider using"
- "recommended approach"

**Fix**: Remove all conditional language:
- Change "consider using DataLoader" → "MUST use DataLoader for all relational queries"
- Change "recommended retry strategy" → "MUST implement exponential backoff with exact parameters"

### 10. Future Enhancements as Excuses
**Problem**: "Future enhancements" sections suggest current implementation is acceptable without them
**Example**: authorization.md has future enhancements section

**Fix**: Move all future items to separate file:
- Current specs = MUST implement now
- Future specs = Version 2.0 requirements (separate file)

## Implementation Verification Checklist

Every implementation MUST pass these checks:

```bash
#!/bin/bash
# MUST run this verification script before marking any phase complete

# 1. No panics in code
if grep -r "unwrap()" src/ | grep -v "test"; then
  echo "FAIL: unwrap() found in non-test code"
  exit 1
fi

# 2. All MUST requirements implemented
if grep -r "TODO" src/ | grep -i "must"; then
  echo "FAIL: Unimplemented MUST requirement found"
  exit 1
fi

# 3. Security checks
if grep -r "introspection.*true" config/prod*; then
  echo "FAIL: Introspection enabled in production config"
  exit 1
fi

# 4. All errors handled
if grep -r "Error>>" src/ | grep -v "impl"; then
  echo "FAIL: Unhandled error type found"
  exit 1
fi

echo "PASS: All requirements verified"
```

## Summary of Required Changes

1. **Replace all "should" with "MUST"** in every specification file
2. **Define all vague terms** in a glossary section
3. **Add concrete numbers** for all limits and thresholds
4. **Provide complete code examples** without "..." or placeholders
5. **Remove optional features** or add strict guards
6. **Close all open decisions** in ROADMAP.md
7. **Add build configuration checks** to prevent requirement bypass
8. **Implement automated verification** in CI/CD
9. **Remove conditional language** throughout specs
10. **Separate future enhancements** from current requirements

## Next Steps

1. Update all specification files to address these issues
2. Add automated checks to verify specification compliance
3. Create a specification validation tool that agents must run
4. Require sign-off checklist for each phase completion

These changes will ensure that any agent implementing the system has no escape hatches and must deliver a complete, secure, production-ready implementation.