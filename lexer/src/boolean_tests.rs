#[cfg(test)]
mod test {
    use crate::lexer::Token;
    use logos::Logos;

    #[test]
    fn test_true() {
        let mut lex = Token::lexer("true");
        assert_eq!(lex.next(), Some(Ok(Token::True)));
        assert_eq!(lex.slice(), "true");
    }

    #[test]
    fn test_capital_true_fails() {
        let mut lex = Token::lexer("True");

        assert_ne!(lex.next(), Some(Ok(Token::True)));
    }

    #[test]
    fn test_true_case_insensitive() {
        let mut lex = Token::lexer("trUe");
        assert_eq!(lex.next(), Some(Ok(Token::True)));
        assert_eq!(lex.slice(), "trUe");
    }

    #[test]
    fn test_false() {
        let mut lex = Token::lexer("false");
        assert_eq!(lex.next(), Some(Ok(Token::False)));
        assert_eq!(lex.slice(), "false");
    }

    #[test]
    fn test_false_case_insensitive() {
        let mut lex = Token::lexer("fAlSe");
        assert_eq!(lex.next(), Some(Ok(Token::False)));
        assert_eq!(lex.slice(), "fAlSe");
    }
}
