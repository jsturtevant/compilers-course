#!/bin/bash
# Test WASM component output mode

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

# Check for required tools
if ! command -v wasm-tools &> /dev/null; then
    echo "Error: wasm-tools not found. Install with: cargo install wasm-tools"
    exit 1
fi

if ! command -v wasmtime &> /dev/null; then
    echo "Error: wasmtime not found. Install from https://wasmtime.dev"
    exit 1
fi

echo "=== WASM Component Test ==="

# Build the compiler
echo "Building codegen..."
cargo build -p codegen --quiet

CODEGEN="$PROJECT_ROOT/target/debug/cool-wasm"
TEST_FILE="$PROJECT_ROOT/cool-support/examples/hello_world.cl"
TMP_DIR=$(mktemp -d)

cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

echo ""
echo "--- Testing module output (default) ---"
MODULE_OUT="$TMP_DIR/hello_module.wasm"
$CODEGEN "$TEST_FILE" -o "$MODULE_OUT"
echo "Generated: $MODULE_OUT ($(stat -c%s "$MODULE_OUT" 2>/dev/null || stat -f%z "$MODULE_OUT") bytes)"

echo "Validating module..."
wasm-tools validate "$MODULE_OUT"
echo "✓ Module validates"

echo "Running module..."
OUTPUT=$(wasmtime "$MODULE_OUT")
if [ "$OUTPUT" = "Hello, World." ]; then
    echo "✓ Module output correct: $OUTPUT"
else
    echo "✗ Module output incorrect: $OUTPUT (expected 'Hello, World.')"
    exit 1
fi

echo ""
echo "--- Testing component output (--component) ---"
COMPONENT_OUT="$TMP_DIR/hello_component.wasm"
$CODEGEN "$TEST_FILE" --component -o "$COMPONENT_OUT"
echo "Generated: $COMPONENT_OUT ($(stat -c%s "$COMPONENT_OUT" 2>/dev/null || stat -f%z "$COMPONENT_OUT") bytes)"

echo "Validating component..."
wasm-tools validate "$COMPONENT_OUT"
echo "✓ Component validates"

echo "Running component..."
OUTPUT=$(wasmtime "$COMPONENT_OUT")
if [ "$OUTPUT" = "Hello, World." ]; then
    echo "✓ Component output correct: $OUTPUT"
else
    echo "✗ Component output incorrect: $OUTPUT (expected 'Hello, World.')"
    exit 1
fi

echo ""
echo "=== All component tests passed ==="
