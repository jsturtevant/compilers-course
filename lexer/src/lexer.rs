use logos::{Lexer, Logos, Skip};

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(extras = (usize, usize))]
#[logos(skip r"[ \t\r\f]+")]
#[regex(r"\n", newline_callback)]
pub enum Token {
    Error,

    // integers
    #[regex(r"[0-9]+")]
    Integer,

    // type identifiers (begin with a capital letter)
    #[regex(r"[A-Z][A-Za-z0-9_]*")]
    TypeIdentifier,

    // object identifiers (begin with a lower case letter)
    #[regex(r"[a-z][A-Za-z0-9_]*")]
    ObjectIdentifier,

    // special identifiers
    #[token("self")]
    SelfLit,

    #[token("SELF_TYPE")]
    SelfType,

    #[regex(r#""([^"\\\n]|\\[^0\n]|\\[ \t]*\n)*""#)]
    String,

    // keywords
    #[token("class", ignore(case))]
    Class,

    #[token("else", ignore(case))]
    Else,

    #[token("fi", ignore(case))]
    Fi,

    #[token("if", ignore(case))]
    If,

    #[token("in", ignore(case))]
    In,

    #[token("inherits", ignore(case))]
    Inherits,

    #[token("isvoid", ignore(case))]
    Isvoid,

    #[token("let", ignore(case))]
    Let,

    #[token("loop", ignore(case))]
    Loop,

    #[token("pool", ignore(case))]
    Pool,

    #[token("then", ignore(case))]
    Then,

    #[token("while", ignore(case))]
    While,

    #[token("case", ignore(case))]
    Case,

    #[token("esac", ignore(case))]
    Esac,

    #[token("new", ignore(case))]
    New,

    #[token("of", ignore(case))]
    Of,

    #[token("not", ignore(case))]
    Not,

    #[regex("t[Rr][Uu][Ee]")]
    True,

    #[regex("f[Aa][Ll][Ss][Ee]")]
    False,

    // Comments - single line and multi-line with nesting support
    #[regex(r"--[^\n]*")]
    #[token("(*", comment_multi)]
    Comment,

    // operators
    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("*")]
    Multiply,

    #[token("/")]
    Divide,

    #[token("~")]
    Tilde,

    #[token("<")]
    LessThan,

    #[token("<=")]
    LessThanOrEqual,

    #[token("=")]
    Equal,

    // special characters
    #[token("(")]
    LeftParen,

    #[token(")")]
    RightParen,

    #[token("=>")]
    DoubleArrow,

    #[token("<-")]
    Assign,

    #[token(":")]
    Identify,

    #[token("@")]
    TypeId,

    // brackets and braces
    #[token("{")]
    LeftBrace,

    #[token("}")]
    RightBrace,

    // punctuation
    #[token(";")]
    Semicolon,

    #[token(".")]
    Dot,

    #[token(",")]
    Comma,

    #[regex(r"\n", newline_callback)]
    Newline,
}

fn newline_callback(lex: &mut Lexer<Token>) -> Skip {
    lex.extras.0 += 1;
    lex.extras.1 = lex.span().start + 1;
    Skip
}

/// Processes a multi-line comment with support for nesting
/// Returns true if the comment was properly terminated, false otherwise
fn comment_multi(lex: &mut logos::Lexer<Token>) -> bool {
    let remainder = lex.remainder();
    let mut depth = 1;
    let mut pos = 0;

    while pos < remainder.len() {
        // Look for opening or closing comment markers
        if remainder[pos..].starts_with("(*") {
            depth += 1;
            pos += 2;
        } else if remainder[pos..].starts_with("*)") {
            depth -= 1;
            pos += 2;
            if depth == 0 {
                // We've found the matching closing comment marker
                lex.bump(pos);
                return true;
            }
        } else {
            // Move to the next character
            pos += 1;
        }
    }

    // If we reach here, we had an unclosed comment
    // We'll consume all the remaining text
    lex.bump(remainder.len());
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use logos::Logos;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_lexer_simple() {
        let hello_world = r#"class Main inherits IO {
   main(): SELF_TYPE {
	out_string("Hello, World.\n")
   };
};
"#;

        let mut lexer = Token::lexer(hello_world);
        while let Some(token) = lexer.next() {
            let span = lexer.span();
            match token {
                Ok(t) => {
                    println!(
                        "Token: {:?}, Span: {:?}, Text: '{}'",
                        t,
                        span,
                        &hello_world[span.clone()]
                    );
                }
                Err(_) => {
                    println!(
                        "Failed to match at span {:?}, Text: '{}'",
                        span,
                        &hello_world[span.clone()]
                    );
                    panic!("unmatched item at span {:?}", span);
                }
            }
        }

        assert_eq!(
            lexer.extras.0, 5,
            "Expected 5 lines, got {}",
            lexer.extras.0
        );
    }

    #[test]
    fn test_lexer_on_cl_files() {
        let folder_path = "../samples";
        assert!(Path::new(folder_path).exists());

        // Read all files in the directory
        for f in fs::read_dir(folder_path).unwrap() {
            let entry = f.unwrap();
            let p = entry.path();
            if let Some(extension) = p.extension() {
                if extension == "cl" {
                    println!("Lexing file: {:?}", p);
                    let input = fs::read_to_string(&p).expect("Failed to read file");

                    let mut lexer = Token::lexer(&input);
                    while let Some(token) = lexer.next() {
                        let span = lexer.span();
                        match token {
                            Ok(_) => {}
                            Err(_) => {
                                let line = lexer.extras.0;
                                panic!(
                                    "error lexing {:?}: '{:?}' at {:?} on line {}\n\n{}\n\n",
                                    p,
                                    &input[span.clone()],
                                    span,
                                    line,
                                    input.lines().nth(line).unwrap()
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
