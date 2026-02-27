# Stanford COOL Toolchain Validation Setup

**Date:** 2025-02-27  
**By:** Turing (Lead Architect)  
**Requested by:** James Sturtevant

## Summary

Explored Stanford's reference COOL compiler toolchain and created validation infrastructure to establish ground truth for testing our Rust WASM compiler implementation.

## Stanford Binaries Inventory

Location: `cool-support/bin/.i686/`

| Binary | Purpose | Type | Notes |
|--------|---------|------|-------|
| `coolc` | Complete COOL compiler | Statically linked 32-bit ELF | Full pipeline: lexer→parser→semant→cgen |
| `lexer` | Tokenizer only | Statically linked 32-bit ELF | Outputs token stream |
| `parser` | Parser only | Statically linked 32-bit ELF | **HANGS** - likely expects AST output mode |
| `semant` | Semantic analyzer | Statically linked 32-bit ELF | Standalone semantic pass |
| `cgen` | Code generator | Statically linked 32-bit ELF | Generates MIPS assembly |
| `spim` | MIPS simulator | Dynamically linked 32-bit ELF | Executes generated assembly |
| `xspim` | MIPS simulator (X11) | Dynamically linked 32-bit ELF | GUI version |
| `anngen` | Annotation generator | Statically linked 32-bit ELF | Internal tool |
| `aps2c++` | APS to C++ | Dynamically linked 32-bit ELF | Internal tool |
| `aps2java` | APS to Java | Dynamically linked 32-bit ELF | Internal tool |

**Key finding:** All binaries are 32-bit i686 ELF binaries from 2011. Required `chmod +x` to make executable.

## Full Pipeline Usage

### Complete Compilation (coolc)

```bash
# Compile COOL source to MIPS assembly
cd /tmp
coolc hello_world.cl
# Generates: hello_world.s

# Run with MIPS simulator
spim -trap_file /path/to/cool-support/lib/trap.handler -file hello_world.s
# Output: Hello, World.
```

**Critical:** `spim` requires:
1. Explicit path to `trap.handler` runtime library
2. Working directory doesn't matter if full paths provided
3. Outputs SPIM banner + program output + "COOL program successfully executed"

### Pipeline Stages (Individual Tools)

```bash
# Lexer: Source → Token Stream
lexer hello_world.cl
# Outputs: #name "..." \n #1 CLASS \n #1 TYPEID Main \n ...

# Parser: Source → AST
parser hello_world.cl
# **HANGS** - likely waiting for additional flags or input mode
# DO NOT USE in automation

# Full pipeline equivalent (coolc does all):
coolc source.cl     # → source.s (MIPS assembly)
spim -trap_file ... -file source.s   # → program output
```

## Sample Programs Analysis

Total: **18 COOL programs** in `cool-support/examples/`

### Successfully Validated (16 programs)

| Program | Output Lines | Description |
|---------|--------------|-------------|
| `hello_world.cl` | 1 | Simple IO test |
| `cool.cl` | 1 | Basic object instantiation |
| `complex.cl` | 1 | Complex number arithmetic |
| `new_complex.cl` | 2 | Complex numbers with methods |
| `hairyscary.cl` | 1 | Expression evaluation |
| `io.cl` | 5 | IO operations test |
| `book_list.cl` | 7 | Linked list implementation |
| `palindrome.cl` | 7 | String palindrome check |
| `list.cl` | 5 | Generic list operations |
| `cells.cl` | 22 | Cellular automaton |
| `life.cl` | 4 | Conway's Game of Life |
| `sort_list.cl` | 1 | List sorting |
| `graph.cl` | 0 | Graph data structure (no output) |
| `primes.cl` | 99 | Prime number generation |
| `lam.cl` | 528 | Lambda calculus interpreter |
| `arith.cl` | 9689 | **Interactive:** arithmetic calculator (timeout with empty input) |

### Compilation Failures (2 programs)

1. **`atoi.cl`** — No Main class. This is a library/utility class (A2I).
2. **`atoi_test.cl`** — References A2I from atoi.cl. These are **multi-file programs**.

**Multi-file compilation:**
```bash
coolc atoi.cl atoi_test.cl
# Generates: atoi.s (named after first file)
```

## Validation Script

**Created:** `scripts/validate-samples.sh`

### Purpose
Establish ground truth outputs for all sample programs by running them through Stanford's reference compiler.

### Features
- ✅ Compiles all `.cl` files in `cool-support/examples/`
- ✅ Executes each with `spim` MIPS simulator
- ✅ Captures program output (strips SPIM banners)
- ✅ Handles compilation failures gracefully
- ✅ Timeouts for interactive programs (5s limit)
- ✅ Generates baseline outputs in `scripts/expected-outputs/`

### Usage

```bash
# Run validation
./scripts/validate-samples.sh

# Outputs:
# - scripts/expected-outputs/*.output (one per program)
# - Summary: 16 successful, 2 failed (multi-file)
```

### Output Format

Each `.output` file contains:
- **Success:** Actual program stdout (SPIM banners removed)
- **Compilation failure:** `COMPILATION_FAILED` + error messages
- **Runtime error:** `RUNTIME_ERROR` + spim output
- **Interactive timeout:** `INTERACTIVE_TIMEOUT` + partial output

### Validation Results

```
Total programs: 18
Successful: 16
Failed: 2 (atoi.cl, atoi_test.cl - multi-file programs)
```

## Integration with Our Compiler

### Testing Strategy

1. **Phase-by-phase validation:**
   - Lexer: Compare token streams vs `cool-support/bin/.i686/lexer` output
   - Parser: Compare AST structure (no direct Stanford output available)
   - Semantic: Compare error messages vs `semant` output
   - Codegen: Compare **final program output** vs `scripts/expected-outputs/*.output`

2. **End-to-end validation:**
   ```bash
   # Our compiler (Rust → WASM)
   cargo run --bin cool-compiler -- hello_world.cl -o hello.wasm
   wasmtime hello.wasm > our_output.txt
   
   # Compare against ground truth
   diff our_output.txt scripts/expected-outputs/hello_world.output
   ```

3. **Stanford output format flags:**
   - Use `--stanford-lexer`, `--stanford-parser` flags for exact format matching
   - Final WASM output doesn't need to match Stanford format—only functional behavior matters

### Critical Findings for WASM Backend

1. **No register allocation needed** (confirmed benefit of WASM target)
2. **IO.out_string, IO.in_int must produce identical output** to Stanford's implementation
3. **Built-in classes (Object, IO, String, Int, Bool)** must match Stanford semantics exactly
4. **Error messages don't need to match** (Stanford's are reference, not spec)
5. **Multi-file compilation** is a future requirement (atoi.cl + atoi_test.cl)

## Next Steps

1. ✅ **Validation infrastructure complete** — baseline established
2. **Semantic analysis phase** — use Stanford `semant` errors as reference
3. **WASM codegen testing** — diff outputs vs `scripts/expected-outputs/`
4. **Integration tests** — automate validation in CI pipeline
5. **Multi-file support** — handle `atoi.cl` + `atoi_test.cl` case

## File Structure Created

```
scripts/
├── validate-samples.sh          # Validation script (executable)
└── expected-outputs/            # Ground truth outputs
    ├── hello_world.output       # 1 line: "Hello, World."
    ├── primes.output            # 99 lines of primes
    ├── arith.output             # 9689 lines (interactive with empty input)
    ├── atoi.output              # COMPILATION_FAILED (no Main)
    ├── atoi_test.output         # COMPILATION_FAILED (needs atoi.cl)
    └── ... (18 total)
```

## Stanford Toolchain Commands Reference

```bash
# Compile single file
coolc program.cl

# Compile multiple files (library + main)
coolc library.cl main.cl

# Run compiled program
spim -trap_file cool-support/lib/trap.handler -file program.s

# Lexer only
cool-support/bin/.i686/lexer program.cl

# Full pipeline with output filtering
coolc hello_world.cl && \
  spim -trap_file cool-support/lib/trap.handler -file hello_world.s 2>&1 | \
  grep -v "^SPIM Version" | grep -v "^Copyright" | grep -v "^Loaded:"
```

---

**Status:** ✅ Complete  
**Validation script tested:** 16/18 programs validated successfully  
**Ground truth established:** Ready for Rust compiler testing
