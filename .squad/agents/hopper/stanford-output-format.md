# Stanford COOL Compiler Output Format Specification

This document specifies the exact output format of the Stanford reference compiler phases for byte-for-byte compatibility testing.

**Source:** Verified against `cool-support/bin/.i686/lexer` and `cool-support/bin/.i686/parser` (32-bit ELF, statically linked binaries that run successfully on this system).

---

## 1. Lexer Output Format

### 1.1 Header Line

The lexer outputs a header line with the filename:
```
#name "filepath"
```

Example:
```
#name "/home/user/test.cl"
```

### 1.2 Token Format

Each token is output on a single line:
```
#<line_number> <token_type> [value]
```

Where:
- `#<line_number>` — Line number where the token appears (1-indexed)
- `<token_type>` — Token name or single-character in quotes
- `[value]` — Present only for tokens with semantic values

### 1.3 Token Types

#### Named Tokens (Keywords)
Output as uppercase identifier, no value:
```
#1 CLASS
#1 ELSE
#1 FI
#1 IF
#1 IN
#1 INHERITS
#1 LET
#1 LOOP
#1 POOL
#1 THEN
#1 WHILE
#1 CASE
#1 ESAC
#1 OF
#1 NEW
#1 ISVOID
#1 NOT
```

#### Multi-Character Operators
```
#1 ASSIGN     (for <-)
#1 DARROW     (for =>)
#1 LE         (for <=)
```

#### Single-Character Tokens
Wrapped in single quotes:
```
#1 '+'
#1 '-'
#1 '*'
#1 '/'
#1 '~'
#1 '<'
#1 '='
#1 '.'
#1 ','
#1 ';'
#1 ':'
#1 '('
#1 ')'
#1 '@'
#1 '{'
#1 '}'
```

#### Tokens with Values

**INT_CONST** — Integer value as-is (no leading zeros stripped):
```
#1 INT_CONST 42
#1 INT_CONST 0
#1 INT_CONST 123456789
```

**BOOL_CONST** — Lowercase `true` or `false`:
```
#1 BOOL_CONST true
#1 BOOL_CONST false
```
Note: Case-insensitive lexing for true/false (fAlSe → false)

**STR_CONST** — Double-quoted with escape sequences:
```
#1 STR_CONST "hello world"
#1 STR_CONST "line1\nline2"
#1 STR_CONST "tab:\tquote:\""
```

**TYPEID** — Type identifiers (start with uppercase):
```
#1 TYPEID Main
#1 TYPEID IO
#1 TYPEID Int
#1 TYPEID SELF_TYPE
```

**OBJECTID** — Object identifiers (start with lowercase):
```
#1 OBJECTID main
#1 OBJECTID out_string
#1 OBJECTID self
#1 OBJECTID x
```

**ERROR** — Error token with message:
```
#1 ERROR "$"
#1 ERROR "Unterminated string constant"
```

### 1.4 String Escape Sequences

The `print_escaped_string()` function (utilities.cc:43-71) handles escapes:

| Character | Output | Notes |
|-----------|--------|-------|
| `\` | `\\` | Backslash |
| `"` | `\"` | Double quote |
| `\n` (newline) | `\n` | Two characters: backslash + n |
| `\t` (tab) | `\t` | Two characters: backslash + t |
| `\b` (backspace) | `\b` | Two characters: backslash + b |
| `\f` (form feed) | `\f` | Two characters: backslash + f |
| Non-printable | `\###` | **3-digit octal**, zero-padded |
| Printable | literal | Characters where `isprint()` is true |

**Critical:** Non-printable characters are output as 3-digit octal with leading zeros.

Example: Bell character (ASCII 7) → `\007`
Example: NUL character (ASCII 0) → `\000`

### 1.5 Complete Lexer Example

Input (`hello_world.cl`):
```cool
class Main inherits IO {
   main(): SELF_TYPE {
	out_string("Hello, World.\n")
   };
};
```

Output:
```
#name "/home/jstur/projects/compilers-course-project/compilers-course/cool-support/examples/hello_world.cl"
#1 CLASS
#1 TYPEID Main
#1 INHERITS
#1 TYPEID IO
#1 '{'
#2 OBJECTID main
#2 '('
#2 ')'
#2 ':'
#2 TYPEID SELF_TYPE
#2 '{'
#3 OBJECTID out_string
#3 '('
#3 STR_CONST "Hello, World.\n"
#3 ')'
#4 '}'
#4 ';'
#5 '}'
#5 ';'
```

---

## 2. Parser Output Format

The parser reads lexer output from stdin and produces an AST dump.

### 2.1 Pipeline Usage

```bash
lexer file.cl | parser
```

### 2.2 AST Node Format

Each node follows this pattern:
```
<padding>#<line_number>
<padding>_<node_type>
<padding+2><child_or_value>
...
<padding>: <type>
```

Where:
- `<padding>` — 2 spaces per nesting level
- `#<line_number>` — Source line of the construct
- `_<node_type>` — Node name with underscore prefix
- `<child_or_value>` — Symbols printed directly, children recursively dumped
- `: <type>` — Expression type (`: _no_type` before semantic analysis)

### 2.3 Node Type Names

| AST Node | Output Name |
|----------|-------------|
| Program | `_program` |
| Class | `_class` |
| Method | `_method` |
| Attribute | `_attr` |
| Formal Parameter | `_formal` |
| Case Branch | `_branch` |
| Assignment | `_assign` |
| Static Dispatch | `_static_dispatch` |
| Dispatch | `_dispatch` |
| If/Then/Else | `_cond` |
| While/Loop | `_loop` |
| Case/Of/Esac | `_typcase` |
| Block | `_block` |
| Let | `_let` |
| Plus | `_plus` |
| Minus | `_sub` |
| Multiply | `_mul` |
| Divide | `_divide` |
| Negate (~) | `_neg` |
| Less Than | `_lt` |
| Less or Equal | `_leq` |
| Equal | `_eq` |
| Not/Complement | `_comp` |
| Integer Literal | `_int` |
| Boolean Literal | `_bool` |
| String Literal | `_string` |
| New | `_new` |
| IsVoid | `_isvoid` |
| No Expression | `_no_expr` |
| Object Reference | `_object` |

### 2.4 Symbol Dumping

`dump_Symbol()` outputs a symbol with padding followed by the symbol text:
```
<padding><symbol_name>
```

Example for `Main`:
```
    Main
```

### 2.5 Boolean Dumping

`dump_Boolean()` outputs 0 or 1:
```
<padding>0     (for false)
<padding>1     (for true)
```

### 2.6 String Dumping

Strings in AST use the same `print_escaped_string()` function, wrapped in quotes:
```
<padding>"Hello, World.\n"
```

### 2.7 Class Node Structure

```
#<line>
_class
  <class_name>
  <parent_class>
  "<filename>"
  (
  <features...>
  )
```

### 2.8 Method Node Structure

```
#<line>
_method
  <method_name>
  <formal_params...>
  <return_type>
  <body_expr>
```

### 2.9 Dispatch Node Structure

```
#<line>
_dispatch
  <receiver_expr>
  <method_name>
  (
  <arguments...>
  )
: <type>
```

### 2.10 Complete Parser Example

Input (lexer output from `hello_world.cl`):
```
#name "/path/to/hello_world.cl"
#1 CLASS
#1 TYPEID Main
#1 INHERITS
#1 TYPEID IO
...
```

Parser Output:
```
#1
_program
  #1
  _class
    Main
    IO
    "/home/jstur/projects/compilers-course-project/compilers-course/cool-support/examples/hello_world.cl"
    (
    #2
    _method
      main
      SELF_TYPE
      #3
      _dispatch
        #3
        _object
          self
        : _no_type
        out_string
        (
        #3
        _string
          "Hello, World.\n"
        : _no_type
        )
      : _no_type
    )
```

---

## 3. Key Implementation Notes

### 3.1 Padding Function

The `pad(n)` function returns exactly `n` space characters (max 80):
```cpp
static char *padding = "                                                                                ";  // 80 spaces
char *pad(int n) {
    if (n > 80) return padding;
    if (n <= 0)  return "";
    return padding+(80-n);
}
```

### 3.2 Expression Type Suffix

All expressions have a type suffix line:
- Before semantic analysis: `: _no_type`
- After semantic analysis: `: <actual_type>` (e.g., `: Int`, `: String`)

### 3.3 Implicit Self for Dispatch

When dispatching on an implicit `self` (e.g., `out_string("hi")`), the receiver is:
```
#<line>
_object
  self
: _no_type
```

### 3.4 No-Expression Placeholder

Uninitialized attributes and optional expressions use:
```
#0
_no_expr
: _no_type
```

Note: Line number 0 for synthetic nodes.

---

## 4. Test Compatibility Checklist

For byte-for-byte compatibility, our Rust implementation must:

- [ ] Output `#name "<filepath>"` header line from lexer
- [ ] Use exact token names (uppercase, single quotes for chars)
- [ ] Use 3-digit zero-padded octal for non-printable chars
- [ ] Output BOOL_CONST as lowercase `true`/`false`
- [ ] Use exactly 2-space indentation increments in parser
- [ ] Output node names with underscore prefix (`_program`, `_class`, etc.)
- [ ] End each expression node with `: _no_type` or `: <type>`
- [ ] Wrap string literals in double quotes with proper escaping
- [ ] Use parentheses `(` `)` for feature and argument lists

---

## 5. Source Files Reference

- **Token names:** `cool-support/assignments/PA2/utilities.cc` — `cool_token_to_string()`
- **Token dumping:** `cool-support/assignments/PA2/utilities.cc` — `dump_cool_token()`
- **String escaping:** `cool-support/assignments/PA2/utilities.cc` — `print_escaped_string()`
- **AST dumping:** `cool-support/assignments/PA3/dumptype.cc` — `dump_with_types()` methods
- **Symbol dumping:** `cool-support/assignments/PA2/stringtab.cc` — `dump_Symbol()`
- **Boolean dumping:** `cool-support/assignments/PA3/cool-tree.handcode.h` — `dump_Boolean()`
- **Token definitions:** `cool-support/include/PA2/cool-parse.h` — enum values

---

## 6. Testing Commands

```bash
# Test lexer output
cool-support/bin/.i686/lexer cool-support/examples/hello_world.cl

# Test full pipeline
cool-support/bin/.i686/lexer examples/test.cl | cool-support/bin/.i686/parser

# Compare our output (when --stanford flag implemented)
diff <(cool-support/bin/.i686/lexer test.cl) <(cargo run -p lexer -- --stanford test.cl)
```
