# Compilers Course

A collection of compiler implementation projects for a compilers course, focusing on the COOL (Classroom Object-Oriented Language) programming language.

## Projects

- **[lexer/](lexer/)** - Lexical analyzer for COOL language built with Rust and logos
- **[parser/](parser/)** - Parser for COOL language using Chumsky parser combinators
- **[semant/](semant/)** - Semantic analyzer with type checking
- **[codegen/](codegen/)** - WebAssembly code generator

## Quick Start

```bash
# Compile and run a COOL program
cargo run -p codegen -- cool-support/examples/hello_world.cl -o hello.wasm
wasmtime hello.wasm
```

## WASM Output Modes

The compiler supports two output formats:

### Core Module (default)
```bash
cargo run -p codegen -- input.cl -o output.wasm
```
Produces a standard WebAssembly module (~2KB for hello_world).

### Component Model
```bash
cargo run -p codegen -- input.cl --component -o output.wasm
```
Produces a WASM Component (~22KB for hello_world) with WASI Preview1 adapter included. Components are the modern packaging format for WebAssembly, enabling better composability and interface definitions.

Both formats run with wasmtime:
```bash
wasmtime output.wasm
```

## Examples

```bash
# Hello World
cargo run -p codegen -- cool-support/examples/hello_world.cl -o hello.wasm && wasmtime hello.wasm

# Sort list (interactive - enter number of items)
cargo run -p codegen -- cool-support/examples/sort_list.cl -o sort.wasm
echo "5" | wasmtime sort.wasm

# Game of Life
cargo run -p codegen -- cool-support/examples/life.cl -o life.wasm && wasmtime life.wasm
```

## Getting Started

Each project directory contains its own README with specific build and run instructions.

## CI/CD

All projects include automated testing via GitHub Actions that runs on Linux, Windows, and macOS.
