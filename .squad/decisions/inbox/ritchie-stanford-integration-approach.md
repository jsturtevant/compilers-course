# Decision: Stanford Test Harness Integration via Text Formats

**Date:** 2026-02-27  
**Proposed by:** Ritchie (Systems Developer)  
**Status:** Proposed

## Context

Investigated Stanford CS143 COOL distribution to understand how their C test harness works and how we can integrate our Rust compiler.

## Decision

**Use text-based integration via Unix pipes — no FFI required.**

The Stanford compiler phases communicate via text formats over stdin/stdout:
```
./lexer file.cl | ./parser | ./semant | ./cgen > output.s
```

Our Rust compiler should:
1. Emit text output compatible with Stanford formats at each phase
2. Accept text input in Stanford formats (optional, for hybrid testing)
3. Support mixing Rust phases with Stanford reference binaries

## Implications

### For Lexer (Grace/Frontend):
- Add `emit_stanford_tokens()` function matching their token format
- Format: `#<line> <TOKEN_TYPE> [value]`

### For Parser (Grace/Frontend):
- Add `emit_stanford_ast()` function matching their `dump_with_types` format
- Optionally: add token stream parser for reading from Stanford lexer

### For Semantic Analysis (Hopper):
- Add AST text parser for reading Stanford parser output
- Add typed AST emitter for output

### For Code Generation (Ritchie):
- No Stanford compatibility needed — we target x86-64/ARM64, not MIPS
- Can still read typed AST format if we want to use Stanford semant

### For Testing (Everyone):
- Integration tests can compare output against Stanford reference binaries
- Incremental testing: swap in one Rust phase at a time

## Alternatives Considered

1. **FFI with C++ code** — Rejected. Would require complex build setup, 32-bit compatibility
2. **Ignore Stanford format** — Rejected. Loses valuable test infrastructure
3. **Full format match** — Accepted. Simple text formats, easy to implement

## Action Items

1. [ ] Grace: Add token emitter to lexer crate
2. [ ] Grace: Add AST emitter to parser crate  
3. [ ] Scribe: Document output format specifications
4. [ ] All: Set up integration test comparing against reference binaries
