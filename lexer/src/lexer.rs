use logos::{Lexer, Logos, Skip};
use std::fmt;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(extras = (usize, usize))]
#[logos(skip r"[ \t\r\f]+")]
#[regex(r"\n", newline_callback)]
pub enum Token {
    #[allow(dead_code)]
    Error,

    // integers
    #[regex(r"[0-9]+", callback = |lex| lex.slice().parse::<i32>().ok())]
    Integer(i32),

    // type identifiers (begin with a capital letter)
    #[regex(r"[A-Z][A-Za-z0-9_]*", callback = |lex| lex.slice().to_string())]
    TypeIdentifier(String),

    // object identifiers (begin with a lower case letter)
    #[regex(r"[a-z][A-Za-z0-9_]*", callback = |lex| lex.slice().to_string())]
    ObjectIdentifier(String),

    // special identifiers
    #[token("self")]
    SelfLit,

    #[token("SELF_TYPE")]
    SelfType,

    #[regex(r#""([^"\\\n]|\\[^0\n]|\\[ \t]*\n)*""#, callback = |lex| lex.slice().to_string())]
    String(String),

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
    Colon,

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

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Error => write!(f, "Error"),
            Token::Integer(i) => write!(f, "{i}"),
            Token::TypeIdentifier(id) => write!(f, "{id}"),
            Token::ObjectIdentifier(id) => write!(f, "{id}"),
            Token::SelfLit => write!(f, "self"),
            Token::SelfType => write!(f, "SELF_TYPE"),
            Token::String(s) => write!(f, "{s}"),
            Token::Class => write!(f, "class"),
            Token::Else => write!(f, "else"),
            Token::Fi => write!(f, "fi"),
            Token::If => write!(f, "if"),
            Token::In => write!(f, "in"),
            Token::Inherits => write!(f, "inherits"),
            Token::Isvoid => write!(f, "isvoid"),
            Token::Let => write!(f, "let"),
            Token::Loop => write!(f, "loop"),
            Token::Pool => write!(f, "pool"),
            Token::Then => write!(f, "then"),
            Token::While => write!(f, "while"),
            Token::Case => write!(f, "case"),
            Token::Esac => write!(f, "esac"),
            Token::New => write!(f, "new"),
            Token::Of => write!(f, "of"),
            Token::Not => write!(f, "not"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Assign => write!(f, "<-"),
            Token::Comma => write!(f, ","),
            Token::Semicolon => write!(f, ";"),
            Token::Dot => write!(f, "."),
            Token::Comment => write!(f, "Comment"),
            Token::Multiply => write!(f, "*"),
            Token::Divide => write!(f, "/"),
            Token::Tilde => write!(f, "~"),
            Token::LessThan => write!(f, "<"),
            Token::LessThanOrEqual => write!(f, "<="),
            Token::Equal => write!(f, "="),
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::DoubleArrow => write!(f, "=>"),
            Token::Colon => write!(f, ":"),
            Token::TypeId => write!(f, "@"),
            Token::LeftBrace => write!(f, "{{"),
            Token::RightBrace => write!(f, "}}"),
            Token::Newline => writeln!(f),
        }
    }
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
