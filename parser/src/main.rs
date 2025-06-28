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
        }
        Err(errors) => {
            for error in errors {
                println!("Error: {:?}", error);
            }
        }
    }

    Ok(())
}

fn parser<'tokens, I>() -> impl Parser<'tokens, I, ast::Program, extra::Err<Rich<'tokens, Token>>>
where
    I: ValueInput<'tokens, Token = Token, Span = SimpleSpan>,
{
    let ident = select! { Token::ObjectIdentifier(s) => s };
    let type_id = select! { Token::TypeIdentifier(s) => s, Token::SelfType => "SELF_TYPE".to_string() };

    let expr = recursive(|expr| {
        let new_expr = just(Token::New)
            .ignore_then(type_id)
            .map(ast::Expr::New);

        let block = expr
            .clone()
            .separated_by(choice((just(Token::Dot), just(Token::Semicolon))))
            .allow_trailing()
            .at_least(1)
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LeftBrace), just(Token::RightBrace))
            .map(ast::Expr::Block);

        let atom = choice((
            select! { Token::String(s) => ast::Expr::String(s) },
            select! { Token::Integer(i) => ast::Expr::Integer(i) },
            just(Token::Isvoid)
                .ignore_then(expr.clone())
                .map(|e| ast::Expr::IsVoid(Box::new(e))),
            just(Token::SelfLit).to(ast::Expr::Id("self".to_string())),
            ident.map(ast::Expr::Id),
            new_expr,
            block,
            expr.clone()
                .delimited_by(just(Token::LeftParen), just(Token::RightParen)),
        ));

        let term = atom.foldl(
            just(Token::Dot)
                .ignore_then(ident)
                .then(
                    expr.clone()
                        .separated_by(just(Token::Comma))
                        .allow_trailing()
                        .collect::<Vec<_>>()
                        .delimited_by(just(Token::LeftParen), just(Token::RightParen)),
                )
                .map(|(method, args)| (method, args))
                .repeated(),
            |expr, (method, args)| ast::Expr::Dispatch {
                expr: Box::new(expr),
                static_type: None,
                method,
                args,
            },
        );

        let call = ident
            .then(
                expr.clone()
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LeftParen), just(Token::RightParen)),
            )
            .map(|(name, args)| ast::Expr::FuncCall { name, args });

        choice((call, term))
    });

    let formal = ident
        .then_ignore(just(Token::Colon))
        .then(type_id)
        .map(|(name, typ)| ast::Formal { name, typ });

    let method_feature = ident
        .then(
            formal
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::LeftParen), just(Token::RightParen)),
        )
        .then_ignore(just(Token::Colon))
        .then(type_id)
        .then(
            expr.clone()
                .delimited_by(just(Token::LeftBrace), just(Token::RightBrace)),
        )
        .map(|(((name, formals), return_type), body)| {
            ast::Feature::Method(ast::MethodFeature {
                name,
                formals,
                return_type,
                body,
            })
        });

    let feature = method_feature.then_ignore(just(Token::Semicolon));

    let class = just(Token::Class)
        .ignore_then(type_id)
        .then(just(Token::Inherits).ignore_then(type_id).or_not())
        .then(
            feature
                .repeated()
                .collect()
                .delimited_by(just(Token::LeftBrace), just(Token::RightBrace)),
        )
        .then_ignore(just(Token::Semicolon))
        .map(|((name, parent), features)| ast::Class {
            name,
            parent,
            features,
        });

    class
        .repeated()
        .collect()
        .map(|classes| ast::Program { classes })
}
