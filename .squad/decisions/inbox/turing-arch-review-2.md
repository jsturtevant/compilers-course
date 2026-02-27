# Architecture Review: Parser Maturity & Phase Sequencing

**Date:** 2026-02-27 (Evening)  
**By:** Turing (Lead/Architect)  
**Status:** FINAL RECOMMENDATIONS for team action

## Summary

Parser is **functionally complete but undertested**. All 29 BNF productions implemented. Lexer solid. **Critical path blocker: IR specification.** Team readiness assessment shows clear phase dependencies.

---

## Parser Status: Complete AST, No Tests

**Fact:** Parser can successfully parse all sample files (hello_world.cl, arith.cl, etc.). Zero unit tests exist.

**Risk:** Regression will be undetected. Parser combinators are powerful but fragile without test discipline.

**Action Required:**
- Hopper must add BNF-per-production unit tests (at least 1 test per grammar rule)
- Parser error recovery tests (verify graceful failure modes)
- Integration tests using samples/

**Priority:** HIGH. Complete before semantic analysis begins.

---

## IR Specification Is Sequence Blocker

Ritchie has correctly identified (inbox/ritchie-ir-design-needed.md) that **codegen cannot begin without IR agreement**.

**Current State:**
- No IR defined
- No AST→IR lowering pass
- Codegen work stalled

**Decision Affirmed:** CFG with basic blocks
- Statement-oriented IR (not expression-oriented)
- Preserve class structure until lowering phase
- Basic blocks for control flow representation
- Type information preserved in IR (needed for dispatch codegen)

**Next Step:** Hopper + Ritchie must pair-program IR specification document:
- Type signatures for IR node types
- Lowering rules from AST→IR for each expression form
- Memory model (stack frame layout, register assumptions)
- Example: Lower `expr.method(args)` → dispatch IR node + vtable lookup code

**Timeline:** 1-2 days for spec, then Ritchie implements lowering.

---

## Phase Sequencing: Clear Dependencies

```
Parser (DONE) ──┐
                ├──→ Semantic Analysis + Symbol Table ──→ IR Generation ──┐
                │                                                          │
Parser Tests    │    (Hopper lead)                  (Hopper + Ritchie)   │
(CRITICAL)  ────┘                                                        │
                                                                         │
                        x86-64 Codegen ←─ Register Allocation ←─────────┘
                        ARM64 Codegen   (Ritchie lead)
```

**Blocking Order:**
1. Parser unit tests (Hopper) — 3-5 days
2. Semantic analysis skeleton (Hopper) — symbol table API, type environment
3. IR spec + lowering (Hopper + Ritchie) — 2-3 days pairing
4. IR implementation (Ritchie) — AST→IR lowering
5. Codegen (Ritchie) — x86-64 then ARM64

---

## Sample Programs Validate Full Complexity

**Arith.cl (260 LOC):** Tests inheritance, method overriding, dispatch, arithmetic, nested let/while/if. **Excellent integration test.**

**Life.cl (300 LOC):** Cellular automaton. Tests memory management, 2D grid logic, loops. **Stress test.**

**Recommendation:** Define end-to-end test harness where:
1. Parse sample → AST (already works)
2. Semantic check → Type-checked AST
3. Lower → IR
4. Codegen → Assembly
5. Assemble + link → Binary
6. Execute → verify output

Each phase produces stable output format (Debug impl or custom Display) for golden file diffs.

---

## Standard Library: Synthesize, Don't Hand-Code

**Decision:** Compiler synthesizes Object, IO, String, Int, Bool as built-in classes during semantic analysis.

**Why:**
- Avoids code duplication (would need asm for x86 + ARM)
- Single source of truth (class definitions consistent across backends)
- Easier to test (semantic analyzer can validate built-in classes like any other)

**Implementation:** Semantic analyzer injects class definitions into symbol table before traversing user code. Methods like `IO::out_string` are compiler-known (link to runtime stubs at assembly time).

---

## Type Environment & Symbol Table Design

**Recommendation:** Scope stack with trait-based registry.

**Structure:**
```rust
pub trait Symbol {
    fn name(&self) -> &str;
    fn kind(&self) -> SymbolKind;
}

pub struct Scope {
    bindings: HashMap<String, Box<dyn Symbol>>,
    parent: Option<Box<Scope>>,  // parent scope (supports shadowing)
}

pub struct TypeEnvironment {
    scopes: Vec<Scope>,  // stack
}
```

**Enables:**
- Efficient parent-scope lookup (recursive walk)
- Shadowing detection (same name at different scope levels)
- Extension for optimization passes (trait-based, add new Symbol kinds without refactoring)

**Hopper responsibility:** Implement TypeEnvironment, use during semantic analysis.

---

## Calling Convention Clarity

**x86-64 System V ABI (Linux/Unix):**
- Parameter passing: RDI, RSI, RDX, RCX, R8, R9 (integers), XMM0-7 (floats)
- Return: RAX, RDX (or RDX:RAX for 128-bit)
- Caller-saved: RAX, RCX, RDX, RSI, RDI, R8-11
- Callee-saved: RBX, RSP, RBP, R12-15

**ARM64 EABI:**
- Parameter passing: X0-X7 (integers)
- Return: X0, X1
- Caller-saved: X0-X18
- Callee-saved: X19-X28

**Action:** Document both in codegen module. Use abstraction layer for register allocation so both backends can reuse logic.

---

## Workspace Structure Ready

Current workspace is clean for adding new crates:
```
├── lexer/
├── parser/
├── ir/              ← New crate (IR types + lowering)
└── codegen/         ← New crate (x86/ARM backends)
```

No restructuring needed. Dependencies flow: lexer → parser → ir → codegen.

---

## Quality Gates (Before Merging Each Phase)

1. **Parser Tests:** All 29 BNF rules have ≥1 test
2. **Semantic Analysis:** Type checker passes on arith.cl + life.cl
3. **IR Generation:** IR output matches expected form (golden files)
4. **Codegen:** Generated assembly assembles without errors, basic execution tests pass

---

## Action Summary

| Task | Owner | Priority | Blockers |
|------|-------|----------|----------|
| Parser unit tests | Hopper | HIGH | None—start now |
| IR specification | Hopper + Ritchie pair | HIGH | None—parallel to tests |
| Semantic analyzer skeleton | Hopper | HIGH | Parser tests done |
| Symbol table implementation | Hopper | MEDIUM | Semantic design done |
| IR lowering implementation | Ritchie | MEDIUM | IR spec locked |
| x86-64 codegen | Ritchie | MEDIUM | IR implementation done |
| ARM64 codegen | Ritchie | MEDIUM | x86 codegen foundation laid |
| Documentation (architecture + IR) | Knuth | MEDIUM | IR spec from Hopper |

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Parser regression (no tests) | Medium | Add tests immediately (Hopper) |
| IR spec delays codegen | High | Pair programming + 2-day timebox |
| Standard library misalignment | Medium | Synthesize built-ins, test thoroughly |
| Calling convention bugs | High | Document clearly, align Ritchie + system libs early |
| Test harness complexity | Medium | Phase-by-phase output validation (parse→AST, analyze→Types, lower→IR, codegen→ASM) |

---

## Next Checkpoint

1. Hopper starts parser test suite
2. Hopper + Ritchie schedule IR spec pairing session (2-3 hours)
3. Ritchie reviews team decisions + backend readiness
4. Knuth drafts IR documentation skeleton
5. Daily standup on phase progress

---
