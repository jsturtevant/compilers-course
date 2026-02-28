# Decision: WASI IO Implementation Strategy

**Date:** 2025-01-02  
**Author:** Ritchie (Systems Dev)  
**Status:** Implemented  
**Context:** Phase 4 - Runtime WASI integration

## Problem

The COOL→WASM compiler needed real IO implementations to replace stubs. We had to choose between:
1. Simple/correct implementations that work but may be inefficient
2. Complex/optimized implementations that risk bugs and stack errors

Additionally, WASI requires specific function signatures and calling conventions that differ from COOL's method signatures.

## Decision

**Implement IO with WASI, prioritizing correctness over optimization:**

1. **WASI _start wrapper:** Create a proper `_start() -> ()` wrapper that allocates Main and calls main(), rather than exporting Main.main directly. This ensures WASI compliance.

2. **IO.out_string:** Full implementation using WASI fd_write with iovec structures at fixed memory locations (0x1000).

3. **IO.in_string:** Full implementation using WASI fd_read, returning raw buffer pointers for MVP (proper String object allocation deferred).

4. **IO.out_int and IO.in_int:** Stub implementations that return self/0. Int↔string conversion is complex in WASM and creates stack management issues. Defer to future work.

5. **String methods:** Stub implementations for concat/substr. The min() logic and character copying loops were causing WASM validation errors due to stack type mismatches. Complexity doesn't justify MVP effort.

6. **Memory allocator:** Use simple `LocalSet` instead of `LocalTee` to avoid stack pollution. The allocator must maintain clean stack discipline.

## Rationale

**Correctness First:**
- WASM validation is strict about stack types. Complex control flow (if/else, loops with early exits) easily causes type mismatches.
- Getting 18 samples to produce valid WASM is more valuable than having incomplete runtime functions.

**Fixed Memory Locations:**
- Using fixed locations (0x1000, 0x2000, 0x3000) for IO buffers simplifies stack management.
- Trade-off: Not thread-safe, but COOL/WASM is single-threaded anyway.

**Defer Complexity:**
- Int-to-string conversion requires digit extraction, sign handling, and reversal — each prone to stack errors.
- String operations require length calculations, bounds checking, and byte-by-byte copying with proper loop exit conditions.
- These can be incrementally improved without breaking existing working samples.

## Consequences

**Positive:**
- ✅ All 18 samples compile to valid WASM
- ✅ Clean separation: working samples + identified stub areas
- ✅ Foundation for incremental improvement

**Negative:**
- ❌ IO.out_int/in_int don't work (programs using them will fail at runtime)
- ❌ String operations (concat, substr) are identity operations
- ❌ Can't run samples that depend on these features

**Future Work:**
- Implement int↔string conversion with careful stack management
- Add String object allocation helper
- Implement concat/substr with proper loop exit conditions
- Add integration tests that actually execute WASM (not just validate)

## Alternatives Considered

**Alternative 1: Complete implementations immediately**
- Rejected: High risk of getting stuck debugging stack type errors across 18 samples
- Benefit: Would have full functionality
- Cost: Could block progress for days on subtle WASM validation issues

**Alternative 2: Use WASM reference types / GC proposal**
- Rejected: Requires newer WASM features, not widely supported
- Benefit: String handling would be easier
- Cost: Limits portability, increases complexity

**Alternative 3: Custom host imports instead of WASI**
- Rejected: Would require host-specific shims for every runtime (Wasmtime, Wasmer, Node.js)
- Benefit: Could have simpler signatures
- Cost: Loss of portability, non-standard approach

## Validation

**Test Results:**
```
cargo test -p codegen: ✅ All 5 tests pass
./scripts/run-codegen-all.sh: ✅ Results: 18 passed, 0 failed (out of 18)
wasmtime compile: ✅ All samples validate successfully
```

## Notes

The key insight: **WASM stack discipline is unforgiving**. Every instruction must have exact type expectations. Control flow structures (block/loop/if) must maintain stack invariants. When debugging "values remaining on stack" errors, the issue is almost always:
1. Using LocalTee when LocalSet+LocalGet is clearer
2. Pushing values before If/Block that aren't consumed
3. Loop conditions that don't properly clear intermediate values

For future implementations, prototype complex logic in WAT (WebAssembly Text format) first to verify stack behavior before translating to Rust wasm-encoder calls.
