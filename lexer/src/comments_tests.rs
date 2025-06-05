#[cfg(test)]
mod tests {
    use crate::lexer::Token;
    use logos::Logos;

    #[test]
    fn test_comment() {
        let mut lex = Token::lexer("(*comment*)");
        assert_eq!(lex.next(), Some(Ok(Token::Comment)));
        assert_eq!(lex.slice(), "(*comment*)");
    }

    #[test]
    fn test_comment_with_numbers() {
        let mut lex = Token::lexer("(*12345*)");
        assert_eq!(lex.next(), Some(Ok(Token::Comment)));
        assert_eq!(lex.slice(), "(*12345*)");
    }

    #[test]
    fn test_comment_with_mixed_characters() {
        let mut lex = Token::lexer("(*a1b2c3*)");
        assert_eq!(lex.next(), Some(Ok(Token::Comment)));
        assert_eq!(lex.slice(), "(*a1b2c3*)");
    }

    #[test]
    fn test_comment_with_special_characters() {
        let mut lex = Token::lexer("(*a1!@#$%^&*()b2c3*)");
        assert_eq!(lex.next(), Some(Ok(Token::Comment)));
        assert_eq!(lex.slice(), "(*a1!@#$%^&*()b2c3*)");
    }

    #[test]
    fn test_single_line_comment() {
        let mut lex = Token::lexer("-- This is a comment\n");
        assert_eq!(lex.next(), Some(Ok(Token::Comment)));
        assert_eq!(lex.slice(), "-- This is a comment");
    }

    #[test]
    fn test_single_line_comment_no_space() {
        let mut lex = Token::lexer("--Comment");
        assert_eq!(lex.next(), Some(Ok(Token::Comment)));
        assert_eq!(lex.slice(), "--Comment");
    }

    #[test]
    fn test_single_line_comment_with_numbers() {
        let mut lex = Token::lexer("--12345\n");
        assert_eq!(lex.next(), Some(Ok(Token::Comment)));
        assert_eq!(lex.slice(), "--12345");
    }

    #[test]
    fn test_single_line_comment_with_special_characters() {
        let mut lex = Token::lexer("--!@#$%^&*()\n");
        assert_eq!(lex.next(), Some(Ok(Token::Comment)));
        assert_eq!(lex.slice(), "--!@#$%^&*()");
    }

    #[test]
    fn test_nested_comments_one_level() {
        let mut lex = Token::lexer("(* outer comment (* nested comment *) still outer *)");
        assert_eq!(lex.next(), Some(Ok(Token::Comment)));
        assert_eq!(lex.slice(), "(* outer comment (* nested comment *) still outer *)");
    }

    #[test]
    fn test_nested_comments_multiple_levels() {
        let mut lex = Token::lexer("(* level 1 (* level 2 (* level 3 *) more level 2 *) more level 1 *)");
        assert_eq!(lex.next(), Some(Ok(Token::Comment)));
        assert_eq!(lex.slice(), "(* level 1 (* level 2 (* level 3 *) more level 2 *) more level 1 *)");
    }

    #[test]
    fn test_comments_with_newlines() {
        let mut lex = Token::lexer("(* Comment with\nnew lines\ninside it *)");
        assert_eq!(lex.next(), Some(Ok(Token::Comment)));
        assert_eq!(lex.slice(), "(* Comment with\nnew lines\ninside it *)");
    }

    #[test]
    fn test_nested_comments_with_newlines() {
        let mut lex = Token::lexer("(* Outer comment\n   (* Nested\n      comment *)\n   More outer\n*)");
        assert_eq!(lex.next(), Some(Ok(Token::Comment)));
        assert_eq!(lex.slice(), "(* Outer comment\n   (* Nested\n      comment *)\n   More outer\n*)");
    }

    #[test]
    fn test_unclosed_comment() {
        let mut lex = Token::lexer("(* This comment is not closed");
        lex.next().unwrap().unwrap_err();
    }

    #[test]
    fn test_comment_with_code_after() {
        let mut lex = Token::lexer("(* Comment *) class");
        assert_eq!(lex.next(), Some(Ok(Token::Comment)));
        assert_eq!(lex.next(), Some(Ok(Token::Class)));
    }

    #[test]
    fn test_nested_comment_with_code_after() {
        let mut lex = Token::lexer("(* Outer (* nested *) comment *) if");
        assert_eq!(lex.next(), Some(Ok(Token::Comment)));
        assert_eq!(lex.next(), Some(Ok(Token::If)));
    }

}
