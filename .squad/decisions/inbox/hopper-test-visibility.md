# Hopper: Lexer Tests Not Running

**Date:** 2026-02-27
**Agent:** Hopper
**Status:** OBSERVATION — Requires Decision

## Issue
Lexer test coverage is hidden. Three test modules were written but not included in the module tree:
- `boolean_tests.rs` (5 tests)
- `string_tests.rs` (5 tests)  
- `comments_tests.rs` (13 tests)

Currently only 2 tests run via `cargo test --lib` (those in lexer.rs directly). Total: ~23 tests written, ~2 tests executed.

## Root Cause
Files exist in `lexer/src/` but are NOT declared as modules in `lib.rs`. The file `lib.rs` only exports `mod lexer;`, making the standalone test modules invisible to cargo.

## Options
1. **Add mod declarations to lib.rs** → All tests become discoverable and runnable
   ```rust
   mod boolean_tests;
   mod string_tests;
   mod comments_tests;
   ```
   Downside: Inline tests in lexer.rs + separate modules might be redundant.

2. **Delete separate test files, move tests to lexer.rs** → Single test file per convention
   Downside: lexer.rs becomes very large.

3. **Leave as-is** → Tests exist but remain dormant
   Downside: False sense of test coverage; CI would not catch regressions.

## Recommendation for Squad
Clarify: Should lexer tests be organized as separate modules (option 1) or consolidated in lexer.rs (option 2)?

This affects:
- How we structure parser tests (should they follow same pattern?)
- Test discovery and CI visibility
- Code organization conventions for this project
