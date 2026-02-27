# Compiler Architecture Roadmap

**Date:** 2026-02-27  
**By:** Turing (Lead/Architect)  
**Status:** For team discussion

## Overview
After exploring the COOL compiler project, five architectural decisions need to be made before scaling beyond the parser phase.

## Decisions Required

### 1. Symbol Table & Type Environment Design
**Issue:** Semantic analysis requires persistent, queryable symbol storage across multiple passes.  
**Options:**
- A. Single global HashMap<String, Symbol> rebuilt per pass (simple, but inefficient for large programs)
- B. Persistent tree structure (scopes nested in AST-like hierarchy, supports shadowing)
- C. Trait-based registry system (extensible for future optimization passes)

**Recommendation:** Option B—use a scope stack during AST traversal. Enables efficient parent-scope lookup, shadowing, and integration with IR phases.

### 2. Intermediate Representation (IR) Design
**Issue:** Bridging high-level COOL AST to low-level x86/ARM requires an IR abstraction.  
**Candidates:**
- 3-address code (simple, good for teaching)
- Control Flow Graph (CFG) with basic blocks
- LLVM-like IR (powerful but complex)

**Recommendation:** CFG with basic blocks. Structured enough for pattern matching and optimization, simple enough for codegen targeting.

### 3. COOL Standard Library Implementation
**Issue:** COOL depends on Object, IO, String, Main classes. How are these provided?  
**Options:**
- A. Hand-written x86/ARM assembly stubs for each backend
- B. Generated from COOL source (requires metacircular definition)
- C. Compiler-synthesized (inject class definitions during semantic analysis)

**Recommendation:** Option C. Compiler synthesizes Object, IO, String, Int, Bool as built-in classes. Avoids code duplication per backend; handled at semantic analysis time.

### 4. Calling Convention & ABI
**Issue:** Must align with target OS for linking. Different per architecture.  
**Targets:**
- x86-64: System V AMD64 ABI (Unix) or x64 ABI (Windows)
- ARM64: ARM64 EABI

**Recommendation:** Start with x86-64 System V ABI (most common). Document parameter passing, return registers, and stack layout in codegen module. ARM64 can follow with analogous rules.

### 5. Test Harness Architecture
**Issue:** Currently only parser has tests. Semantic + codegen phases lack test infrastructure.  
**Approach:**
- End-to-end test: source file → compiled binary → run → check output
- Unit tests: parser output → semantic output → IR output
- Golden file diffs for AST/IR validation

**Recommendation:** Add integration test framework. Each test is a source file + expected output. Phases produce stable output format (Debug or custom Display) for diff comparison.

## Action Items
- [ ] Knuth: Update architecture docs with IR design (CFG with basic blocks)
- [ ] Hopper: Complete parser to cover all BNF rules
- [ ] Hopper: Design symbol table API for semantic analyzer
- [ ] Ritchie: Prototype x86-64 codegen skeleton (register allocation, calling convention)
- [ ] Stroustrup: Code review parser completion + symbol table design

## Dependencies
Parser must be complete before semantic analysis. Semantic analysis informs IR design. IR design informs codegen.
