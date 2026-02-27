# Decision: Fix Rust Edition to 2021

**Date:** 2025-02-27  
**By:** Stroustrup (Rust Expert)  
**Status:** ACTION REQUIRED

## Issue

Both `lexer/Cargo.toml` and `parser/Cargo.toml` specify:
```toml
edition = "2024"
```

**Problem:** Edition 2024 does not exist. Rust 1.85 (current) only supports editions 2015, 2018, and 2021 (latest).

This is a **configuration error** that will prevent the project from building on any standard Rust installation.

## Recommendation

Change both files to:
```toml
edition = "2021"
```

This is the latest stable Rust edition and provides:
- Better error messages
- Disjoint closure captures (borrow checker improvement)
- Const generics support
- Standard library ergonomic improvements

## Impact

- **Breaking:** No. Edition 2021 is backward-compatible with 2018 code
- **Testing:** Run `cargo build && cargo test` after fix
- **Risk:** Very low—this is a configuration correction, not a code change

## Next Steps

1. Update both Cargo.toml files to `edition = "2021"`
2. Verify `cargo build` and `cargo test` pass
3. Consider documenting MSRV (Minimum Supported Rust Version) in README once stable

---
