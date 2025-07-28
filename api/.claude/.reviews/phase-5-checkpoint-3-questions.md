# Phase 5 Checkpoint 3 Questions

**To**: Senior Developer  
**From**: Junior Developer  
**Date**: 2025-07-28

## Questions About Cleanup Plan

After reviewing the cleanup plan and warning details, I have these clarifications:

### 1. Deprecated Logging Module Removal
**Question**: Before removing the entire `src/logging/` directory, should I verify that all test cases are still passing with the new `observability::logging` module? Some tests might be using specific functions from the old module that need to be mapped to the new implementation.

### 2. Unused Struct Fields Strategy  
**Question**: For the unused struct fields (`resource_id`, `original`, `config`), should I:
- Remove them entirely if they appear to be unused  
- Add `#[allow(dead_code)]` if they look like they're designed for future features
- Check if they're part of a public API that external code might depend on

### 3. Ambiguous Re-exports Decision
**Question**: For the `services/mod.rs` ambiguous exports, which approach do you prefer:
- Solution 1: Explicit imports (more verbose but clearer)
- Solution 2: Rename conflicts (more concise but potentially confusing)

### 4. Test Coverage During Cleanup
**Question**: Should I run the full test suite after each cleanup phase, or is it sufficient to run tests at the end? Given that some tests had compilation issues earlier, should I fix those as part of this cleanup?

### 5. Commit Granularity
**Question**: You mentioned 3-5 focused commits. Should I create separate commits for:
- Remove deprecated logging module
- Fix ambiguous exports  
- Remove unused imports/variables
- Remove unused struct fields
- Any final cleanup

Or would you prefer a different grouping?

---

**Status**: Ready to proceed with cleanup implementation once these questions are clarified.

**Estimated Timeline**: 1-2 hours as suggested in the cleanup plan.