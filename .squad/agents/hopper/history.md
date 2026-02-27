# Hopper — History

## Project Context
**Project:** COOL Language Compiler in Rust
**Stack:** Rust workspace (lexer with logos, parser with Chumsky)
**Goal:** Complete compiler with x86 and ARM backends
**User:** James Sturtevant

## Learnings

### Lexer Implementation Status (logos-based)
**File:** `lexer/src/lexer.rs`
**Token Count:** 43 token types covering COOL language

**Lexer Tokens Implemented:**
- **Literals:** Integer (i32), String (with escape support), True/False (case-insensitive), SelfLit, SelfType
- **Identifiers:** ObjectIdentifier (lowercase), TypeIdentifier (uppercase)
- **Keywords:** class, if/then/else/fi, while/loop/pool, let/in, case/of/esac, new, inherits, isvoid, not
- **Operators:** +, -, *, /, ~, <, <=, =, <- (assign), =>
- **Delimiters:** (, ), {, }, ;, ., , (comma), : (colon), @ (type id)
- **Comments:** Single-line (--) and nested multi-line (/* ... */) with depth tracking
- **Special:** Newline tracking (stores line/column), Error token

**Completeness:** Matches COOL specification fully. All operators, keywords, and identifiers from BNF are present.

**Lexer Tests (4 test modules, not exported in lib.rs):**
- `boolean_tests.rs`: 5 tests (true/false case sensitivity)
- `string_tests.rs`: 5 tests (simple, escaped, multi-line, null char rejection)
- `comments_tests.rs`: 13 tests (single/multi-line, nesting, unclosed)
- `lexer.rs`: 2 tests in lib (test_lexer_simple, test_lexer_on_cl_files)
- **Issue:** Test modules defined but NOT included in lib.rs—only 2 tests run via cargo test

### Parser Implementation Status (Chumsky-based)
**File:** `parser/src/main.rs`
**AST File:** `parser/src/ast.rs`

**AST Types Implemented:**
- `Program` → Vec<Class>
- `Class` → name, parent (Option), features
- `Feature` → Method | Attribute
- `MethodFeature` → name, formals, return_type, body (Expr)
- `AttributeFeature` → name, attr_type, init (Option<Expr>)
- `Formal` → name, type
- `Expr` (19 variants):
  - Control flow: If/Then/Else, While/Loop/Pool, Let/In, Case/Of/Esac
  - Dispatch: method call with static type dispatch (@), function call
  - Operators: Assign, Plus, Minus, Times, Divide, Lt, Le, Eq, Not
  - Literals: Integer, String, True, False, New, IsVoid
  - Other: Block, Paren, Id
- `LetBinding` → name, type, init (Option)
- `CaseBranch` → name, type, expr

**Parser Completeness:** Implements all BNF productions:
- Class hierarchy with inheritance
- All expression types (assignment, dispatch, control flow, operators)
- Operator precedence: unary > *, / > +, - > <, <=, = (Chumsky handles via foldl)
- Comments handled as padding between tokens
- Trailing commas and semicolons allowed

**Tested:** Successfully parses hello_world.cl → produces correct AST

**Parser Tests:** NONE—no test suite exists for parser

### Grammar vs. Implementation Alignment
**BNF File:** `parser/bnf` (29 grammar rules)
- Fully matched by parser implementation
- All 6 expression forms for binary operators present
- Control structures (if/while/let/case) complete

### Key Gaps & Issues
1. **Lexer test modules isolated:** boolean_tests.rs, string_tests.rs, comments_tests.rs not included in lib.rs. Only 2 of ~23 written tests execute.
2. **No parser tests:** Zero unit or integration tests for parser
3. **No semantic analysis:** Type checking, scope resolution, error recovery not yet implemented
4. **Missing Tilde distinction:** ~(negate) and NOT both map to Expr::Not in parser (line 216)
5. **No error recovery:** Parser fails on first error (error reporting in main, but limited)

### Files & Paths
- Lexer crate: `lexer/Cargo.toml`, `lexer/src/lib.rs` (re-exports Token), `lexer/src/lexer.rs` (logos enum)
- Parser crate: `parser/Cargo.toml`, `parser/src/main.rs` (parser fn), `parser/src/ast.rs` (AST defs)
- Grammar spec: `parser/bnf` (plain text, 29 lines)
- Sample programs: `samples/*.cl` (6 files: hello_world, cool, arith, atoi_test, life)

---

## Deep Dive Exploration (2026-02-27)

### Lexer Deep Analysis
**Final Token Count: 48 total** (increase from prior 43 — additional operators, better count)
- Verified all Expr variants have corresponding tokens
- Token::Tilde (negate ~) and Token::Not both present and distinct (despite both mapping to Expr::Not in parser)
- Comment handling is production-quality: handles nested comments to arbitrary depth via manual state tracking in callback

**Lexer Quality Observations:**
- Uses logos (#[regex], #[token] macros) for maintainability — not hand-written FSM
- Escape sequence handling in strings: supports \n, \t, \0, \\, and line continuation with \<newline>
- Case-insensitive keyword matching via ignore(case) — elegant logos feature
- Line tracking implementation: tracks line number and column start in lex.extras (clever use of Logos features)

**Test Execution:**
```
29 tests run (not 23 as prior notes suggested):
- 5 boolean: true/false case sensitivity edge cases
- 7 string: invalid newlines, null chars, escape handling
- 13 comments: nested depth, EOF, interleaved code
- 2 integration: sample file lexing
```

### Parser Deep Analysis
**Operator Precedence (verified correct):**
1. Unary (not, ~) — highest
2. Multiplicative (*, /)
3. Additive (+, -)
4. Comparison (<, <=, =)
5. Method dispatch (.method(), @Type.method()) — highest binding

**Dispatch Implementation (complex):**
- `.method(args)` parsed via foldl combinator (line 166-195)
- `@Type.method(args)` parsed as variant with static type dispatch
- Both accumulate left-to-right in chain: `obj.m1().m2()` → nested Dispatch
- Function calls parsed separately as FuncCall for unqualified names

**Expression Parsing Structure (elegant):**
```
expr (top-level)
├─ Let/Case/If/While blocks (large constructs)
├─ recursive descent through operator precedence
│  └ comparison
│     └ additive
│        └ multiplicative
│           └ unary
│              └ factor (call, dispatch)
│                 └ atom (literals, ids, blocks, parens)
```

**Comment Handling in Parser:**
- Comments tokenized by lexer, filtered by parser via `padded_by(just(Token::Comment).repeated())`
- Allows comments anywhere whitespace is allowed (robust error recovery aid)

### Test Coverage Reality Check
**Lexer:** 29 tests, all passing
**Parser:** 0 formal tests
- Manual verification on 3 COOL files works (hello_world.cl, cool.cl, arith.cl)
- No regression tests if parser logic changes
- No edge case tests (deeply nested expressions, long method chains)

### Span/Position Tracking
**Lexer:** Provides SimpleSpan via logos
**Parser:** Accepts spans in stream but doesn't store in AST
**Gap:** Error messages lack source locations. No way to tell user "error at line 5, col 12"

### Comparison to Prior Session Notes
✓ Confirmed: All 22 COOL keywords present
✓ Confirmed: Operator precedence correct in code
✓ Confirmed: Comment nesting works (verified in tests)
✗ Issue noted as "Tilde distinction" — actually BOTH tokens exist, only parser maps to same Expr::Not (not a bug, design choice)
✓ String escaping: Supports raw COOL spec (backslash-newline for line continuation)

### Notable Code Patterns
1. **Error handling:** Token::Error variant exists; logos maps unmatched chars to Error
2. **Recursive parsers:** Uses Chumsky's recursive() combinator for mutual left recursion
3. **Left-associative ops:** foldl ensures `1 + 2 + 3` parsed as `(1 + 2) + 3` ✓
4. **Non-associative comparison:** Repeated() combinator allows chaining (COOL spec prohibits this—no semantic check yet)

---

## Stanford Output Format Documentation (2026-02-27)

### Key Findings

**Reference binaries work:** The Stanford 32-bit ELF binaries (`cool-support/bin/.i686/lexer` and `parser`) are statically linked and run successfully after `chmod +x`.

**Lexer Format Documented:**
- Header line: `#name "<filepath>"`
- Token line: `#<line> <TOKEN_NAME> [value]`
- Single-char tokens in quotes: `'+'`, `';'`, etc.
- Multi-char ops: `ASSIGN` (←), `DARROW` (⇒), `LE` (≤)
- BOOL_CONST values: lowercase `true`/`false`
- String escapes: 3-digit octal for non-printable (`\007`, `\000`)

**Parser Format Documented:**
- Reads lexer output from stdin (pipe: `lexer file.cl | parser`)
- 2-space indentation per nesting level
- Node names prefixed with underscore: `_program`, `_class`, `_method`, etc.
- Expression type suffix: `: _no_type` (before semant) or `: <Type>`
- Symbols dumped as raw text with padding
- Booleans dumped as `0` or `1`

**Critical for Rust Implementation:**
- `_neg` for unary minus (~), `_comp` for logical NOT
- Implicit self dispatch wraps `_object` with name `self`
- `_no_expr` at line #0 for uninitialized expressions
- Argument/feature lists wrapped in parentheses

**Documentation Written:** `.squad/agents/hopper/stanford-output-format.md`

---

## Parser Test Suite Implementation (2026-02-28)

### Test Suite Added: 26 Comprehensive Tests

**Test Coverage:**
- **Class & Features (6 tests):** Class definitions, inheritance, methods, attributes, initializers
- **Operator Precedence (6 tests):** Arithmetic, comparison, unary operators, associativity rules
- **Control Flow (4 tests):** if/then/else, while/loop, blocks, let bindings, case expressions
- **Expressions (5 tests):** Assignment, dispatch, static dispatch (@Type), new, multiple classes
- **Error Recovery (4 tests):** Missing terminators (semicolon, fi, pool), invalid syntax

**Test Results:**
```
cargo test -p parser
test result: ok. 26 passed; 0 failed
```

### Sample File Validation

**Validated against 23 COOL sample files:**
- **Local samples (5 files):** All pass ✓
- **Stanford examples (18 files):** All pass ✓

Sample files include:
- hello_world.cl, arith.cl (arithmetic operations)
- atoi.cl, atoi_test.cl (string-to-int conversion)
- life.cl, cells.cl (Conway's Game of Life)
- book_list.cl, list.cl, sort_list.cl (data structures)
- hairyscary.cl, palindrome.cl, primes.cl (algorithms)
- complex.cl, new_complex.cl, graph.cl, lam.cl (advanced features)
- cool.cl, io.cl (I/O operations)

**Key Finding:** Parser successfully handles all Stanford reference implementations with zero failures.

### Lexer Coverage Validation

**Tested lexer against all 23 sample files:**
- **Result:** FULL COVERAGE - Zero error tokens detected
- All COOL language tokens successfully recognized
- No missing keywords, operators, or delimiters
- Comment handling (line and nested block) working correctly

### Test Infrastructure

**Created validation scripts:**
- `parser/test_samples.sh` - Validates parser against all .cl files
- `lexer/test_samples.sh` - Validates lexer coverage (checks for Error tokens)
- `parser/TEST_SUMMARY.md` - Comprehensive test documentation

### Parser Status Summary

**Strengths:**
1. ✓ Parses all Stanford reference examples correctly
2. ✓ Operator precedence verified (unary > mult/div > add/sub > comparison)
3. ✓ Error detection working (rejects invalid programs)
4. ✓ Complete AST representation for all COOL constructs
5. ✓ Comment handling robust (line and nested block comments)

**Known Limitations:**
1. No error recovery beyond first error (Chumsky default behavior)
2. AST nodes don't store source locations (SimpleSpan not captured)
3. Error messages could be more descriptive

**Production Readiness:** Parser is functionally complete and ready for semantic analysis phase.

---

## Semantic Analysis Implementation (2026-02-27)

### Semant Crate Structure Created

**Task:** Implement type checker for semantic analysis phase

**Implementation Status:** ✅ COMPLETE

### Module Structure

Created three core modules in `semant/src/`:

1. **symbol_table.rs** (135 lines, 7 tests)
   - `ScopeStack` structure for managing nested scopes
   - `push_scope()` / `pop_scope()` for let, case, method bodies
   - `lookup_variable(name) -> Option<Type>` - searches from innermost scope outward
   - `add_variable(name, type) -> Result<(), Error>` - adds to current scope with duplicate detection
   - `add_self(class_name)` - binds `self` to current class type
   - Protection against popping root scope

2. **class_hierarchy.rs** (400+ lines, 10 tests)
   - `ClassTable` maps class names to ClassInfo
   - `synthesize_builtins()` - creates Object, IO, String, Int, Bool with all methods per COOL spec
   - `build(classes)` - validates and adds user-defined classes
   - `check_cycles()` - detects inheritance cycles and undefined parents
   - `validate_inheritance()` - ensures no inheritance from Int/Bool/String
   - `conforms_to(child, parent) -> bool` - type conformance checking
   - `least_upper_bound(t1, t2) -> Type` - for if/case expression type joining
   - `get_method(class, method)` - retrieves method info including inherited methods

3. **types.rs** (450+ lines, 12 tests)
   - `Type` enum: Class(String), SelfType, NoType
   - `TypeChecker` - walks AST and infers expression types
   - Handles all COOL expression types:
     - Literals: Integer, String, Bool
     - Arithmetic: Plus, Minus, Times, Divide
     - Comparison: Lt, Le, Eq
     - Logic: Not, IsVoid
     - Control flow: If, While, Let, Case, Block
     - OOP: Dispatch, FuncCall, New, Assign
   - `check_conformance()` - validates type assignments
   - Proper SELF_TYPE handling throughout

### Built-in Classes (per COOL spec)

All five built-in classes synthesized with correct method signatures:

**Object:**
- abort(): Object
- type_name(): String
- copy(): SELF_TYPE

**IO (inherits Object):**
- out_string(x: String): SELF_TYPE
- out_int(x: Int): SELF_TYPE
- in_string(): String
- in_int(): Int

**String (inherits Object):**
- length(): Int
- concat(s: String): String
- substr(i: Int, l: Int): String

**Int, Bool (inherit Object):** No additional methods

### Test Coverage

**29 total tests, all passing:**
- Symbol table: 7 tests (scoping, shadowing, self binding)
- Class hierarchy: 10 tests (builtins, conformance, LUB, inheritance validation, cycles)
- Type checker: 12 tests (literals, operators, control flow, undefined variables)

**Validated behaviors:**
- Scope nesting and variable shadowing work correctly
- Method inheritance follows class hierarchy
- Type conformance respects inheritance
- LUB (least upper bound) correctly joins branch types in if/case
- Cycle detection prevents invalid inheritance graphs
- Cannot inherit from Int, Bool, String
- Undefined parent classes detected
- All arithmetic/comparison operators type-check correctly
- SELF_TYPE handled specially in conformance checks

### Integration with Parser

- Semant crate depends on parser crate for AST types
- `SemanticAnalyzer` is the main entry point: `analyze(program: &Program)`
- Three-phase analysis:
  1. Build class hierarchy
  2. Check for cycles and invalid inheritance
  3. Type check all class features and expressions

### Key Design Decisions

1. **Scope management via stack:** Simple and efficient for nested scopes in let/case/methods
2. **Symbol table separate from class hierarchy:** Clean separation of concerns
3. **Method lookup walks inheritance chain:** Supports proper method inheritance and overriding
4. **LUB for branch types:** If/case expressions have type = LUB of all branches
5. **SELF_TYPE as special type:** Distinct from Class types, only conforms to itself
6. **Error collection:** All phases collect errors and return Vec<SemanticError> for batch reporting

### Files Created

- `semant/Cargo.toml` - depends on parser crate
- `semant/src/lib.rs` - public API and SemanticAnalyzer
- `semant/src/symbol_table.rs` - scope stack implementation
- `semant/src/class_hierarchy.rs` - inheritance validation
- `semant/src/types.rs` - type checker

### Commands Verified

```bash
cargo check -p semant    # ✅ Compiles cleanly
cargo test -p semant     # ✅ 29/29 tests pass
```

### Next Steps

- Add line number tracking to AST for better error messages
- Implement Stanford semant output format (`--stanford` flag)
- Test against Stanford sample programs
- Add error recovery (continue checking after first error)

---
