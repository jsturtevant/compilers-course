# COOL Language Lexer

A lexical analyzer (lexer) for the COOL (Classroom Object-Oriented Language) programming language, implemented in Rust using the [logos](https://github.com/maciejhirsz/logos) lexer generator.

## Features

- **Complete COOL Language Support**: Tokenizes all COOL language constructs including:
  - Keywords (class, if, then, else, fi, while, loop, pool, etc.)
  - Identifiers (type identifiers, object identifiers, special identifiers)
  - Literals (integers, strings, booleans)
  - Operators and punctuation
  - Comments (single-line `--` and nested multi-line `(* ... *)`)
- **Robust Comment Handling**: Supports nested multi-line comments with proper depth tracking
- **Line Tracking**: Maintains line and column position information for error reporting
- **Comprehensive Testing**: Includes unit tests and integration tests with sample COOL files

## Building and Running

### Build
```bash
cargo build
```

### Run Tests
```bash
cargo test
```

### Run with Sample Files
```bash
cargo run cool.cl
```

## Testing

The project includes several test suites:
- Unit tests for individual token types
- Integration tests with real COOL program files

Run tests with:
```bash
cargo test
```

For verbose output:
```bash
cargo test -- --nocapture
```
