use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[  \v\r\t\n\f]+")] // Ignore this regex pattern between tokens
pub enum Token {
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

    #[regex(r#""([^"\\n]|[\t\f\\])*""#)]
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
     TypeId
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
