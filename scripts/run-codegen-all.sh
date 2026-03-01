#!/bin/bash
# Run codegen on all sample COOL files and validate with wasm-tools

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SAMPLES_DIR="$PROJECT_ROOT/samples"
OUTPUT_DIR="/tmp/cool-wasm-output"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Build the codegen crate first
echo "Building codegen..."
cargo build -p codegen --quiet

# Require wasm-tools for validation
if ! command -v wasm-tools &> /dev/null; then
    echo -e "${RED}Error: wasm-tools not found. Install with: cargo install wasm-tools${NC}"
    exit 1
fi

PASSED=0
FAILED=0
TOTAL=0

echo ""
echo "Running codegen on all samples..."
echo "================================="

# Files that require multiple source files
SKIP_FILES="atoi_test"

for cl_file in "$SAMPLES_DIR"/*.cl; do
    filename=$(basename "$cl_file" .cl)
    wasm_file="$OUTPUT_DIR/$filename.wasm"
    
    # Skip files that depend on other source files (tested separately)
    if echo "$SKIP_FILES" | grep -qw "$filename"; then
        continue
    fi
    
    TOTAL=$((TOTAL + 1))
    
    # Try to compile
    if cargo run -p codegen --quiet -- "$cl_file" -o "$wasm_file" 2>/dev/null; then
        # Check if wasm file was created
        if [ -f "$wasm_file" ]; then
            # Validate with wasm-tools if available
            if wasm-tools validate "$wasm_file" 2>/dev/null; then
                    echo -e "${GREEN}✓${NC} $filename.cl → valid WASM ($(wc -c < "$wasm_file") bytes)"
                    PASSED=$((PASSED + 1))
            else
                echo -e "${RED}✗${NC} $filename.cl → compiled but invalid WASM"
                FAILED=$((FAILED + 1))
            fi
        else
            echo -e "${RED}✗${NC} $filename.cl → no output file"
            FAILED=$((FAILED + 1))
        fi
    else
        echo -e "${RED}✗${NC} $filename.cl → compilation failed"
        FAILED=$((FAILED + 1))
    fi
done

echo ""
echo "================================="
echo -e "Results: ${GREEN}$PASSED passed${NC}, ${RED}$FAILED failed${NC} (out of $TOTAL)"

# Cleanup
# rm -rf "$OUTPUT_DIR"

if [ $FAILED -gt 0 ]; then
    exit 1
fi
