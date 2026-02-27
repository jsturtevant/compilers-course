# Decisions

Team decisions are recorded here. Append-only.

---

## 2026-02-27: Team Formation

**By:** Squad (Coordinator)
**What:** Initial team formed with 5 active members + Scribe + Ralph
**Why:** User requested compiler team with Rust expertise and x86/ARM backend capability
**Decision:** No dedicated tester — testing is everyone's responsibility

---

## 2026-02-27: Stanford Distribution Not Version-Controlled

**By:** James Sturtevant (User Directive)
**What:** Stanford COOL course distribution stored in `cool-support/` but NOT checked into git
**Why:** Keeps repo small; avoids licensing concerns; each developer extracts locally
**Implementation:**
- `cool-support/` added to `.gitignore`
- README in `cool-support/` with extraction instructions
- Files set to read-only (chmod 444) for protection

---

## 2026-02-27: Text-Based Integration with Stanford Test Harness

**By:** Ritchie (Systems Developer)
**What:** Integrate with Stanford test harness via text formats, not FFI
**Why:** Stanford compiler phases communicate via Unix pipes with text on stdin/stdout
**Decision:** 
- Add `--stanford` output format to lexer and parser
- No C++ linking or `extern "C"` required
- Compare our output against reference binaries in integration tests

---
