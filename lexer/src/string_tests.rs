#[cfg(test)]
mod tests {
    use crate::lexer::Token;
    use logos::Logos;

    #[test]
    fn test_simple_string() {
        let mut lexer = Token::lexer(r#""Hello, World!""#);
        assert_eq!(lexer.next(), Some(Ok(Token::String)));
    }

    #[test]
    fn test_crazy_string() {
        let mut lexer = Token::lexer(r#""Hello,\ test *!&@#^@$*(!&$) World!""#);
        assert_eq!(lexer.next(), Some(Ok(Token::String)));
    }

    #[test]
    fn test_valid_string() {
        let mut lexer = Token::lexer(
            r#""This \ 
 is OK""#,
        );
        assert_eq!(lexer.next(), Some(Ok(Token::String)));
    }

    #[test]
    fn test_invalid_string() {
        let mut lexer = Token::lexer(
            r#""This is not
 OK""#,
        );
        lexer.next().unwrap().unwrap_err();
    }

    #[test]
    fn test_string_with_null_character() {
        let mut lexer = Token::lexer(r#""This string contains a null character \0""#);
        lexer.next().unwrap().unwrap_err();
    }

    #[test]
    fn test_string_with_newline() {
        let mut lexer = Token::lexer(r#""This string contains a newline character \n""#);
        assert_eq!(lexer.next(), Some(Ok(Token::String)));
    }
}
