#!/usr/bin/env bash
#
# validate-samples.sh
#
# Validates COOL sample programs using Stanford's reference compiler.
# Creates a baseline of expected outputs for each program.
#
# Usage: ./scripts/validate-samples.sh
# Output: scripts/expected-outputs/ directory with .output files

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
COOL_SUPPORT="$ROOT_DIR/cool-support"
EXAMPLES_DIR="$COOL_SUPPORT/examples"
OUTPUT_DIR="$SCRIPT_DIR/expected-outputs"
WORK_DIR="/tmp/cool-validation-$$"

# Stanford binaries
COOLC="$COOL_SUPPORT/bin/.i686/coolc"
SPIM="$COOL_SUPPORT/bin/.i686/spim"
TRAP_HANDLER="$COOL_SUPPORT/lib/trap.handler"

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

# Check if Stanford binaries exist and are executable
check_prerequisites() {
    log "Checking prerequisites..."
    
    if [[ ! -x "$COOLC" ]]; then
        error "Stanford coolc not found or not executable: $COOLC"
        error "Run: chmod +x $COOL_SUPPORT/bin/.i686/*"
        exit 1
    fi
    
    if [[ ! -x "$SPIM" ]]; then
        error "Stanford spim not found or not executable: $SPIM"
        exit 1
    fi
    
    if [[ ! -f "$TRAP_HANDLER" ]]; then
        error "trap.handler not found: $TRAP_HANDLER"
        exit 1
    fi
    
    log "Prerequisites OK"
}

# Compile and run a COOL program, capturing output
validate_program() {
    local cool_file="$1"
    local basename="$(basename "$cool_file" .cl)"
    local asm_file="$WORK_DIR/${basename}.s"
    local output_file="$OUTPUT_DIR/${basename}.output"
    
    log "Processing: $basename"
    
    # Copy to work dir and compile
    cp "$cool_file" "$WORK_DIR/${basename}.cl"
    cd "$WORK_DIR"
    
    # Compile with Stanford coolc
    if ! "$COOLC" "${basename}.cl" 2>"${basename}.compile.err"; then
        warn "Compilation failed for $basename"
        echo "COMPILATION_FAILED" > "$output_file"
        cat "${basename}.compile.err" >> "$output_file"
        return 1
    fi
    
    # Check if assembly was generated
    if [[ ! -f "${basename}.s" ]]; then
        warn "No assembly generated for $basename"
        echo "NO_ASSEMBLY_GENERATED" > "$output_file"
        return 1
    fi
    
    # Run with spim (provide empty input for interactive programs, timeout after 5s)
    if ! timeout 5 "$SPIM" -trap_file "$TRAP_HANDLER" -file "${basename}.s" < /dev/null > "${basename}.spim.out" 2>&1; then
        local exit_code=$?
        if [[ $exit_code -eq 124 ]]; then
            warn "Timeout for $basename (likely interactive program)"
            echo "INTERACTIVE_TIMEOUT" > "$output_file"
            cat "${basename}.spim.out" >> "$output_file"
            return 1
        else
            warn "Runtime error for $basename"
            echo "RUNTIME_ERROR" > "$output_file"
            cat "${basename}.spim.out" >> "$output_file"
            return 1
        fi
    fi
    
    # Extract actual program output (skip SPIM header and success message)
    grep -v "^SPIM Version" "${basename}.spim.out" | \
        grep -v "^Copyright" | \
        grep -v "^All Rights Reserved" | \
        grep -v "^See the file README" | \
        grep -v "^Loaded:" | \
        grep -v "^COOL program successfully executed" | \
        grep -v "^Stats" | \
        sed '/^$/d' > "$output_file"
    
    log "  ✓ Captured output: $output_file"
    return 0
}

main() {
    log "Stanford COOL Validation Script"
    log "================================"
    
    check_prerequisites
    
    # Create output directory
    mkdir -p "$OUTPUT_DIR"
    
    # Create temporary work directory
    mkdir -p "$WORK_DIR"
    trap "rm -rf '$WORK_DIR'" EXIT
    
    # Find all .cl files
    local total=0
    local success=0
    local failed=0
    
    while IFS= read -r -d '' cool_file; do
        total=$((total + 1))
        if validate_program "$cool_file"; then
            success=$((success + 1))
        else
            failed=$((failed + 1))
        fi
    done < <(find "$EXAMPLES_DIR" -name "*.cl" -type f -print0 | sort -z)
    
    # Summary
    echo ""
    log "================================"
    log "Validation Summary"
    log "================================"
    log "Total programs: $total"
    log "Successful: $success"
    if [[ $failed -gt 0 ]]; then
        warn "Failed: $failed"
    fi
    log "Output directory: $OUTPUT_DIR"
    
    # List generated outputs
    echo ""
    log "Generated output files:"
    ls -1 "$OUTPUT_DIR"/*.output | while read -r f; do
        local size=$(wc -l < "$f")
        echo "  - $(basename "$f") ($size lines)"
    done
    
    if [[ $failed -gt 0 ]]; then
        exit 1
    fi
}

main "$@"
