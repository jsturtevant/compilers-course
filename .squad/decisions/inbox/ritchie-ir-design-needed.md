# Decision: IR Design is Critical Path Blocker

**Raised By:** Ritchie (Systems Developer)
**Date:** 2026-02-27
**Status:** PENDING (awaiting Hopper input)

## Context
Backend codegen cannot begin without agreement on the Intermediate Representation (IR) interface.

## The Question
Before implementing x86 and ARM code generation, **Ritchie and Hopper must agree on:**

1. **Form of IR** — expression-based, statement-based, or three-address code?
2. **Scope** — does IR preserve class structure, or is everything flattened?
3. **Memory Model** — register assumptions, stack frame layout, allocation strategy?
4. **Type System** — does IR include type info for dispatch codegen?
5. **Ownership** — who owns IR definition and implementation?

## Proposed Path
- Hopper defines IR type (likely in a new `ir/` crate, or shared module in parser)
- Ritchie implements AST → IR lowering pass
- Ritchie uses IR as source for both x86 and ARM backends

## Why It Matters
- **Blocked:** x86 and ARM instruction selection cannot start until lowering is defined
- **Risk:** Without shared IR, frontend and backend become tightly coupled
- **Quality:** Good IR enables cleaner register allocation, optimization, and dual-target support

## Next Steps
1. Ritchie schedules pairing with Hopper to design IR
2. Document IR spec (types, operations, invariants)
3. Implement IR crate with lowering pass
4. Unblock backend implementation

---
