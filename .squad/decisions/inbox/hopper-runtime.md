# Runtime Memory and Object Implementation

**Decision Date:** 2026-02-27  
**Author:** Hopper (Compiler Dev)  
**Status:** Implemented  
**Impact:** Core runtime functionality

## Context

Phase 4 of the COOL→WASM compiler required implementing the memory allocator and runtime methods for Object and String classes. The previous implementation had only stubs.

## Decision

### Memory Allocator

Implemented a **bump allocator** with the following characteristics:
- **Global heap pointer** starting at 1KB (0x400) to avoid collision with static data
- **4-byte alignment** for all allocations: `aligned_size = (size + 3) & ~3`
- **No garbage collection** - arena-style allocation (matches COOL semantics)
- Simple and efficient: O(1) allocation time

### Object Layout

Standardized object header format:
```
offset 0:  class_tag (i32)     - for type dispatch
offset 4:  size (i32)          - total object size in bytes
offset 8:  vtable_ptr (i32)    - pointer to method table
offset 12+: attributes         - class-specific fields
```

This layout enables:
- Fast type checking (read class_tag)
- Object copying (read size, memcpy entire object)
- Method dispatch (vtable indirection)

### String Representation

Strings are objects with **inline data** (not pointer-based):
```
offset 0-11: Object header (class_tag, size, vtable_ptr)
offset 12:   length (i32)
offset 16+:  UTF-8 bytes (inline)
```

**Rationale:** Inline data avoids double indirection and simplifies allocation. Trade-off: larger minimum allocation (16 bytes + data vs potential pointer sharing).

### Object Methods

- **`abort()`**: Direct `unreachable` trap - no error message (can add WASI output later)
- **`type_name()`**: Stub returning null - deferred until class metadata table implemented
- **`copy()`**: Full shallow copy via word-by-word memcpy loop

### String Methods

All fully implemented:
- **`length()`**: Direct memory load from offset 12
- **`concat()`**: Allocate new string, copy both payloads byte-by-byte
- **`substr(i, l)`**: Allocate substring, copy slice of source data

**Bounds checking**: Simplified for MVP - assumes valid input. LIR/semantic analysis should prevent invalid indices.

## Alternatives Considered

### Allocator Alternatives

1. **Free list allocator** - Too complex for COOL (no explicit free)
2. **Reference counting** - Runtime overhead, doesn't handle cycles
3. **Mark-and-sweep GC** - Future enhancement, not needed for correctness

**Chosen:** Bump allocator - simplest, matches COOL semantics (no deallocation)

### String Representation Alternatives

1. **Pointer to separate data buffer** - More flexible, enables string sharing
2. **Inline small strings, pointer for large** - Optimization complexity
3. **UTF-16 encoding** - COOL spec uses ASCII/UTF-8

**Chosen:** Inline data - simpler implementation, sufficient for coursework

### Memory Layout Alternatives

1. **Start heap at 0** - Risk of null pointer confusion
2. **Start at 64KB** - Wastes address space
3. **Dynamic sizing** - Requires memory.grow tracking

**Chosen:** 1KB start - leaves room for ~1KB static data (vtables, string constants)

## Consequences

### Positive

- ✅ All 18 sample programs compile to valid WASM
- ✅ Runtime methods ready for code generation
- ✅ Simple, predictable allocator behavior
- ✅ No memory leaks (arena never frees)

### Negative

- ❌ No garbage collection - memory grows monotonically
- ❌ String concat creates copies (no sharing)
- ❌ `type_name()` not fully implemented (needs metadata)

### Future Work

- **Garbage collection**: Add mark-and-sweep or reference counting
- **String interning**: Share identical string literals
- **Bounds checking**: Add runtime checks for substr/string indexing
- **Class metadata table**: Enable `type_name()` implementation
- **Memory.grow**: Handle heap exhaustion gracefully

## Implementation Notes

### Function Indexing

Runtime functions occupy indices 0-12:
- 0-1: WASI imports (fd_write, fd_read)
- 2: alloc
- 3-5: Object methods (abort, type_name, copy)
- 6-9: IO methods (out_string, out_int, in_string, in_int)
- 10-12: String methods (length, concat, substr)

Program functions start at index 13.

### WASM Specifics

- Global 0: `heap_ptr` (mut i32)
- Memory: 16 pages (1MB) initial allocation
- No imports beyond WASI fd_read/fd_write

## Validation

```bash
cargo test -p codegen        # ✅ 5/5 tests pass
./scripts/run-codegen-all.sh # ✅ 18/18 samples valid WASM
wasm-tools validate *.wasm   # ✅ All pass
```

## Team Coordination

- **Ritchie**: IO methods (fd_read/fd_write) - stubs remain, awaiting WASI implementation
- **Hopper**: Allocator and Object/String methods - complete
- **Integration**: AST→HIR pass can now rely on functional runtime

---

**Status Update:** Runtime core complete. IO implementation blocked on Ritchie's WASI integration.
