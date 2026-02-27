# Stroustrup — History

## Project Context
**Project:** COOL Language Compiler in Rust
**Stack:** Rust workspace (lexer with logos, parser with Chumsky)
**Goal:** Complete compiler with x86 and ARM backends
**User:** James Sturtevant

## Learnings

### Code Review: Rust Quality & Patterns (2025-02-27)

**Workspace Setup: ✓ Well-Structured**
- Workspace resolver "3" is correct for multi-crate setup
- Lexer and parser are cleanly separated with proper path-based dependency
- Both crates expose clean public APIs via lib.rs

**Edition Alert: ⚠️ CRITICAL**
- Both crates use `edition = "2024"`, which **does not exist**. Rust 1.85 only supports 2021 (latest)
- Should be `edition = "2021"` in both lexer/Cargo.toml and parser/Cargo.toml
- This is a build configuration error that will fail on different Rust versions

**Dependencies: ✓ Well-Chosen**
- `logos 0.15`: Perfect for lexing. Logos is 2-3x faster than hand-written lexers, compile-time generated
- `chumsky 0.10.0`: Excellent parser combinator library. Clean error recovery and great error messages via `ariadne`
- `ariadne 0.5.1`: Used by parser, provides beautiful diagnostic output
- `clap 4.5` with derive feature: Idiomatic CLI in lexer (good UX)
- Dependency versions are pinned but not too restrictive—reasonable choices

**Error Handling: ✓ Good Patterns**
- **Lexer**: Proper error recovery. Uses `Token::Error` variant for invalid tokens, allowing parsing to continue. Line/column tracking in extras is solid
- **Parser**: Excellent error design. Chumsky's `Rich<Token>` errors with spans allow meaningful diagnostics
- Both use `Result` types correctly; no panics in core logic except test assertions
- No unwrap() in critical paths—reserved for tests and CLI argument parsing where it's appropriate

**Code Organization & Modules: ✓ Clean**
- Lexer:
  - lib.rs exports Token publicly + clean module hierarchy
  - Main.rs handles CLI with proper error messages (stderr output)
  - Test modules organized by feature (boolean_tests, comments_tests, string_tests)
  - Nice: sample file testing via fs::read_dir in tests
- Parser:
  - ast.rs contains clean, well-structured AST definition
  - Main.rs builds token stream correctly (logos → Stream → chumsky)
  - Recursive expression parsing is well-structured with proper precedence handling

**Anti-Patterns & Issues: ⚠️ Minor**

1. **Clippy Warning in parser/src/main.rs:261** — `let_and_return`
   - Line 244-261: `let comparison = ...` followed by `comparison` return
   - Should inline the expression directly (cosmetic, not breaking)

2. **Unused imports in ast.rs**
   - `use std::boxed::Box;` is unnecessary (Box is in prelude)
   - `#![allow(unused)]` at the top suppresses warnings—should remove this and clean up

3. **Token::Error handling inconsistency**
   - Lexer defines `#[allow(dead_code)]` on Error variant, but it IS used in parser
   - No real problem, but the comment suggests uncertainty

4. **Comment handling in nested match**
   - Parser line 216: `Token::Tilde => ast::Expr::Not(...)` with comment "Use Not for now, could add Negate later"
   - Should create a separate Negate variant when language supports negation

5. **Missing error context in CLI**
   - Parser main.rs: `unwrap()` on file read and env args—should use proper error messages
   - Lexer handles this better with eprintln! diagnostics

**Performance & Idiom Notes: ✓ Solid**
- Logos DFA compilation happens at compile-time—zero runtime cost
- Chumsky parser is well-structured; no obvious inefficiencies
- String cloning in regex callbacks is unavoidable with logos (idiomatic)
- `.clone()` on parser expressions is necessary due to recursive grammar—good use of Rc/Box for AST

**Testing: ✓ Good Coverage**
- 29 tests pass (lexer comprehensive)
- Tests cover: booleans, comments (nested!), strings, edge cases
- Sample files tested: .cl files from samples/ directory
- Parser has no tests yet (expected—still under development)

**Suggestions for Next Phase:**
1. Fix edition to 2021 immediately
2. Remove `#![allow(unused)]` from ast.rs and clean up imports
3. Address clippy warning (inline comparison)
4. Add proper CLI error handling to parser/main.rs
5. Consider semantic versioning for deps once compiler is feature-complete

---
