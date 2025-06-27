use std::fs;

use chumsky::{
    input::{Stream, ValueInput},
    prelude::*,
};

use lexer::Token;
use logos::Logos;

mod ast;

fn main() -> Result<(), std::io::Error> {
    let file_path = std::env::args().nth(1).unwrap();
    let src = fs::read_to_string(&file_path).unwrap();

     let token_iter = Token::lexer(&src)
        .spanned()
        // Convert logos errors into tokens. We want parsing to be recoverable and not fail at the lexing stage, so
        // we have a dedicated `Token::Error` variant that represents a token error that was previously encountered
        .map(|(tok, span)| match tok {
            // Turn the `Range<usize>` spans logos gives us into chumsky's `SimpleSpan` via `Into`, because it's easier
            // to work with
            Ok(tok) => (tok, SimpleSpan::from(span)),
            Err(()) => (Token::Error, span.into()),
        });

    // Turn the token iterator into a stream that chumsky can use for things like backtracking
    let token_stream = Stream::from_iter(token_iter)
        // Tell chumsky to split the (Token, SimpleSpan) stream into its parts so that it can handle the spans for us
        // This involves giving chumsky an 'end of input' span: we just use a zero-width span at the end of the string
        .map((0..src.len()).into(), |(t, s): (_, _)| (t, s));

    match parser().parse(token_stream).into_result() {
        Ok(parsed) => {
            println!("Parsed successfully!");
            println!("AST: {:#?}", parsed);
        },
        Err(errors) => {
            for error in errors {
                println!("Error: {:?}", error);
            }
        }
    }

    Ok(())
}


fn parser<'tokens, I>(
) -> impl Parser<'tokens, I, ast::Program, extra::Err<Rich<'tokens, Token>>>
where
    I: ValueInput<'tokens, Token = Token, Span = SimpleSpan>,
{
    // Parse a basic class structure  
    let class = just(Token::TypeIdentifier)
        .then_ignore(just(Token::LeftBrace))
        .then_ignore(just(Token::RightBrace))
        .map(|_| ast::Class { 
            name: "TestClass".to_string(), 
            inherits: None, 
            features: Vec::new(), 
            span: 0..0 
        });

    // Parse multiple classes - collect them properly
    let program = class.repeated().collect::<Vec<_>>().map(|classes| ast::Program { classes });

    program.then_ignore(end())
}