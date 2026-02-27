#!/bin/bash

# Test lexer coverage against all sample files
# Check for any tokens we're missing

LEXER_BIN="./target/debug/lexer"
SAMPLES_DIR="./cool-support/examples"
LOCAL_SAMPLES_DIR="./samples"

echo "=== Testing Lexer Coverage Against Sample Files ==="
echo

# Build lexer if needed
if [ ! -f "$LEXER_BIN" ]; then
    echo "Building lexer..."
    cargo build -p lexer --bin lexer
    echo
fi

test_lexer() {
    local file=$1
    local filename=$(basename "$file")
    
    # Run lexer and capture output
    output=$($LEXER_BIN "$file" 2>&1)
    exit_code=$?
    
    # Check for Error tokens in output
    error_count=$(echo "$output" | grep -c "Token::Error" || true)
    
    printf "%-30s " "$filename"
    if [ $exit_code -eq 0 ] && [ $error_count -eq 0 ]; then
        echo "✓ PASS (no errors)"
    elif [ $error_count -gt 0 ]; then
        echo "✗ FAIL ($error_count error tokens)"
    else
        echo "✗ FAIL (exit code $exit_code)"
    fi
}

# Test local samples first
if [ -d "$LOCAL_SAMPLES_DIR" ]; then
    echo "Testing local samples in $LOCAL_SAMPLES_DIR:"
    for file in "$LOCAL_SAMPLES_DIR"/*.cl; do
        if [ -f "$file" ]; then
            test_lexer "$file"
        fi
    done
    echo
fi

# Test Stanford examples if they exist
if [ -d "$SAMPLES_DIR" ]; then
    echo "Testing Stanford examples in $SAMPLES_DIR:"
    for file in "$SAMPLES_DIR"/*.cl; do
        if [ -f "$file" ]; then
            test_lexer "$file"
        fi
    done
else
    echo "Stanford examples directory not found at $SAMPLES_DIR"
fi

echo
echo "=== Lexer Coverage Test Complete ==="
