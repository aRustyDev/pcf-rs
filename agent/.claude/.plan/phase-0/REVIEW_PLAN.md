# Phase 1 Review Plan - Guidelines for Reviewing Agents

## Overview

This document provides comprehensive guidance for agents conducting reviews at Phase 1 checkpoints. As a reviewing agent, you are responsible for ensuring the implementation meets all specifications and is ready to proceed to the next stage.

## Your Responsibilities as Reviewer

1. **Thoroughly examine all provided artifacts**
2. **Test the implementation against specifications**
3. **Verify TDD practices were followed**
4. **Check code documentation and comments**
5. **Ensure code cleanliness (no stubs, TODOs, or test artifacts)**
6. **Provide explicit feedback on findings**
7. **Recommend specific changes if needed**
8. **Create an updated plan for completion if issues are found**
9. **Give clear approval or rejection**

## Core Review Principles

### Test-Driven Development (TDD) Verification
At every checkpoint, verify that TDD practices were followed:
1. **Tests exist before implementation** - Check git history or ask for evidence
2. **Tests fail first, then pass** - Red-Green-Refactor cycle
3. **Tests drive the design** - Implementation matches test requirements
4. **Tests are comprehensive** - Edge cases, error paths, happy paths

### Documentation Standards
All code must be well-documented:
1. **Rustdoc comments** on all public items
2. **Module documentation** explaining purpose and usage
3. **Inline comments** for complex logic (explaining "why" not "what")
4. **Example usage** where appropriate
5. **No outdated comments** from previous iterations

### Code Cleanliness Requirements
No development artifacts should remain:
1. **No TODOs or FIXMEs** - All must be resolved
2. **No debug prints** - No `println!`, `dbg!`, or `eprintln!`
3. **No commented code** - Delete it, don't comment it
4. **No test stubs** - All tests must be real
5. **No unused code** - Run `cargo clippy` and `cargo fix`
6. **Proper formatting** - Must pass `cargo fmt --check`

## Review Process

For each checkpoint review:

1. **Receive from implementing agent**:
   - Link to this REVIEW_PLAN.md
   - Link to WORK_PLAN.md
   - Specific checkpoint number
   - All artifacts listed for that checkpoint
   - Git commit history showing TDD approach

2. **Perform the review** using the checkpoint-specific checklist

3. **Verify the three pillars**:
   - TDD practices followed
   - Documentation complete
   - Code is clean

4. **Document your findings** in a structured format

5. **Provide clear decision**: APPROVED or CHANGES REQUIRED

## Checkpoint-Specific Review Guidelines

### 🛑 CHECKPOINT 1: Project Foundation Review

**What You're Reviewing**: Basic project structure and dependencies

**Key Specifications to Verify**:
- Rust edition 2024 is used
- All dependencies from the specifications are included
- No additional unnecessary dependencies
- Module structure matches the specification

**Required Tests**:
```bash
# Verify compilation using justfile
just build
just test

# Check for security vulnerabilities
cargo audit

# Verify dependency tree
cargo tree | grep -E "axum|tokio|figment|garde|tracing"
```

**Common Issues to Look For**:
- Version conflicts between dependencies
- Missing feature flags (e.g., `["macros"]` for axum)
- Incorrect module structure
- Missing .gitignore entries
- Lack of initial test structure
- Missing rustdoc module documentation

**Your Review Output Should Include**:
```markdown
## Checkpoint 1 Review Results

### Tested Compilation
- [ ] `just build`: [PASS/FAIL - include any errors]
- [ ] `just test`: [PASS/FAIL - include any errors]
- [ ] `cargo audit`: [PASS/FAIL - note any vulnerabilities]

### Dependency Review
- [ ] All required dependencies present: [YES/NO - list missing]
- [ ] Versions compatible: [YES/NO - note conflicts]
- [ ] No unnecessary dependencies: [YES/NO - list extras]

### Module Structure
- [ ] Follows specification: [YES/NO - note deviations]
- [ ] Naming conventions correct: [YES/NO - note issues]

### TDD Verification
- [ ] Tests were written before implementation: [YES/NO - evidence]
- [ ] All features have corresponding tests: [YES/NO - missing tests]
- [ ] Tests follow Arrange-Act-Assert pattern: [YES/NO - examples]
- [ ] Tests are isolated and independent: [YES/NO]

### Documentation Review
- [ ] All public items have rustdoc comments: [YES/NO - missing docs]
- [ ] Comments explain "why" not "what": [YES/NO - examples]
- [ ] Complex logic has explanatory comments: [YES/NO]
- [ ] No outdated or misleading comments: [YES/NO - found issues]

### Code Cleanliness
- [ ] No TODO/FIXME comments remain: [YES/NO - list any found]
- [ ] No debug prints or console logs: [YES/NO - locations]
- [ ] No commented-out code blocks: [YES/NO - locations]
- [ ] No test stubs or mock data in src/: [YES/NO]
- [ ] Consistent code formatting: [YES/NO - run `cargo fmt --check`]
- [ ] No compiler warnings: [YES/NO - run `cargo clippy`]

### Issues Found
1. [Issue description]
   - Impact: [HIGH/MEDIUM/LOW]
   - Required fix: [Specific action needed]

### Updated Plan for Completion
[If issues found, provide specific steps to fix them]

### Decision: [APPROVED / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 2: Core Infrastructure Review

**What You're Reviewing**: Error handling and configuration system

**Key Specifications to Verify**:
- Error types cover all specified categories
- Configuration uses 4-tier hierarchy
- Garde validation is comprehensive
- Demo mode compile-time check exists

**Required Tests**:
```bash
# Test configuration loading hierarchy
echo 'port = 9999' > config/test.toml
APP_SERVER__PORT=8888 cargo run -- --port 7777
# Should use port 7777 (CLI overrides all)

# Test validation
# Create invalid config and verify it fails with clear error
echo 'port = 99999' > config/invalid.toml

# Verify demo mode protection
cargo build --release --features demo
# Should fail with compile error
```

**Critical Code Reviews**:
```rust
// Verify panic handler exists in main.rs or lib.rs
std::panic::set_hook(Box::new(|panic_info| {
    // Should log FATAL and exit
}));

// Verify demo mode check exists
#[cfg(all(not(debug_assertions), feature = "demo"))]
compile_error!("Demo mode MUST NOT be enabled in release builds");

// Verify no .unwrap() or .expect() in error handling
// grep -r "unwrap()" src/ should only show test files
```

**Your Review Output Should Include**:
```markdown
## Checkpoint 2 Review Results

### Error Handling Review
- [ ] All error categories implemented: [YES/NO - list missing]
- [ ] Error messages safe (no internal details): [YES/NO - examples]
- [ ] IntoResponse implemented correctly: [YES/NO]
- [ ] Panic handler installed: [YES/NO]

### Configuration System Review
- [ ] 4-tier hierarchy works correctly: [YES/NO - test results]
- [ ] Validation catches invalid inputs: [YES/NO - test results]
- [ ] Environment variable override works: [YES/NO]
- [ ] CLI argument override works: [YES/NO]

### Security Checks
- [ ] Demo mode compile check present: [YES/NO]
- [ ] No .unwrap() in production code: [YES/NO - locations found]
- [ ] Secrets handling planned: [YES/NO]

### TDD Verification  
- [ ] Test modules created before implementation: [YES/NO]
- [ ] Error handling tests written first: [YES/NO]
- [ ] Config validation tests precede implementation: [YES/NO]
- [ ] Test structure follows standard patterns: [YES/NO]

### Documentation Review
- [ ] Error types have clear documentation: [YES/NO]
- [ ] Configuration fields are documented: [YES/NO]
- [ ] Module-level documentation exists: [YES/NO]
- [ ] Examples provided where helpful: [YES/NO]

### Code Cleanliness
- [ ] No placeholder implementations: [YES/NO]
- [ ] No temporary test code in src/: [YES/NO]
- [ ] All imports are used: [YES/NO - run `cargo fix`]
- [ ] Code passes `cargo fmt`: [YES/NO]

### Issues Found
[Detailed list with required fixes]

### Updated Plan for Completion
[Specific steps to address any issues]

### Decision: [APPROVED / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 3: Logging Infrastructure Review

**What You're Reviewing**: Logging, tracing, and security sanitization

**Key Specifications to Verify**:
- JSON logs in production, pretty in development
- Every log has trace_id
- Sensitive data is sanitized
- Async/non-blocking implementation

**Required Tests**:
```bash
# Test production logging
ENVIRONMENT=production cargo run 2>&1 | head -5 | jq .
# Should see JSON with timestamp, level, target, message, trace_id

# Test development logging
ENVIRONMENT=development cargo run 2>&1 | head -5
# Should see pretty formatted logs

# Test sensitive data sanitization
# Create a test that logs various sensitive patterns
cargo test logging::sanitization --nocapture
```

**Security Patterns to Verify Are Sanitized**:
- Email addresses (should show ***@domain.com or hash)
- Passwords (should never appear)
- API keys/tokens (should be [REDACTED])
- Credit card numbers (should be [REDACTED])
- IP addresses (should show subnet only)
- File paths with user directories

**Your Review Output Should Include**:
```markdown
## Checkpoint 3 Review Results

### Log Format Review
- [ ] Production logs are valid JSON: [YES/NO - sample]
- [ ] Development logs are human-readable: [YES/NO - sample]
- [ ] All required fields present: [YES/NO - missing fields]
- [ ] Trace IDs generated and included: [YES/NO - example]

### Security Sanitization
- [ ] Email addresses sanitized: [PASS/FAIL - test output]
- [ ] Passwords never logged: [PASS/FAIL - test output]
- [ ] Tokens/API keys redacted: [PASS/FAIL - test output]
- [ ] Credit cards redacted: [PASS/FAIL - test output]
- [ ] No PII in logs: [PASS/FAIL - examples]

### Performance
- [ ] Async logging implemented: [YES/NO]
- [ ] Non-blocking: [YES/NO - how verified]
- [ ] Buffer management present: [YES/NO]

### TDD Verification
- [ ] Sanitization tests written before implementation: [YES/NO]
- [ ] Tests cover all sensitive patterns: [YES/NO - missing patterns]
- [ ] Negative tests (ensuring things aren't logged): [YES/NO]
- [ ] Performance tests for async logging: [YES/NO]

### Documentation Review
- [ ] Logging configuration documented: [YES/NO]
- [ ] Sanitization patterns documented: [YES/NO]
- [ ] Environment-specific behavior explained: [YES/NO]
- [ ] Security considerations documented: [YES/NO]

### Code Cleanliness
- [ ] No test log outputs in production code: [YES/NO]
- [ ] No hardcoded test data: [YES/NO]
- [ ] Sanitizer patterns are maintainable: [YES/NO]
- [ ] No temporary debugging code: [YES/NO]

### Issues Found
[List with severity and required fixes]

### Updated Plan for Completion  
[Specific remediation steps]

### Decision: [APPROVED / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 4: Basic Server Review

**What You're Reviewing**: HTTP server, health endpoints, graceful shutdown

**Key Specifications to Verify**:
- Server binds to configured address/port
- Health endpoint returns "OK"
- Graceful shutdown within 30 seconds
- Clear error messages

**Required Tests**:
```bash
# Test 1: Basic startup and health check
cargo run &
SERVER_PID=$!
sleep 3
curl -v http://localhost:8080/health
# Should return 200 OK with body "OK"

# Test 2: Graceful shutdown
kill -TERM $SERVER_PID
# Should see shutdown logs and complete within 30s

# Test 3: Port conflict handling
cargo run &
sleep 2
cargo run
# Second instance should show clear port-in-use error

# Test 4: Configuration override
PORT=9090 cargo run &
curl http://localhost:9090/health
```

**Shutdown Behavior to Verify**:
- Stops accepting new connections immediately
- Completes in-flight requests
- Closes all resources cleanly
- Logs shutdown progress
- Completes within 30 seconds

**Your Review Output Should Include**:
```markdown
## Checkpoint 4 Review Results

### Server Startup
- [ ] Binds to configured port: [YES/NO - evidence]
- [ ] Startup logs clear and informative: [YES/NO - sample]
- [ ] Configuration override works: [YES/NO - test results]

### Health Endpoint
- [ ] Returns 200 OK: [YES/NO]
- [ ] Body is "OK": [YES/NO]
- [ ] No authentication required: [YES/NO]
- [ ] Responds within 1 second: [YES/NO - timing]

### Graceful Shutdown
- [ ] Handles SIGTERM correctly: [YES/NO]
- [ ] Drains in-flight requests: [YES/NO - how tested]
- [ ] Completes within 30 seconds: [YES/NO - actual time]
- [ ] Logs shutdown progress: [YES/NO - log samples]

### Error Handling
- [ ] Port conflict error is clear: [YES/NO - actual message]
- [ ] Includes suggestion for resolution: [YES/NO]
- [ ] No panics during startup/shutdown: [YES/NO]

### TDD Verification
- [ ] Integration tests for server lifecycle: [YES/NO]
- [ ] Tests for signal handling: [YES/NO]
- [ ] Graceful shutdown tests: [YES/NO]
- [ ] Port conflict tests: [YES/NO]

### Documentation Review  
- [ ] Main function documented: [YES/NO]
- [ ] Shutdown behavior documented: [YES/NO]
- [ ] Middleware stack explained: [YES/NO]
- [ ] Configuration options documented: [YES/NO]

### Code Cleanliness
- [ ] No test servers left running: [YES/NO]
- [ ] Clean separation of concerns: [YES/NO]
- [ ] No hardcoded ports/addresses: [YES/NO]
- [ ] Proper error handling throughout: [YES/NO]

### Issues Found
[Detailed findings with impact assessment]

### Updated Plan for Completion
[Specific fixes required]

### Decision: [APPROVED / CHANGES REQUIRED]
```

### 🛑 CHECKPOINT 5: Complete Phase 1 System Review

**What You're Reviewing**: Complete Phase 1 implementation

**Key Specifications to Verify**:
- All Phase 1 "Done Criteria" met
- Test coverage meets requirements
- Documentation is complete
- Production-ready code quality

**Required Comprehensive Tests**:
```bash
# Run the provided verification script
./scripts/verify-phase-1.sh
# Should complete successfully

# Check test coverage
cargo tarpaulin --out Html
# Open target/coverage/index.html
# Verify ≥80% overall, 100% on critical paths

# Security audit
cargo audit
cargo clippy -- -D warnings

# Documentation check
cargo doc --no-deps --open
# Verify all public items documented
```

**Code Quality Checks**:
```bash
# No unwrap/expect in production
grep -r "unwrap()\|expect(" src/ --exclude-dir=tests

# All TODOs addressed
grep -r "TODO\|FIXME" src/

# Error messages don't leak internals
# Review all error Display implementations
```

**Your Review Output Should Include**:
```markdown
## Phase 1 Complete System Review

### Done Criteria Verification
- [ ] Server starts successfully with Axum: [YES/NO]
- [ ] Configuration loads from all 4 tiers: [YES/NO - test evidence]
- [ ] Health check endpoints respond correctly: [YES/NO - test results]
- [ ] Graceful shutdown implemented: [YES/NO - test results]
- [ ] Structured logging with tracing operational: [YES/NO - examples]

### Test Coverage Analysis
- Overall Coverage: ___%
- Critical Paths:
  - [ ] Configuration validation: ___%
  - [ ] Error handling: ___%
  - [ ] Health state transitions: ___%
  - [ ] Panic recovery: ___%

### Code Quality
- [ ] No .unwrap() or .expect() in production: [PASS/FAIL - locations]
- [ ] All public APIs documented: [YES/NO - missing items]
- [ ] Clippy warnings addressed: [YES/NO - remaining warnings]
- [ ] Security audit clean: [YES/NO - vulnerabilities]

### Documentation Review
- [ ] README.md complete with setup instructions: [YES/NO]
- [ ] Configuration examples provided: [YES/NO - missing examples]
- [ ] API documentation accurate: [YES/NO]

### Operational Readiness
- [ ] Verification script passes: [YES/NO - output]
- [ ] Can be containerized: [YES/NO]
- [ ] Logging suitable for production: [YES/NO]
- [ ] Metrics endpoint functional: [YES/NO]

### TDD Verification Summary
- [ ] Evidence of test-first development throughout: [YES/NO]
- [ ] Test coverage aligns with implementation: [YES/NO]
- [ ] Tests are meaningful, not just for coverage: [YES/NO]
- [ ] Test naming follows conventions: [YES/NO]

### Documentation Completeness
- [ ] README.md is comprehensive: [YES/NO]
- [ ] Configuration examples are clear: [YES/NO]
- [ ] API documentation is accurate: [YES/NO]
- [ ] No missing rustdoc on public items: [YES/NO]
- [ ] Inline comments are helpful and current: [YES/NO]

### Final Cleanliness Check
- [ ] `grep -r "TODO\|FIXME" src/`: [PASS/FAIL - list any]
- [ ] `grep -r "println!\|dbg!" src/`: [PASS/FAIL - list any]
- [ ] `grep -r "unwrap()\|expect(" src/ --exclude-dir=tests`: [PASS/FAIL]
- [ ] `find . -name "*.rs.bk" -o -name "*.orig"`: [PASS/FAIL - list any]
- [ ] No unused dependencies in Cargo.toml: [YES/NO]
- [ ] No unused files or modules: [YES/NO]

### Outstanding Issues
[Any issues that must be fixed before Phase 2]

### Recommendations for Phase 2
[Specific suggestions based on Phase 1 implementation]

### Decision: [APPROVED FOR PHASE 2 / CHANGES REQUIRED]

### Sign-off
Reviewed by: [Agent/Human Name]
Date: [Date]
Phase 1 Status: [COMPLETE / INCOMPLETE]
```

## How to Handle Issues

When you find issues during review:

1. **Categorize by severity**:
   - **CRITICAL**: Blocks progress, must fix immediately
   - **HIGH**: Should fix before continuing
   - **MEDIUM**: Can fix in parallel with next phase
   - **LOW**: Can defer to cleanup phase

2. **Provide specific fixes**:
   ```markdown
   Issue: Configuration validation allows port 0
   Severity: HIGH
   Fix: Update validation in src/config/validation.rs:
   ```rust
   #[garde(range(min = 1024, max = 65535))]
   pub port: u16,
   ```
   
3. **Update the work plan**:
   - Add specific tasks to fix issues
   - Estimate additional work units
   - Identify what can be done in parallel

## Review Decision Framework

### APPROVED
Grant approval when:
- All checklist items pass
- Only LOW severity issues found
- Implementation meets specifications
- Code quality is acceptable

### CHANGES REQUIRED
Require changes when:
- Any CRITICAL or HIGH severity issues found
- Multiple MEDIUM issues that compound
- Specifications not properly implemented
- Security vulnerabilities present
- Test coverage insufficient

## Final Review Checklist

Before submitting your review:
- [ ] Tested all specified scenarios
- [ ] Documented all findings clearly
- [ ] Provided specific fixes for issues
- [ ] Updated work plan if needed
- [ ] Made clear APPROVED/CHANGES REQUIRED decision
- [ ] Included all test outputs as evidence

## Template for Review Submission

```markdown
# Phase 1 - Checkpoint [N] Review

**Reviewer**: [Your designation]
**Date**: [Current date]
**Implementation Agent**: [Agent who requested review]

## Review Summary
[2-3 sentences summarizing the state of the implementation]

## Detailed Findings
[Your complete review output for this checkpoint]

## Required Actions
1. [Specific action with priority]
2. [Specific action with priority]

## Updated Timeline
[If changes required, how many additional work units needed]

## Decision
**[APPROVED / CHANGES REQUIRED]**

[If CHANGES REQUIRED]
The implementing agent must:
1. [Specific requirement]
2. [Specific requirement]
3. Request re-review when complete

[If APPROVED]
The implementing agent may proceed to [next checkpoint/phase].
```

Remember: Your review ensures quality and specification compliance. Be thorough but constructive, and always provide actionable feedback.