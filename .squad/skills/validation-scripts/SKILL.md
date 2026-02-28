# Skill: Validation Scripts

**Confidence:** high  
**Domain:** compiler phases, testing, CI

## Pattern

Every compiler phase MUST have a corresponding validation script in `scripts/` that:

1. Runs the phase against ALL sample files in `cool-support/examples/`
2. Requires any external validation tools (exits with error if missing)
3. Reports pass/fail counts with colored output
4. Exits with non-zero status if any sample fails

## Script Naming Convention

```
scripts/run-{phase}-all.sh
```

Examples:
- `scripts/run-lexer-all.sh`
- `scripts/run-parser-all.sh`
- `scripts/run-semant-all.sh`
- `scripts/run-codegen-all.sh`

## Required Script Structure

```bash
#!/bin/bash
set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

# Require external tools (fail fast, don't warn)
if ! command -v {tool} &> /dev/null; then
    echo -e "${RED}Error: {tool} not found. Install with: {install_cmd}${NC}"
    exit 1
fi

# Build first
cargo build -p {crate} --quiet

# Run against all samples
PASSED=0
FAILED=0

for cl_file in cool-support/examples/*.cl; do
    # ... run phase and validate ...
    if [ success ]; then
        echo -e "${GREEN}✓${NC} $filename"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}✗${NC} $filename"
        FAILED=$((FAILED + 1))
    fi
done

echo "Results: $PASSED passed, $FAILED failed"

if [ $FAILED -gt 0 ]; then
    exit 1
fi
```

## When to Create

Create a validation script when:
- Implementing a new compiler phase
- Adding a new crate that processes COOL source files
- Before marking a phase as "complete"

## When to Run

- After implementing changes to a phase
- Before committing code
- In CI pipelines

## Current Scripts

| Phase | Script | External Tool | Validates |
|-------|--------|---------------|-----------|
| Lexer | `run-lexer-all.sh` | — | Token output |
| Parser | `run-parser-all.sh` | — | AST structure |
| Semantic | `run-semant-all.sh` | — | Type checking |
| Codegen | `run-codegen-all.sh` | `wasm-tools` | WASM structure |
| Runtime | `run-wasmtime-all.sh` | `wasmtime` | WASM execution |
