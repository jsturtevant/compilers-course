# Turing — History

## Project Context
**Project:** COOL Language Compiler in Rust
**Stack:** Rust workspace (lexer with logos, parser with Chumsky)
**Goal:** Complete compiler targeting WebAssembly (WASM)
**User:** James Sturtevant

## Learnings

### 2026-02-27: WASM Target Architecture Decision

**Major Pivot:** Target changed from x86/ARM to **WebAssembly (WASM)**.

**Rationale for WASM:**
1. Single target simplifies codegen (no dual-backend complexity)
2. WASM's stack-based model matches COOL's expression semantics naturally
3. Runs in browsers + CLI via wasmtime/wasmer — more accessible than native binaries
4. No register allocation needed (WASM locals + stack)
5. Structured control flow maps directly from COOL's if/while/case

**Key Architecture Decisions Made:**

1. **Three-Tier IR:** HIR (typed AST) → MIR (stack-based CFG) → LIR (linear WASM-like)
   - HIR preserves COOL semantics for semantic analysis
   - MIR eliminates complex expressions, makes control flow explicit
   - LIR maps 1:1 to WASM instructions for trivial codegen

2. **Stack-Based MIR (not SSA):**
   - WASM is a stack machine; SSA's register-like model adds unnecessary complexity
   - Stack-based IR naturally evaluates COOL expression trees in correct order
   - Rejected: SSA (overkill), 3-address code (wrong model), tree-IR (too simple)

3. **Dynamic Dispatch via `call_indirect`:**
   - Each class has vtable (array of function indices) in linear memory
   - Object header contains vtable pointer
   - Method slot assignment: parent methods first, overrides replace, new methods append

4. **Memory Management: Arena Allocator (MVP):**
   - Bump allocator for MVP (simple, no GC complexity)
   - Future: reference counting or WASM GC proposal
   - Objects stored in linear memory with header (class_tag, size, vtable_ptr)

5. **IO via Host Imports:**
   - IO.out_string, IO.in_int, etc. are WASM imports
   - Dual runtime: JS glue for browsers, WASI for CLI
   - Clean separation: compiler generates pure WASM, runtime is pluggable

6. **Stanford Compatibility Preserved:**
   - `--stanford-lexer`, `--stanford-parser`, `--stanford-semant` flags
   - Validate front-end against reference compilers before WASM codegen

**Object Layout in WASM Linear Memory:**
```
Offset  Field
0       class_tag (i32) — for case dispatch
4       size (i32) — for copy()
8       vtable_ptr (i32) — points to method table
12+     attributes (i32 each)
```

**String Layout:**
```
Offset  Field
0       length (i32)
4+      UTF-8 data
```

**Complexity Assessment:**
| Phase | Complexity | Risk |
|-------|------------|------|
| Semantic Analysis | Large | Medium (SELF_TYPE is tricky) |
| IR Design | Large | Low (well-understood problem) |
| WASM Codegen | XL | High (dynamic dispatch, object layout) |
| Runtime | Large | Medium (IO bridging, memory mgmt) |

**Timeline:** ~10 weeks to complete compiler

**Critical Path:** Semantic Analysis → IR → WASM Codegen (these are sequential blockers)

---

### 2026-02-27 (SECOND PASS): Architecture Finalization & Team Status

**Project Maturity Assessment:**
✅ **Lexer:** COMPLETE. 43 token types, logos-based, tested (29 unit tests passing). Handles full COOL syntax including nested comments, string escaping, case-sensitive keywords.
✅ **Parser:** FUNCTIONALLY COMPLETE but undertested. All 29 BNF rules implemented in Chumsky. Parses hello_world.cl, arith.cl correctly. AST structure is well-designed for codegen. Zero parser unit tests—this is a gap.
❌ **Semantic Analysis:** NOT STARTED
❌ **IR Layer:** NOT STARTED
❌ **Codegen (x86/ARM):** NOT STARTED

**Team Assignments (from charter review):**
- **Hopper** 🔧: Compiler developer — owns lexer/parser, semantic analysis next
- **Ritchie** ⚙️: Systems developer — owns codegen, IR design, register allocation
- **Stroustrup** 🦀: Rust expert — code review, idioms, performance guidance
- **Knuth** 📝: Documentation — BNF, READMEs, architecture docs
- **Ralph**: Testing visibility (referenced in inbox)

**Critical Architectural Insights:**

1. **Parser is sound but needs testing discipline.** Chumsky's combinator approach handles precedence well (unary > ×/÷ > +/- > </≤/=). No test suite exists—Hopper must add parser unit tests immediately to prevent regression.

2. **Symbol table design is necessary before semantic pass.** Parser produces correct AST, but type checking requires a scope stack. Recommend Hopper implements scope tracking as trait-based registry (matches Stroustrup's Rust idioms). This enables future optimization passes without refactoring.

3. **IR decision is the critical path blocker.** Ritchie has flagged this correctly (see inbox/ritchie-ir-design-needed.md). Without IR agreement, backend work stalls. **Recommend CFG with basic blocks** (matches my inbox decision). Structured enough for pattern matching, simple enough for dual x86/ARM backends.

4. **Standard library synthesis is the pragmatic choice.** COOL needs Object, IO, String, Int, Bool. Rather than hand-code asm per backend, recommend synthesizing these as built-in classes during semantic analysis. Cleaner, avoids duplication, aligns with compiler theory.

5. **Calling convention consistency between targets.** System V ABI (x86-64) and ARM64 system convention have different parameter passing. Recommend codegen module documents both clearly; register allocation and calling convention stack frame layout must be decided upfront.

**Sample Programs Validate Scope:**
- **hello_world.cl:** 2 lines, IO usage
- **cool.cl:** Minimal, object instantiation + dispatch
- **atoi_test.cl:** ~40 lines, string manipulation, loops
- **arith.cl:** ~260 lines—*complex*: inheritance chains, method overriding, dispatch, arithmetic, nested let/while/if, block structures. Tests the whole pipeline.
- **life.cl:** ~300 lines, cellular automaton, 2D grid logic. Stresses memory management.

Samples prove the parser handles real COOL complexity. Arith.cl and life.cl are good integration test targets.

**Workspace Health:**
- Clean Cargo.toml structure (workspace with 2 members)
- No TODOs/FIXMEs in source code (search came up empty)
- Build succeeds, tests pass, no broken dependencies
- Good separation: lexer crate → parser crate (clean boundary)
- Ready to add `ir/` and `codegen/` crates without disruption

**Architectural Concerns Requiring Action:**

1. **Parser test coverage:** 0 tests for parser logic. This is a vector for regression. Hopper must add BNF-per-production tests + error recovery tests.
2. **Error recovery strategy:** Parser currently fails on first error. Ariadne integration is present but unused. Plan error reporting architecture before semantic analysis.
3. **IR-to-Codegen gap:** Without IR, Ritchie cannot begin x86/ARM instruction selection. This is the sequence blocker.
4. **Type environment persistence:** Semantic analyzer will traverse AST multiple times (symbol resolution, type checking, lowering). Need efficient scope-aware data structure—recommend scope stack with parent pointers.

**Recommended Phase Sequence (for team):**
1. **Parser hardening** (Hopper) — add parser unit tests, error recovery, fix any parsing gaps
2. **Semantic analysis foundation** (Hopper) — symbol table + type environment, single-pass type checker
3. **IR specification** (Hopper + Ritchie pair) — finalize CFG+basic blocks design, document invariants
4. **IR lowering** (Ritchie) — AST to IR, class layout, vtable generation
5. **x86-64 codegen** (Ritchie) — instruction selection, register allocation, System V ABI integration
6. **ARM64 codegen** (Ritchie) — parallel target using shared IR

**File Structure Summary:**
```
compilers-course/
├── Cargo.toml (workspace: lexer, parser)
├── README.md
├── lexer/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs (re-exports Token)
│   │   ├── lexer.rs (logos enum, 43 tokens)
│   │   ├── main.rs (CLI tool)
│   │   ├── boolean_tests.rs (5 tests)
│   │   ├── string_tests.rs (5 tests)
│   │   └── comments_tests.rs (13 tests)
│   └── [28 unit tests defined, all passing]
├── parser/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs (Chumsky parser, 438 LOC)
│   │   └── ast.rs (19 Expr variants, 108 LOC)
│   ├── bnf (29-line grammar specification)
│   └── [0 unit tests]
├── samples/
│   ├── hello_world.cl
│   ├── arith.cl (~260 LOC, complex)
│   ├── life.cl (~300 LOC, complex)
│   ├── atoi_test.cl
│   └── cool.cl
└── .squad/ (team coordination, this session's archives)
```

**Key Decisions Ratified This Session:**
- ✅ CFG with basic blocks for IR (defers expression vs statement detail to Hopper/Ritchie pairing)
- ✅ Compiler-synthesized standard library (no hand-written asm)
- ✅ System V ABI for x86-64, ARM64 system convention (backend-aware callsites)
- ✅ Scope stack for symbol table (trait-based, extensible)
- ✅ Integration test harness per phase (parser → semantic → IR → codegen)

**Next Checkpoint:** IR specification document (Knuth + Ritchie to draft). Once IR interface is locked, Hopper can design AST→IR lowering, and Ritchie can begin instruction selection.

---

### 2026-02-27: Stanford Reference Toolchain Validation

**Task:** Explore Stanford binaries, document toolchain usage, create validation infrastructure.

**Stanford Binaries Discovered (`cool-support/bin/.i686/`):**
- `coolc` — Full compiler pipeline (lexer→parser→semant→cgen), generates MIPS assembly
- `lexer` — Tokenizer only, outputs token stream in Stanford format
- `parser` — Parser only, **HANGS** (likely needs special flags, DO NOT USE in automation)
- `semant` — Semantic analyzer (standalone)
- `cgen` — Code generator (MIPS assembly output)
- `spim` — MIPS simulator/runtime, executes generated `.s` files
- Utilities: `xspim` (GUI), `anngen`, `aps2c++`, `aps2java`

**Critical Discovery:** All binaries are 32-bit i686 ELF from 2011, required `chmod +x` to execute.

**SPIM Runtime Requirements:**
1. Must specify `-trap_file` pointing to `cool-support/lib/trap.handler`
2. Outputs SPIM banner + program output + success message (must filter for validation)
3. Full command: `spim -trap_file <path>/trap.handler -file program.s`

**Sample Programs Analysis (18 total in `cool-support/examples/`):**
- **16 single-file programs:** Successfully compile and run
- **2 multi-file programs:** `atoi.cl` (library, no Main) + `atoi_test.cl` (depends on A2I class)
- Most complex: `arith.cl` (9689 lines output, interactive), `lam.cl` (528 lines, lambda calculus)
- Simplest: `hello_world.cl` (1 line), `cool.cl` (1 line)

**Validation Infrastructure Created:**

1. **`scripts/validate-samples.sh`** (executable):
   - Loops through all `.cl` files in `cool-support/examples/`
   - Compiles with Stanford `coolc`
   - Runs with `spim` (5-second timeout for interactive programs)
   - Captures program output to `scripts/expected-outputs/*.output`
   - Handles compilation failures gracefully
   - **Results:** 16 successful, 2 failed (multi-file programs)

2. **`scripts/expected-outputs/`** directory:
   - 18 `.output` files (one per program)
   - Contains ground truth outputs for testing our WASM compiler
   - Format: Plain program stdout (SPIM banners stripped)
   - Failures marked as `COMPILATION_FAILED`, `RUNTIME_ERROR`, or `INTERACTIVE_TIMEOUT`

**Stanford Toolchain Usage Patterns:**

```bash
# Single file compilation
coolc program.cl              # → program.s
spim -trap_file lib/trap.handler -file program.s

# Multi-file compilation
coolc lib.cl main.cl          # → lib.s (first filename)

# Lexer only (for --stanford-lexer comparison)
lexer program.cl              # → token stream

# Parser HANGS — DO NOT USE
```

**Integration Plan for Our Rust Compiler:**

1. **Phase-by-phase testing:**
   - Lexer: Compare against `lexer` output (use `--stanford-lexer` flag)
   - Parser: AST structure validation (no direct Stanford equivalent)
   - Semantic: Compare errors against `semant` output (use `--stanford-semant`)
   - Codegen: **Final outputs must match `scripts/expected-outputs/*.output`**

2. **End-to-end validation:**
   ```bash
   cargo run --bin cool-compiler -- hello_world.cl -o hello.wasm
   wasmtime hello.wasm > our_output.txt
   diff our_output.txt scripts/expected-outputs/hello_world.output
   ```

3. **Key requirements for WASM backend:**
   - IO.out_string, IO.in_int, etc. must produce **identical output** to Stanford
   - Built-in classes (Object, IO, String, Int, Bool) must match Stanford semantics
   - Error messages don't need exact match (functional correctness > message format)

**Critical Testing Insights:**

1. **No need for --stanford output format in final WASM** (user requirement: functional correctness only)
2. **Ground truth established:** 16 validated programs ready for regression testing
3. **Multi-file support needed eventually** (atoi.cl case), but not MVP blocker
4. **Interactive programs** (arith.cl) require input handling design
5. **Lexer token format** is well-defined by Stanford's lexer output

**Files Created:**
- `scripts/validate-samples.sh` — Validation script (178 lines)
- `scripts/expected-outputs/*.output` — 18 ground truth files
- `.squad/decisions/inbox/turing-validation-setup.md` — Full documentation

**Next Actions:**
1. Add `make validate` target that runs validation script in CI
2. Create integration test harness that diffs WASM outputs vs expected-outputs/
3. Use Stanford lexer/semant for front-end validation before WASM work starts
4. Consider parser hardening (Stanford's parser hangs, ours shouldn't)

**Validation Script Stats:**
- 18 programs processed
- 16 successful outputs captured (hello_world: 1 line → lam: 528 lines)
- 2 multi-file programs correctly identified as compilation failures
- Total ground truth lines: ~10,500 (arith.cl alone is 9689 due to interactive prompts)

---

### 2026-02-27: Architecture Exploration & Assessment

**Compiler Pipeline Status:**
- ✅ **Lexer (Complete)**: logos-based tokenizer, ~336 LOC. Handles full COOL syntax including keywords, identifiers, literals, operators, and nested comments with depth tracking. Includes unit & integration tests.
- ⏳ **Parser (In Progress)**: Chumsky combinators, ~331 LOC. AST defined and partially working. Can parse hello_world.cl and arith.cl but incomplete coverage of all expression types.
- ❌ **Semantic Analysis**: None yet
- ❌ **IR Generation**: None yet
- ❌ **Code Generation (x86/ARM)**: None yet
- ❌ **Optimization**: None yet
- ❌ **Linker/Runtime**: None yet

**Key Architecture Decisions:**
- Workspace split: `lexer/` and `parser/` as independent crates with lexer as dependency
- BNF grammar formally defined in parser/bnf (29 production rules)
- AST structure in place: Program → Classes → Features (Methods/Attributes) → Expressions
- Error handling: Token::Error variant for graceful parsing recovery
- Testing: Test modules inline with production code (comments_tests.rs, string_tests.rs, etc.)

**Critical Missing Phases (in order):**
1. **Semantic Analysis**: Type checking, symbol resolution, attribute/method validation
2. **Intermediate Representation**: AST → IR (consider lowering to 3-address code or similar)
3. **Code Generation Backend**: x86-64 and ARM64 target selection
4. **Runtime/Standard Library**: COOL has built-in classes (IO, Object, etc.)
5. **Linker Integration**: Connect generated code to system libraries

**File Structure:**
- Root: Cargo.toml workspace with lexer + parser members
- Samples: 5 test programs (hello_world.cl, arith.cl, life.cl, atoi_test.cl, cool.cl)
- Each crate has: Cargo.toml, src/(lib.rs, main.rs, *.rs tests)

**Technical Observations:**
- Parser uses logos error recovery (not crashing on bad tokens)
- Chumsky's recursive descent parser with combinator patterns
- Ariadne integration present but not yet utilized for pretty error reporting
- No error span tracking beyond token-level
- AST uses Box<Expr> for recursive types (standard Rust pattern)

**Architectural Concerns & Recommendations:**
1. **Symbol Table / Type Environment**: Plan a persistent data structure for semantic passes. Consider trait-based approach for extensibility (will need for multiple passes).
2. **IR Design**: COOL is high-level (OOP, methods); IR should target a machine-like abstraction (e.g., basic blocks, CFG). This bridges front-end and code generation.
3. **Standard Library**: COOL expects Object, IO, Main classes. Decide early: hand-written asm stubs or generated from COOL source?
4. **Linkage Model**: When generating x86/ARM code, what calling convention? SysV ABI (x86-64 Linux) vs ARM64 EABI? Affects codegen phase.
5. **Testing Strategy**: Currently only lexer + parser are tested. Semantic analysis and codegen need test infrastructure (expected outputs, diffing AST/IR).
6. **Separation of Concerns**: Parser main.rs does parsing + printing. Consider refactoring to a library boundary so tests can compose phases.

**Next Immediate Steps** (for team):
1. Complete parser (expression precedence, all rules in BNF must parse to AST)
2. Write semantic analyzer pass (symbol resolution, type checking)
3. Design IR and add lowering pass from AST to IR
4. Choose backend (x86-64 first, given team expertise)
5. Add test harness that validates end-to-end (source → asm with expected output)

---

### 2026-02-27: Semantic Analysis Architecture Design

**Task:** Design and implement the semantic analysis crate structure for Phase 2 of the COOL compiler.

**Architecture Created:**

1. **`semant/` Crate Structure:**
   - `src/lib.rs` - Main analyzer with three-phase pipeline
   - `src/symbol_table.rs` - Scope stack for variable/method bindings
   - `src/class_hierarchy.rs` - Inheritance graph with built-in class synthesis
   - `src/types.rs` - Type representation and conformance checking

2. **Three-Phase Analysis Pipeline:**
   - Phase 1: Build class hierarchy (detect duplicates, validate inheritance)
   - Phase 2: Detect cyclic inheritance
   - Phase 3: Type checking (delegated to Hopper for implementation)

3. **Built-in Classes Synthesized:**
   - Object (root): abort(), type_name(), copy()
   - IO: out_string(), out_int(), in_string(), in_int()
   - String: length(), concat(), substr()
   - Int, Bool: inherit from Object

4. **Key Design Decisions:**
   - **Scope Stack:** Reused existing ScopeStack implementation (push/pop scopes, variable lookup with shadowing)
   - **Type System:** Type enum with Class, SelfType, NoType variants
   - **Conformance:** Implemented as hierarchy traversal (subtype checking)
   - **LUB (Least Upper Bound):** Finds common ancestor for case expressions
   - **Error Recovery:** SemanticError enum with 9 error types

5. **COOL-Specific Constraints Enforced:**
   - Cannot inherit from Int, String, Bool, or SELF_TYPE
   - All classes implicitly inherit from Object if no parent specified
   - Cycle detection in inheritance chains
   - Duplicate class definition detection

6. **Integration Points:**
   - Created `parser/src/lib.rs` to expose AST types as library
   - Added semant to workspace Cargo.toml members
   - Dependencies: parser crate (for AST types)

7. **Testing:**
   - 12 unit tests created covering:
     - Built-in class hierarchy
     - Type conformance
     - LUB computation
     - Scope stack operations (nesting, shadowing, duplicate detection)
     - Type representation and literal inference
   - All tests passing

**Interfaces Defined for Hopper:**
- `SemanticAnalyzer::check_program()` - Entry point for full program validation
- `TypeChecker::infer_type()` - Expression type inference (stubs for complex expressions)
- `TypeChecker::check_conformance()` - Type compatibility checking
- Error handling via Result<(), Vec<SemanticError>>

**Implementation Strategy:**
- Architecture focuses on structure and public interfaces
- Placeholder implementations for complex logic (method dispatch, case expressions, let bindings)
- Hopper will fill in:
  - Full expression type checking
  - Method signature validation
  - Attribute initialization checking
  - Dispatch resolution (static and dynamic)
  - SELF_TYPE handling in all contexts

**Build Status:** ✅ Compiles cleanly, all tests pass

---
