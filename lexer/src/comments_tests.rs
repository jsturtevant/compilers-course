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

    //TODO
    // #[test]
    // fn test_comment_with_speical_characters() {
    //     let mut lex = Token::lexer("(*a1!@#$%^&*()b2c3*)");
    //     assert_eq!(lex.next(), Some(Ok(Token::Comment)));
    //     assert_eq!(lex.slice(), "(*a1!@#$%^&*()b2c3*)");
    // }

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

}
