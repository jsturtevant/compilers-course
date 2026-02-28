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

        // Binary operators with precedence
        let factor = term;

        let unary = just(Token::Not)
            .or(just(Token::Tilde))
            .repeated()
            .foldr(factor, |op, expr| match op {
                Token::Not => ast::Expr::Not(Box::new(expr)),
                Token::Tilde => ast::Expr::Negate(Box::new(expr)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use ast::*;

    fn parse_program(src: &str) -> Result<Program, Vec<Rich<Token>>> {
        let token_iter = Token::lexer(src).spanned().map(|(tok, span)| match tok {
            Ok(tok) => (tok, SimpleSpan::from(span)),
            Err(()) => (Token::Error, span.into()),
        });
        let token_stream =
            Stream::from_iter(token_iter).map((0..src.len()).into(), |(t, s): (_, _)| (t, s));

        parser().parse(token_stream).into_result()
    }

    // === Class and Feature Tests ===

    #[test]
    fn test_simple_class() {
        let src = "class Main { };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        assert_eq!(prog.classes.len(), 1);
        assert_eq!(prog.classes[0].name, "Main");
        assert!(prog.classes[0].parent.is_none());
        assert_eq!(prog.classes[0].features.len(), 0);
    }

    #[test]
    fn test_class_with_inheritance() {
        let src = "class Child inherits Parent { };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        assert_eq!(prog.classes[0].name, "Child");
        assert_eq!(prog.classes[0].parent, Some("Parent".to_string()));
    }

    #[test]
    fn test_method_feature() {
        let src = "class Main { foo():Int { 42 }; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        assert_eq!(prog.classes[0].features.len(), 1);
        match &prog.classes[0].features[0] {
            Feature::Method(m) => {
                assert_eq!(m.name, "foo");
                assert_eq!(m.return_type, "Int");
                assert_eq!(m.formals.len(), 0);
                matches!(m.body, Expr::Integer(42));
            }
            _ => panic!("Expected method"),
        }
    }

    #[test]
    fn test_method_with_parameters() {
        let src = "class Main { add(x:Int, y:Int):Int { x }; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Method(m) => {
                assert_eq!(m.formals.len(), 2);
                assert_eq!(m.formals[0].name, "x");
                assert_eq!(m.formals[0].typ, "Int");
                assert_eq!(m.formals[1].name, "y");
                assert_eq!(m.formals[1].typ, "Int");
            }
            _ => panic!("Expected method"),
        }
    }

    #[test]
    fn test_attribute_feature() {
        let src = "class Main { x:Int; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Attribute(a) => {
                assert_eq!(a.name, "x");
                assert_eq!(a.attr_type, "Int");
                assert!(a.init.is_none());
            }
            _ => panic!("Expected attribute"),
        }
    }

    #[test]
    fn test_attribute_with_init() {
        let src = "class Main { x:Int <- 42; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Attribute(a) => {
                assert_eq!(a.name, "x");
                assert!(a.init.is_some());
            }
            _ => panic!("Expected attribute"),
        }
    }

    // === Operator Precedence Tests ===

    #[test]
    fn test_addition() {
        let src = "class Main { main():Int { 1 + 2 }; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Method(m) => matches!(m.body, Expr::Plus(_, _)),
            _ => panic!("Expected method"),
        };
    }

    #[test]
    fn test_multiplication_precedence() {
        // 1 + 2 * 3 should parse as 1 + (2 * 3)
        let src = "class Main { main():Int { 1 + 2 * 3 }; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Method(m) => match &m.body {
                Expr::Plus(left, right) => {
                    matches!(**left, Expr::Integer(1));
                    matches!(**right, Expr::Times(_, _));
                }
                _ => panic!("Expected plus with times on right"),
            },
            _ => panic!("Expected method"),
        }
    }

    #[test]
    fn test_left_associativity() {
        // 1 + 2 + 3 should parse as (1 + 2) + 3
        let src = "class Main { main():Int { 1 + 2 + 3 }; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Method(m) => match &m.body {
                Expr::Plus(left, right) => {
                    matches!(**left, Expr::Plus(_, _));
                    matches!(**right, Expr::Integer(3));
                }
                _ => panic!("Expected left-associative plus"),
            },
            _ => panic!("Expected method"),
        }
    }

    #[test]
    fn test_comparison_lower_precedence() {
        // 1 + 2 < 3 + 4 should parse as (1 + 2) < (3 + 4)
        let src = "class Main { main():Bool { 1 + 2 < 3 + 4 }; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Method(m) => match &m.body {
                Expr::Lt(left, right) => {
                    matches!(**left, Expr::Plus(_, _));
                    matches!(**right, Expr::Plus(_, _));
                }
                _ => panic!("Expected comparison with plus on both sides"),
            },
            _ => panic!("Expected method"),
        }
    }

    #[test]
    fn test_unary_not() {
        let src = "class Main { main():Bool { not true }; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Method(m) => matches!(m.body, Expr::Not(_)),
            _ => panic!("Expected method"),
        };
    }

    #[test]
    fn test_unary_tilde() {
        let src = "class Main { main():Int { ~42 }; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Method(m) => matches!(m.body, Expr::Not(_)),
            _ => panic!("Expected method"),
        };
    }

    // === Control Flow Tests ===

    #[test]
    fn test_if_expression() {
        let src = "class Main { main():Int { if true then 1 else 2 fi }; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Method(m) => match &m.body {
                Expr::If {
                    cond,
                    then_branch,
                    else_branch,
                } => {
                    matches!(**cond, Expr::True);
                    matches!(**then_branch, Expr::Integer(1));
                    matches!(**else_branch, Expr::Integer(2));
                }
                _ => panic!("Expected if expression"),
            },
            _ => panic!("Expected method"),
        }
    }

    #[test]
    fn test_while_expression() {
        let src = "class Main { main():Object { while true loop 42 pool }; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Method(m) => match &m.body {
                Expr::While { cond, body } => {
                    matches!(**cond, Expr::True);
                    matches!(**body, Expr::Integer(42));
                }
                _ => panic!("Expected while expression"),
            },
            _ => panic!("Expected method"),
        }
    }

    #[test]
    fn test_block_expression() {
        let src = "class Main { main():Int { { 1; 2; 3 } }; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Method(m) => match &m.body {
                Expr::Block(exprs) => {
                    assert_eq!(exprs.len(), 3);
                }
                _ => panic!("Expected block"),
            },
            _ => panic!("Expected method"),
        }
    }

    #[test]
    fn test_let_expression() {
        let src = "class Main { main():Int { let x:Int in x }; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Method(m) => match &m.body {
                Expr::Let { bindings, .. } => {
                    assert_eq!(bindings.len(), 1);
                    assert_eq!(bindings[0].name, "x");
                    assert_eq!(bindings[0].typ, "Int");
                }
                _ => panic!("Expected let expression"),
            },
            _ => panic!("Expected method"),
        }
    }

    #[test]
    fn test_case_expression() {
        let src = "class Main { main():Int { case x of y:Int => 1; esac }; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Method(m) => match &m.body {
                Expr::Case { branches, .. } => {
                    assert_eq!(branches.len(), 1);
                    assert_eq!(branches[0].name, "y");
                    assert_eq!(branches[0].typ, "Int");
                }
                _ => panic!("Expected case expression"),
            },
            _ => panic!("Expected method"),
        }
    }

    // === Assignment and Dispatch Tests ===

    #[test]
    fn test_assignment() {
        let src = "class Main { main():Int { x <- 42 }; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Method(m) => match &m.body {
                Expr::Assign { name, expr } => {
                    assert_eq!(name, "x");
                    matches!(**expr, Expr::Integer(42));
                }
                _ => panic!("Expected assignment"),
            },
            _ => panic!("Expected method"),
        }
    }

    #[test]
    fn test_method_dispatch() {
        let src = "class Main { main():Int { obj.method() }; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Method(m) => match &m.body {
                Expr::Dispatch {
                    method,
                    static_type,
                    args,
                    ..
                } => {
                    assert_eq!(method, "method");
                    assert!(static_type.is_none());
                    assert_eq!(args.len(), 0);
                }
                _ => panic!("Expected dispatch"),
            },
            _ => panic!("Expected method"),
        }
    }

    #[test]
    fn test_static_dispatch() {
        let src = "class Main { main():Int { obj@Type.method() }; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Method(m) => match &m.body {
                Expr::Dispatch {
                    static_type,
                    method,
                    ..
                } => {
                    assert_eq!(static_type, &Some("Type".to_string()));
                    assert_eq!(method, "method");
                }
                _ => panic!("Expected static dispatch"),
            },
            _ => panic!("Expected method"),
        }
    }

    #[test]
    fn test_new_expression() {
        let src = "class Main { main():Int { new Int }; };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        match &prog.classes[0].features[0] {
            Feature::Method(m) => match &m.body {
                Expr::New(typ) => assert_eq!(typ, "Int"),
                _ => panic!("Expected new expression"),
            },
            _ => panic!("Expected method"),
        }
    }

    // === Multiple Classes ===

    #[test]
    fn test_multiple_classes() {
        let src = "class A { }; class B { }; class C { };";
        let result = parse_program(src);
        assert!(result.is_ok());
        let prog = result.unwrap();
        assert_eq!(prog.classes.len(), 3);
    }

    // === Comments ===
    // Note: Comments are handled by lexer and padded around tokens
    // Inline comment tests would require more complex test setup

    // === Error Recovery Tests ===

    #[test]
    fn test_missing_semicolon() {
        let src = "class Main { }"; // Missing semicolon
        let result = parse_program(src);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_fi() {
        let src = "class Main { main():Int { if true then 1 else 2 }; };";
        let result = parse_program(src);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_pool() {
        let src = "class Main { main():Object { while true loop 42 }; };";
        let result = parse_program(src);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_method_no_body() {
        let src = "class Main { foo():Int; };";
        let result = parse_program(src);
        assert!(result.is_err());
    }
}
