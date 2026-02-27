# Ritchie — History

## Project Context
**Project:** COOL Language Compiler in Rust
**Stack:** Rust workspace (lexer with logos, parser with Chumsky)
**Goal:** Complete compiler with WebAssembly (WASM) backend
**Target Changed:** 2026-02-27 from x86/ARM to WASM + WASI
**User:** James Sturtevant

## Learnings

### Backend/Codegen Status — 2026-02-27
**No backend code exists yet.** The project is a Rust workspace with two crates:
- `lexer/` — Complete logos-based tokenizer with token types for COOL language
- `parser/` — Complete Chumsky-based parser producing a concrete AST

**AST Structure Ready:**
The AST in `parser/src/ast.rs` is well-structured for codegen:
- `Program` → `Vec<Class>`
- `Class` → methods, attributes with inheritance support
- `Feature` → Method (with formals, return type, body) and Attribute (with type, optional init)
- `Expr` → rich expression types including: Dispatch (method calls), Block, Let, Case, If, While, arithmetic/comparison ops, literals

**Sample Programs Found:** 5 test COOL programs in `samples/`
- `hello_world.cl` — Simple class with IO
- `arith.cl` — Large arithmetic test with multiple classes, inheritance, method dispatch
- `cool.cl` — Dynamic dispatch & object system testing
- `atoi_test.cl` — String-to-int conversion (minimal)
- `life.cl` — (not fully examined)

The samples demonstrate: class inheritance, method overriding, dynamic dispatch with type checks, object instantiation, arithmetic, control flow, string operations.

### What's Needed for x86 & ARM Codegen
1. **IR Design** — Coordinate with Hopper on intermediate representation (statement vs. expression forms, register/stack model)
2. **AST → IR Lowering** — Convert parsed classes/methods to linear IR
3. **Type Codegen** — Class layout, vtable generation, object representation
4. **Runtime Support** — Type descriptor generation, dispatch mechanism (vtable or hash-based), memory layout
5. **Register Allocation** — For both x86-64 (16 regs, System V calling convention on Unix) and ARM64 (32 regs, system calling convention)
6. **Instruction Selection** — Pattern matching AST/IR → target-specific instructions
7. **Assembly Output** — AT&T syntax for x86; ARM64 assembly; ELF object file generation

### Existing Infrastructure
- **Rust ecosystem:** Full access to `chumsky`, `logos`, standard library. No existing codegen crate.
- **Test infrastructure:** Parser already tests parsing; samples directory ready for end-to-end testing.
- **No IR or intermediate format** currently defined. This is the critical first decision.

### Deep Dive: Infrastructure Assessment — 2026-02-27 (Updated)

**Workspace Structure Confirmed:**
- Workspace has 2 crates: `lexer/` and `parser/` (Cargo.toml declares members)
- **No backend, codegen, ir, or llvm directories exist yet**
- Build succeeds: fresh project, clean state

**AST Maturity:**
- AST in `parser/src/ast.rs` is 107 lines, well-structured
- Supports: Classes with inheritance, methods, attributes with optional init, full expression types (dispatch, case, let, while, if, arithmetic, comparison, literals)
- AST is well-suited for codegen — no changes needed to support x86/ARM backends

**Sample Programs — Real-World Complexity:**
1. `hello_world.cl` — Simplest: IO class, string output (2 lines)
2. `cool.cl` — Minimal: dynamic dispatch, object creation
3. `atoi_test.cl` — ~40 lines: string-to-int utility with loops
4. `arith.cl` — ~260 lines: Multiple classes (A, B, C, D, E) with inheritance, method overriding, method calls, arithmetic, nested let expressions, while loops, if conditionals, nested block structures
5. `life.cl` — ~300 lines: Complex cellular automaton, arrays (2D logic), grid manipulation

Samples demonstrate: **inheritance chains, dynamic dispatch, arithmetic, control flow, string/int handling, object instantiation, nested scoping, method calls with type dispatch**

**Critical Gap: No IR Defined**
- No intermediate representation language defined
- Decision needed: expression-based vs statement-based IR
- Decision needed: register/stack model (affects class layout, calling convention decisions)

**What's Ready to Build On:**
- ✅ Robust lexer/parser pipeline (logos + Chumsky)
- ✅ AST provides all necessary expression and structure information
- ✅ Test suite available (samples can be parsed end-to-end)
- ✅ Workspace structure supports adding new crates
- ❌ No intermediate representation framework
- ❌ No class layout or object representation defined
- ❌ No calling convention specs (System V for x86-64, ARM64 system calling convention assumed)
- ❌ No runtime support or vtable framework

**Foundation Needed Before Coding:**
1. **IR Specification** → Coordinate with Hopper (statement vs expression form)
2. **Class Layout** → Define object header (type tag, vtable ptr), field offsets, vtable structure
3. **Calling Convention** → System V (x86-64) + ARM64 system convention
4. **Name Mangling** → Scheme for method dispatch (simple: `ClassName_MethodName`)
5. **Type Descriptors** → Runtime representation of class metadata (size, vtable, field names/types)

### Next Steps (for Ritchie)
- Coordinate with **Hopper** to finalize IR interface and semantics
- Design and document class layout (object representation, vtable layout)
- Create `codegen/` crate with IR types and lowering infrastructure
- Implement AST → IR lowering (with type information flow)
- Implement x86-64 code generation (registers, calling convention, basic instructions)
- Add ARM64 backend in parallel

---

### Stanford Test Harness Analysis — 2026-02-27

**Source:** `/tmp/student-dist.tar.gz` (Stanford CS143 COOL distribution)

**Key Finding: No FFI Required**

The Stanford compiler uses a **Unix pipe-based architecture**:
```
./lexer file.cl | ./parser | ./semant | ./cgen > output.s
```

Each phase is a standalone executable that reads/writes text formats via stdin/stdout.

**Integration Approach:**
1. Our Rust compiler can produce **text-compatible output** at each phase
2. We can mix-and-match our phases with Stanford reference binaries for testing
3. No need to link against C++ code or use `extern "C"`

**Format Specifications Documented:**
- **Token Format:** `#<line> <TOKEN_TYPE> [value]` (line-based, simple)
- **AST Format:** Indented text with `_nodetype` prefixes and `: Type` suffixes
- **Code Output:** MIPS assembly (we'll produce x86-64/ARM64 instead)

**Reference Binaries:** 32-bit i686 Linux ELF executables in `/tmp/bin/.i686/`

**Required Work for Compatibility:**
1. Add token stream emitter to lexer crate (matching `dump_cool_token` format)
2. Add AST text emitter to parser crate (matching `dump_with_types` format)
3. Set up integration tests comparing output against reference binaries

**Full analysis:** Session workspace `c-test-harness-analysis.md`

---

### WebAssembly Backend Design — 2026-02-27

**Target Change:** From x86/ARM to WebAssembly (WASM) with WASI

**Key Architecture Decisions:**

1. **Three-Tier IR:**
   - HIR (High-Level IR): Typed AST, expression-oriented, direct from parser
   - MIR (Mid-Level IR): Stack-based CFG with basic blocks, prepares for WASM
   - LIR (Low-Level IR): Linear WASM-like instructions, 1:1 mapping to WASM bytecode

2. **Memory Model:**
   - Linear memory layout: Static data (0x0-0xFFFF) | Heap (0x10000+)
   - Arena allocator for objects (no GC initially — COOL has no explicit free)
   - Object layout: class_tag (i32) + size (i32) + vtable_ptr (i32) + attributes
   - String representation: Object with length + data_ptr

3. **Dynamic Dispatch:**
   - VTable per class stored in static memory
   - Method indices assigned during semantic analysis
   - WASM `call_indirect` instruction for dynamic method calls
   - Function table holds all functions/methods

4. **I/O Strategy: WASI (not custom imports)**
   - **Decision:** Use WASI (WebAssembly System Interface) for all I/O
   - **Rationale:** Portability (Wasmtime, Wasmer, Node.js), standard fd_read/fd_write, future-proof (Preview 2 + Component Model)
   - **Trade-off:** Slight overhead vs custom imports, but much better interop and standards compliance
   - IO methods: out_string/out_int/in_string/in_int map to WASI fd_read/fd_write

5. **Rust Tooling:**
   - **Primary:** `wasm-encoder` for low-level binary generation (precise control)
   - **Alternative:** `walrus` for higher-level AST-like API (easier prototyping)
   - Testing: Wasmtime, Wasmer, Node.js WASI

**WASI Research Findings:**
- WASI provides stdin/stdout via fd_read/fd_write (file descriptor 0/1)
- Capability-based security model (host grants access)
- WASI Preview 2 will add native string types (Component Model + WIT)
- Custom imports would require host-specific shims and manual string marshalling

**Design Document:** `.squad/agents/ritchie/wasm-backend-design.md`

**Implementation Phases:**
1. Core IR infrastructure (HIR/MIR/LIR types)
2. Memory and objects (arena allocator, layout)
3. Dynamic dispatch (vtables, call_indirect)
4. I/O and runtime (WASI integration)
5. Control flow and expressions
6. Testing and validation

**Open Questions:**
- GC strategy (defer, or use WASM GC proposal?)
- Separate runtime crate for WASI wrappers?
- Performance targets (naive first, or optimize early?)

---
