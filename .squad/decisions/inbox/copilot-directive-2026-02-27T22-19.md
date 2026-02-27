### 2026-02-27T22:19: User directive
**By:** James Sturtevant (via Copilot)
**What:** Stanford COOL distribution files (cool-support/) should NOT be checked into git. Add to .gitignore instead. Keep filesystem read-only protection and documentation, but don't version control the reference files.
**Why:** User request — keeps repo size small, reference files are external dependency
