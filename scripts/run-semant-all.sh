#!/bin/bash
# Run semantic analysis on all COOL examples

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
EXAMPLES_DIR="$PROJECT_ROOT/samples"

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo "Building semant..."
cargo build -p semant --quiet

passed=0
failed=0
total=0

# Files that require multiple source files (depend on other .cl files)
SKIP_FILES="atoi_test.cl"

for file in "$EXAMPLES_DIR"/*.cl; do
    filename=$(basename "$file")
    
    # Skip files that depend on other source files
    if echo "$SKIP_FILES" | grep -qw "$filename"; then
        continue
    fi
    
    total=$((total + 1))
    
    if cargo run -p semant --quiet -- "$file" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} $filename"
        passed=$((passed + 1))
    else
        echo -e "${RED}✗${NC} $filename"
        # Show the actual error for debugging
        cargo run -p semant --quiet -- "$file" 2>&1 | head -5 | sed 's/^/  /'
        failed=$((failed + 1))
    fi
done

echo ""
echo "================================"
echo -e "Results: ${GREEN}$passed passed${NC}, ${RED}$failed failed${NC} out of $total total"

if [ $failed -eq 0 ]; then
    echo -e "${GREEN}All semantic checks passed!${NC}"
    exit 0
else
    echo -e "${YELLOW}Some files failed semantic analysis${NC}"
    exit 1
fi
