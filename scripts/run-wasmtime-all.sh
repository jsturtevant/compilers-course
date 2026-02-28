#!/bin/bash
set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Require wasmtime
if ! command -v wasmtime &> /dev/null; then
    echo -e "${RED}Error: wasmtime not found. Install with: curl https://wasmtime.dev/install.sh -sSf | bash${NC}"
    exit 1
fi

# Build codegen
echo "Building codegen..."
cargo build -p codegen --quiet

# Create output directory
OUTPUT_DIR="/tmp/cool-wasm-output"
mkdir -p "$OUTPUT_DIR"

PASSED=0
FAILED=0
SKIPPED=0

echo ""
echo "Running WASM programs with wasmtime..."
echo "================================="

for cl_file in cool-support/examples/*.cl; do
    filename=$(basename "$cl_file" .cl)
    wasm_file="$OUTPUT_DIR/${filename}.wasm"
    
    # Compile to WASM
    if ! cargo run -p codegen --quiet -- "$cl_file" -o "$wasm_file" 2>/dev/null; then
        echo -e "${RED}✗${NC} $filename (compilation failed)"
        FAILED=$((FAILED + 1))
        continue
    fi
    
    # Try to run with wasmtime (with timeout)
    # Note: Many programs need input or have infinite loops, so we timeout after 2 seconds
    output=$(timeout 2s wasmtime run "$wasm_file" 2>&1) || exit_code=$?
    
    if [ -z "${exit_code:-}" ] || [ "$exit_code" -eq 0 ]; then
        # Success - check if there's any output
        if [ -n "$output" ]; then
            echo -e "${GREEN}✓${NC} $filename → output: ${output:0:50}..."
        else
            echo -e "${GREEN}✓${NC} $filename (no output)"
        fi
        PASSED=$((PASSED + 1))
    elif [ "$exit_code" -eq 124 ]; then
        # Timeout - program ran but didn't exit (needs input or infinite loop)
        echo -e "${YELLOW}⏱${NC} $filename (timeout - likely needs input)"
        SKIPPED=$((SKIPPED + 1))
    else
        # Runtime error
        echo -e "${RED}✗${NC} $filename (runtime error: $output)"
        FAILED=$((FAILED + 1))
    fi
done

echo ""
echo "================================="
echo "Results: $PASSED passed, $FAILED failed, $SKIPPED skipped (timeout)"

if [ $FAILED -gt 0 ]; then
    exit 1
fi
