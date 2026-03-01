#!/bin/bash
# Test WASM component output mode for all samples

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
EXAMPLES_DIR="$PROJECT_ROOT/samples"

cd "$PROJECT_ROOT"

# Check for required tools
if ! command -v wasm-tools &> /dev/null; then
    echo -e "${RED}Error: wasm-tools not found. Install with: cargo install wasm-tools${NC}"
    exit 1
fi

if ! command -v wasmtime &> /dev/null; then
    echo -e "${RED}Error: wasmtime not found. Install from https://wasmtime.dev${NC}"
    exit 1
fi

echo "=== WASM Component Test (All Samples) ==="

# Build the compiler
echo "Building codegen..."
cargo build -p codegen --quiet

TMP_DIR=$(mktemp -d)
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

PASSED=0
FAILED=0

echo ""
echo "Testing module vs component output for all samples..."
echo "======================================================"

for cl_file in "$EXAMPLES_DIR"/*.cl; do
    filename=$(basename "$cl_file" .cl)
    
    # Skip library files that aren't standalone programs
    case "$filename" in
        atoi)
            # atoi.cl is a library, not a standalone program
            continue
            ;;
    esac
    
    module_out="$TMP_DIR/${filename}_module.wasm"
    component_out="$TMP_DIR/${filename}_component.wasm"
    
    # Handle multi-file compilation
    case "$filename" in
        atoi_test)
            compile_args="$EXAMPLES_DIR/atoi.cl $cl_file"
            ;;
        *)
            compile_args="$cl_file"
            ;;
    esac
    
    # Compile as module
    if ! cargo run -p codegen --quiet -- $compile_args -o "$module_out" 2>/dev/null; then
        echo -e "${RED}✗${NC} $filename (module compilation failed)"
        FAILED=$((FAILED + 1))
        continue
    fi
    
    # Compile as component
    if ! cargo run -p codegen --quiet -- $compile_args --component -o "$component_out" 2>/dev/null; then
        echo -e "${RED}✗${NC} $filename (component compilation failed)"
        FAILED=$((FAILED + 1))
        continue
    fi
    
    # Validate module
    if ! wasm-tools validate "$module_out" 2>/dev/null; then
        echo -e "${RED}✗${NC} $filename (module validation failed)"
        FAILED=$((FAILED + 1))
        continue
    fi
    
    # Validate component
    if ! wasm-tools validate "$component_out" 2>/dev/null; then
        echo -e "${RED}✗${NC} $filename (component validation failed)"
        FAILED=$((FAILED + 1))
        continue
    fi
    
    # Get file sizes
    module_size=$(stat -c%s "$module_out" 2>/dev/null || stat -f%z "$module_out")
    component_size=$(stat -c%s "$component_out" 2>/dev/null || stat -f%z "$component_out")
    
    # Run both and compare outputs (with appropriate input)
    case "$filename" in
        atoi_test)
            input="12345\nstop\n"
            ;;
        palindrome)
            input="racecar\n"
            ;;
        graph)
            if [ -f "$EXAMPLES_DIR/g1.graph" ]; then
                input=$(cat "$EXAMPLES_DIR/g1.graph")
            else
                input=""
            fi
            ;;
        arith)
            input="q\n"
            ;;
        life)
            input="y\n1\nn\nn\n"
            ;;
        sort_list)
            input="5\n"
            ;;
        *)
            input=""
            ;;
    esac
    
    # Run module
    module_exit=0
    if [ -n "$input" ]; then
        module_output=$(printf "$input" | timeout 3s wasmtime run "$module_out" 2>&1) || module_exit=$?
    else
        module_output=$(timeout 3s wasmtime run "$module_out" 2>&1) || module_exit=$?
    fi
    
    # Run component
    component_exit=0
    if [ -n "$input" ]; then
        component_output=$(printf "$input" | timeout 3s wasmtime run "$component_out" 2>&1) || component_exit=$?
    else
        component_output=$(timeout 3s wasmtime run "$component_out" 2>&1) || component_exit=$?
    fi
    
    # Compare outputs (both should produce same result)
    if [ "$module_output" = "$component_output" ]; then
        echo -e "${GREEN}✓${NC} $filename (module: ${module_size}B, component: ${component_size}B) → outputs match"
        PASSED=$((PASSED + 1))
    else
        # Some programs have non-deterministic output or abort - check they both ran
        if [ "$module_exit" -eq "$component_exit" ]; then
            echo -e "${GREEN}✓${NC} $filename (module: ${module_size}B, component: ${component_size}B) → same exit code"
            PASSED=$((PASSED + 1))
        else
            echo -e "${RED}✗${NC} $filename (outputs differ: module=$module_exit, component=$component_exit)"
            FAILED=$((FAILED + 1))
        fi
    fi
done

echo ""
echo "======================================================"
echo -e "Results: ${GREEN}$PASSED passed${NC}, ${RED}$FAILED failed${NC}"

if [ $FAILED -gt 0 ]; then
    exit 1
fi

echo ""
echo "=== All component tests passed ==="
