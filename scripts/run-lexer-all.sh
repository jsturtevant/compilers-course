#!/usr/bin/env bash
# Run our lexer against all COOL examples

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
EXAMPLES_DIR="$ROOT_DIR/samples"

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

passed=0
failed=0

echo "Running lexer on all examples..."
echo "================================"

for cl_file in "$EXAMPLES_DIR"/*.cl; do
    name=$(basename "$cl_file" .cl)
    if cargo run -p lexer --quiet -- "$cl_file" 2>/dev/null; then
        echo -e "${GREEN}✓${NC} $name"
        ((passed++)) || true
    else
        echo -e "${RED}✗${NC} $name"
        ((failed++)) || true
    fi
done

echo "================================"
echo "Passed: $passed  Failed: $failed"
