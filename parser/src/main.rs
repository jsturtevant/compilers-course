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
    let type_id =
        select! { Token::TypeIdentifier(s) => s, Token::SelfType => "SELF_TYPE".to_string() };

    let expr = recursive(|expr| {
        let new_expr = just(Token::New).ignore_then(type_id).map(ast::Expr::New);

        let block = expr
            .clone()
            .padded_by(just(Token::Comment).repeated())
            .separated_by(choice((just(Token::Dot), just(Token::Semicolon))))
            .allow_trailing()
            .collect::<Vec<_>>()
            .then_ignore(just(Token::Comment).repeated())
            .delimited_by(just(Token::LeftBrace), just(Token::RightBrace))
            .map(ast::Expr::Block);

        let assign = ident
            .then_ignore(just(Token::Assign))
            .then(expr.clone())
            .map(|(name, expr)| ast::Expr::Assign {
                name,
                expr: Box::new(expr),
            });

        let let_binding = ident
            .then_ignore(just(Token::Colon))
            .then(type_id)
            .then(just(Token::Assign).ignore_then(expr.clone()).or_not())
            .map(|((name, typ), init)| ast::LetBinding { name, typ, init });

        let let_expr = just(Token::Let)
            .ignore_then(
                let_binding
                    .separated_by(just(Token::Comma))
                    .at_least(1)
                    .collect(),
            )
            .then_ignore(just(Token::In))
            .then(expr.clone().padded_by(just(Token::Comment).repeated()))
            .map(|(bindings, body)| ast::Expr::Let {
                bindings,
                body: Box::new(body),
            });

        let if_expr = just(Token::If)
            .ignore_then(expr.clone().padded_by(just(Token::Comment).repeated()))
            .then_ignore(just(Token::Then))
            .then(expr.clone().padded_by(just(Token::Comment).repeated()))
            .then_ignore(just(Token::Else))
            .then(expr.clone().padded_by(just(Token::Comment).repeated()))
            .then_ignore(just(Token::Fi))
            .map(|((cond, then_branch), else_branch)| ast::Expr::If {
                cond: Box::new(cond),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            });

        let while_expr = just(Token::While)
            .ignore_then(expr.clone().padded_by(just(Token::Comment).repeated()))
            .then_ignore(just(Token::Loop))
            .then(expr.clone().padded_by(just(Token::Comment).repeated()))
            .then_ignore(just(Token::Pool))
            .map(|(cond, body)| ast::Expr::While {
                cond: Box::new(cond),
                body: Box::new(body),
            });

        let case_branch = ident
            .then_ignore(just(Token::Colon))
            .then(type_id)
            .then_ignore(just(Token::DoubleArrow))
            .then(expr.clone())
            .map(|((name, typ), expr)| ast::CaseBranch { name, typ, expr });

        let case_expr = just(Token::Case)
            .ignore_then(expr.clone().padded_by(just(Token::Comment).repeated()))
            .then_ignore(just(Token::Of))
            .then(
                case_branch
                    .separated_by(just(Token::Semicolon))
                    .allow_trailing()
                    .at_least(1)
                    .collect(),
            )
            .then_ignore(just(Token::Esac))
            .map(|(expr, branches)| ast::Expr::Case {
                expr: Box::new(expr),
                branches,
            });

        let atom = choice((
            select! { Token::String(s) => ast::Expr::String(s) },
            select! { Token::Integer(i) => ast::Expr::Integer(i) },
            just(Token::True).to(ast::Expr::True),
            just(Token::False).to(ast::Expr::False),
            just(Token::Isvoid)
                .ignore_then(expr.clone())
                .map(|e| ast::Expr::IsVoid(Box::new(e))),
            just(Token::SelfLit).to(ast::Expr::Id("self".to_string())),
            assign,
            let_expr,
            if_expr,
            while_expr,
            case_expr,
            ident.map(ast::Expr::Id),
            new_expr,
            block,
            expr.clone()
                .delimited_by(just(Token::LeftParen), just(Token::RightParen)),
        ))
        .padded_by(just(Token::Comment).repeated());

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
                .map(|(method, args)| (None, method, args))
                .or(just(Token::TypeId)
                    .ignore_then(type_id)
                    .then_ignore(just(Token::Dot))
                    .then(ident)
                    .then(
                        expr.clone()
                            .separated_by(just(Token::Comma))
                            .allow_trailing()
                            .collect::<Vec<_>>()
                            .delimited_by(just(Token::LeftParen), just(Token::RightParen)),
                    )
                    .map(|((static_type, method), args)| (Some(static_type), method, args)))
                .repeated(),
            |expr, (static_type, method, args)| ast::Expr::Dispatch {
                expr: Box::new(expr),
                static_type,
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

        // Binary operators with precedence
        let factor = choice((call, term));

        let unary = just(Token::Not)
            .or(just(Token::Tilde))
            .repeated()
            .foldr(factor, |op, expr| match op {
                Token::Not => ast::Expr::Not(Box::new(expr)),
                Token::Tilde => ast::Expr::Not(Box::new(expr)), // Use Not for now, could add Negate later
                _ => unreachable!(),
            });

        let multiplicative = unary.clone().foldl(
            choice((just(Token::Multiply), just(Token::Divide)))
                .padded_by(just(Token::Comment).repeated())
                .then(unary.clone())
                .repeated(),
            |lhs, (op, rhs)| match op {
                Token::Multiply => ast::Expr::Times(Box::new(lhs), Box::new(rhs)),
                Token::Divide => ast::Expr::Divide(Box::new(lhs), Box::new(rhs)),
                _ => unreachable!(),
            },
        );

        let additive = multiplicative.clone().foldl(
            choice((just(Token::Plus), just(Token::Minus)))
                .padded_by(just(Token::Comment).repeated())
                .then(multiplicative.clone())
                .repeated(),
            |lhs, (op, rhs)| match op {
                Token::Plus => ast::Expr::Plus(Box::new(lhs), Box::new(rhs)),
                Token::Minus => ast::Expr::Minus(Box::new(lhs), Box::new(rhs)),
                _ => unreachable!(),
            },
        );

        let comparison = additive.clone().foldl(
            choice((
                just(Token::LessThan),
                just(Token::LessThanOrEqual),
                just(Token::Equal),
            ))
            .padded_by(just(Token::Comment).repeated())
            .then(additive.clone())
            .repeated(),
            |lhs, (op, rhs)| match op {
                Token::LessThan => ast::Expr::Lt(Box::new(lhs), Box::new(rhs)),
                Token::LessThanOrEqual => ast::Expr::Le(Box::new(lhs), Box::new(rhs)),
                Token::Equal => ast::Expr::Eq(Box::new(lhs), Box::new(rhs)),
                _ => unreachable!(),
            },
        );

        comparison
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
        .padded_by(just(Token::Comment).repeated())
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

    let attribute_feature = ident
        .then_ignore(just(Token::Colon))
        .then(type_id)
        .then(just(Token::Assign).ignore_then(expr.clone()).or_not())
        .map(|((name, attr_type), init)| {
            ast::Feature::Attribute(ast::AttributeFeature {
                name,
                attr_type,
                init,
            })
        });

    let feature = choice((method_feature, attribute_feature)).then_ignore(just(Token::Semicolon));

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

    just(Token::Comment)
        .repeated()
        .ignore_then(class)
        .separated_by(just(Token::Comment).repeated())
        .allow_trailing()
        .collect()
        .then_ignore(just(Token::Comment).repeated())
        .map(|classes| ast::Program { classes })
}
