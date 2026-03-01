#!/usr/bin/env bash
#
# validate-wasm.sh
#
# Validates COOL sample programs using our WASM codegen.
# Compiles .cl files to .wasm and validates them.
#
# Usage: ./scripts/validate-wasm.sh
# Or:    ./scripts/validate-wasm.sh samples/arith.cl

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
SAMPLES_DIR="$ROOT_DIR/samples"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[INFO]${NC} $*"
}

error() {
    echo -e "${RED}[ERROR]${NC} $*" >&2
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

success() {
    echo -e "${GREEN}[✓]${NC} $*"
}

fail() {
    echo -e "${RED}[✗]${NC} $*"
}

# Check if required tools exist
check_prerequisites() {
    if ! command -v wasm-tools &>/dev/null; then
        error "wasm-tools not found. Install with: cargo install wasm-tools"
        exit 1
    fi
    
    if ! command -v wasmtime &>/dev/null; then
        error "wasmtime not found. Install from: https://wasmtime.dev"
        exit 1
    fi
}

# Compile and validate a COOL program
validate_program() {
    local cool_file="$1"
    local basename="$(basename "$cool_file" .cl)"
    local wasm_file="/tmp/${basename}.wasm"
    
    # Compile
    if ! cargo run --quiet --bin cool-wasm -- "$cool_file" -o "$wasm_file" 2>/dev/null; then
        fail "$basename: Compilation failed"
        return 1
    fi
    
    # Validate
    if ! wasm-tools validate "$wasm_file" 2>/dev/null; then
        fail "$basename: WASM validation failed"
        return 1
    fi
    
    success "$basename: Compiled and validated"
    return 0
}

main() {
    log "COOL to WASM Validation Script"
    log "==============================="
    
    check_prerequisites
    
    # Build the compiler first
    log "Building compiler..."
    if ! cargo build --quiet --bin cool-wasm 2>/dev/null; then
        error "Failed to build compiler"
        exit 1
    fi
    
    local total=0
    local success=0
    local failed=0
    
    if [[ $# -gt 0 ]]; then
        # Validate specific files
        for cool_file in "$@"; do
            total=$((total + 1))
            if validate_program "$cool_file"; then
                success=$((success + 1))
            else
                failed=$((failed + 1))
            fi
        done
    else
        # Validate all examples
        while IFS= read -r -d '' cool_file; do
            total=$((total + 1))
            if validate_program "$cool_file"; then
                success=$((success + 1))
            else
                failed=$((failed + 1))
            fi
        done < <(find "$SAMPLES_DIR" -name "*.cl" -type f -print0 | sort -z)
    fi
    
    # Summary
    echo ""
    log "==============================="
    log "Validation Summary"
    log "==============================="
    log "Total programs: $total"
    log "Successful: $success"
    if [[ $failed -gt 0 ]]; then
        warn "Failed: $failed"
        exit 1
    fi
}

main "$@"
