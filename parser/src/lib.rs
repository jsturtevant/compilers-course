pub mod ast;

use chumsky::{input::Stream, prelude::*};
use lexer::Token;
use logos::Logos;

pub use ast::{
    Program, Class, Feature, MethodFeature, AttributeFeature,
    Formal, Expr, LetBinding, CaseBranch,
};

/// Parse a COOL source string into a Program AST.
/// Returns a vector of parse errors on failure.
pub fn parse_source(src: &str) -> Result<Program, Vec<String>> {
    let token_iter = Token::lexer(src)
        .spanned()
        .map(|(tok, span)| match tok {
            Ok(tok) => (tok, SimpleSpan::from(span)),
            Err(()) => (Token::Error, span.into()),
        });
    let token_stream = Stream::from_iter(token_iter)
        .map((0..src.len()).into(), |(t, s): (_, _)| (t, s));

    parser_internal()
        .parse(token_stream)
        .into_result()
        .map_err(|errors| errors.iter().map(|e| format!("{:?}", e)).collect())
}

// Internal parser combinator - mirrors the one in main.rs
fn parser_internal<'tokens, I>() -> impl chumsky::Parser<'tokens, I, ast::Program, extra::Err<Rich<'tokens, Token>>>
where
    I: chumsky::input::ValueInput<'tokens, Token = Token, Span = SimpleSpan>,
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
                    .padded_by(just(Token::Comment).repeated())
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

        let call = ident
            .then(
                expr.clone()
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LeftParen), just(Token::RightParen)),
            )
            .map(|(name, args)| ast::Expr::FuncCall { name, args });

        // term can start with either atom or a function call, and both can be followed by dispatch chains
        let term = choice((call, atom)).foldl(
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

        let factor = term;

        // Arithmetic negation (~) has high precedence, same as other unary operators
        let arithmetic_negate = just(Token::Tilde)
            .repeated()
            .foldr(factor, |_op, expr| ast::Expr::Negate(Box::new(expr)));

        let multiplicative = arithmetic_negate.clone().foldl(
            choice((just(Token::Multiply), just(Token::Divide)))
                .padded_by(just(Token::Comment).repeated())
                .then(arithmetic_negate.clone())
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

        // Logical not has low precedence - lower than comparison operators
        // so that "not x = y" parses as "not (x = y)" not "(not x) = y"
        let logical_not = just(Token::Not)
            .repeated()
            .foldr(comparison, |_op, expr| ast::Expr::Not(Box::new(expr)));

        logical_not
    });

    let formal = ident
        .then_ignore(just(Token::Colon))
        .then(type_id)
        .map(|(name, typ)| ast::Formal { name, typ });

    // Parse features: need to distinguish method (has parens) from attribute (no parens)
    // We parse the identifier, then check if next token is LeftParen (method) or Colon (attribute)
    let feature = ident
        .then(choice((
            // Method: ident ( formals ) : Type { body }
            formal
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::LeftParen), just(Token::RightParen))
                .then_ignore(just(Token::Colon))
                .then(type_id)
                .padded_by(just(Token::Comment).repeated())
                .then(
                    expr.clone()
                        .delimited_by(just(Token::LeftBrace), just(Token::RightBrace)),
                )
                .map(|((formals, return_type), body)| (true, formals, return_type, body, None)),
            // Attribute: ident : Type [ <- expr ]
            just(Token::Colon)
                .ignore_then(type_id)
                .then(just(Token::Assign).ignore_then(expr.clone()).or_not())
                .map(|(attr_type, init)| (false, vec![], attr_type, ast::Expr::Integer(0), init)),
        )))
        .map(|(name, (is_method, formals, typ, body, init))| {
            if is_method {
                ast::Feature::Method(ast::MethodFeature {
                    name,
                    formals,
                    return_type: typ,
                    body,
                })
            } else {
                ast::Feature::Attribute(ast::AttributeFeature {
                    name,
                    attr_type: typ,
                    init,
                })
            }
        })
        .then_ignore(just(Token::Semicolon));

     let class = just(Token::Class)
        .ignore_then(type_id)
        .then(just(Token::Inherits).ignore_then(type_id).or_not())
        .then(
            feature
                .padded_by(just(Token::Comment).repeated())
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
