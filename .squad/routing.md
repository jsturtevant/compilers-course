# Routing Rules

## Domain Routing

| Pattern | Route To |
|---------|----------|
| Architecture, design decisions, code review | Turing |
| Lexer, parser, AST, semantic analysis | Hopper |
| x86, ARM, codegen, assembly, backend, machine code | Ritchie |
| Rust idioms, performance, unsafe, lifetimes, traits | Stroustrup |
| Documentation, README, comments, BNF | Knuth |
| Session logs, decisions merge, memory | Scribe |
| Work queue, backlog, monitoring | Ralph |

## Keyword Triggers

- `lexer`, `tokenizer`, `logos` → Hopper
- `parser`, `chumsky`, `AST`, `grammar`, `BNF` → Hopper
- `x86`, `ARM`, `codegen`, `emit`, `assembly`, `register`, `instruction` → Ritchie
- `rust`, `lifetime`, `borrow`, `unsafe`, `trait`, `macro`, `cargo` → Stroustrup
- `docs`, `readme`, `comments`, `explain` → Knuth
- `review`, `architecture`, `design`, `tradeoff` → Turing

## Escalation

- Cross-cutting concerns → Turing coordinates
- Performance + correctness → Stroustrup + domain owner
- Backend architecture → Ritchie + Turing
