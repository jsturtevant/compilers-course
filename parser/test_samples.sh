#!/bin/bash

# Test parser against all sample files

PARSER_BIN="./target/debug/parser"
SAMPLES_DIR="./cool-support/examples"
LOCAL_SAMPLES_DIR="./samples"

echo "=== Testing Parser Against Sample Files ==="
echo

# Test local samples first
if [ -d "$LOCAL_SAMPLES_DIR" ]; then
    echo "Testing local samples in $LOCAL_SAMPLES_DIR:"
    for file in "$LOCAL_SAMPLES_DIR"/*.cl; do
        if [ -f "$file" ]; then
            filename=$(basename "$file")
            printf "%-30s " "$filename"
            if $PARSER_BIN "$file" > /dev/null 2>&1; then
                echo "✓ PASS"
            else
                echo "✗ FAIL"
            fi
        fi
    done
    echo
fi

# Test Stanford examples if they exist
if [ -d "$SAMPLES_DIR" ]; then
    echo "Testing Stanford examples in $SAMPLES_DIR:"
    for file in "$SAMPLES_DIR"/*.cl; do
        if [ -f "$file" ]; then
            filename=$(basename "$file")
            printf "%-30s " "$filename"
            if $PARSER_BIN "$file" > /dev/null 2>&1; then
                echo "✓ PASS"
            else
                echo "✗ FAIL"
            fi
        fi
    done
else
    echo "Stanford examples directory not found at $SAMPLES_DIR"
fi

echo
echo "=== Testing Complete ==="
