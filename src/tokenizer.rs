use crate::errors::TokenizerError;
use std::fmt;

#[derive(Debug, Eq, PartialEq)]
pub enum TokenKind {
    //Symbols
    Comma,
    Dot,
    DoubleEquals,
    LeftBrace,
    LeftBracket,
    LeftParen,
    Pipe,
    RightBrace,
    RightBracket,
    RightParen,
    Tick,

    // Keywords
    Conj,
    Disj,
    Var,

    // Literals
    Literal(String),
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenKind::Comma => write!(f, ","),
            TokenKind::DoubleEquals => write!(f, "=="),
            TokenKind::Dot => write!(f, "."),
            TokenKind::LeftBrace => write!(f, "{{"),
            TokenKind::LeftBracket => write!(f, "["),
            TokenKind::LeftParen => write!(f, "("),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::RightBrace => write!(f, "}}"),
            TokenKind::RightBracket => write!(f, "]"),
            TokenKind::RightParen => write!(f, ")"),
            TokenKind::Tick => write!(f, "'"),
            TokenKind::Disj => write!(f, "disj"),
            TokenKind::Conj => write!(f, "conj"),
            TokenKind::Var => write!(f, "var"),
            TokenKind::Literal(s) => write!(f, "{}", s),
        }
    }
}

pub struct Token {
    pub kind: TokenKind,
    pub offset: usize,
}

pub fn scan(src: &str) -> Result<Vec<Token>, TokenizerError> {
    let mut offset = 0;
    let mut tokens = Vec::<Token>::new();
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ',' => tokens.push(Token {
                kind: TokenKind::Comma,
                offset,
            }),
            '.' => tokens.push(Token {
                kind: TokenKind::Dot,
                offset,
            }),
            '=' => {
                if let Some('=') = chars.peek() {
                    tokens.push(Token {
                        kind: TokenKind::DoubleEquals,
                        offset,
                    });
                    chars.next();
                    offset += 1;
                } else {
                    return Err(TokenizerError {
                        msg: "Unexpected token while scanning `==`".to_string(),
                        offset: offset + 1,
                    });
                }
            }
            '{' => tokens.push(Token {
                kind: TokenKind::LeftBrace,
                offset,
            }),
            '[' => tokens.push(Token {
                kind: TokenKind::LeftBracket,
                offset,
            }),
            '(' => tokens.push(Token {
                kind: TokenKind::LeftParen,
                offset,
            }),
            '|' => tokens.push(Token {
                kind: TokenKind::Pipe,
                offset,
            }),
            '}' => tokens.push(Token {
                kind: TokenKind::RightBrace,
                offset,
            }),
            ']' => tokens.push(Token {
                kind: TokenKind::RightBracket,
                offset,
            }),
            ')' => tokens.push(Token {
                kind: TokenKind::RightParen,
                offset,
            }),
            '\'' => tokens.push(Token {
                kind: TokenKind::Tick,
                offset,
            }),
            '\n' | ' ' => {}
            _ => {
                let mut v = vec![c];
                while let Some(c) = chars.peek() {
                    if c.is_alphanumeric() {
                        v.push(*c);
                        chars.next();
                        offset += 1;
                    } else {
                        break;
                    }
                }
                let s: String = v.into_iter().collect();
                match &s[..] {
                    "conj" => tokens.push(Token {
                        kind: TokenKind::Conj,
                        offset,
                    }),
                    "disj" => tokens.push(Token {
                        kind: TokenKind::Disj,
                        offset,
                    }),
                    "var" => tokens.push(Token {
                        kind: TokenKind::Var,
                        offset,
                    }),
                    _ => tokens.push(Token {
                        kind: TokenKind::Literal(s),
                        offset,
                    }),
                }
            }
        }
        offset += 1;
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use crate::tokenizer::*;

    macro_rules! scan {
        ($input:expr, $( $value:expr),* ) => {{
            match scan($input) {
                Ok(mut tokens) => {
                    tokens.reverse();
                    $(
                        match tokens.pop() {
                            Some(t) => {
                                assert_eq!(t.kind, $value);
                            }
                            None => {}
                        }
                    )*
                    assert_eq!(tokens.len(), 0);
                }
                _ => assert!(false),
            }
        }};
    }

    macro_rules! scanfails {
        ($input:expr, $err:tt, $offset:expr) => {{
            match scan($input) {
                Ok(_) => assert!(false),
                Err(e) => {
                    assert_eq!(e.msg, $err);
                    assert_eq!(e.offset, $offset);
                }
            }
        }};
    }

    #[test]
    fn scanning() {
        scan!(
            "'olive",
            TokenKind::Tick,
            TokenKind::Literal("olive".to_string())
        );
        scan!(
            "'olive 'oil",
            TokenKind::Tick,
            TokenKind::Literal("olive".to_string()),
            TokenKind::Tick,
            TokenKind::Literal("oil".to_string())
        );
        scan!(
            "'olive == 'oil({}).next()",
            TokenKind::Tick,
            TokenKind::Literal("olive".to_string()),
            TokenKind::DoubleEquals,
            TokenKind::Tick,
            TokenKind::Literal("oil".to_string()),
            TokenKind::LeftParen,
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
            TokenKind::RightParen,
            TokenKind::Dot,
            TokenKind::Literal("next".to_string()),
            TokenKind::LeftParen,
            TokenKind::RightParen
        );
        scan!(
            "['olive, 'oil] == ['olive, q]",
            TokenKind::LeftBracket,
            TokenKind::Tick,
            TokenKind::Literal("olive".to_string()),
            TokenKind::Comma,
            TokenKind::Tick,
            TokenKind::Literal("oil".to_string()),
            TokenKind::RightBracket,
            TokenKind::DoubleEquals,
            TokenKind::LeftBracket,
            TokenKind::Tick,
            TokenKind::Literal("olive".to_string()),
            TokenKind::Comma,
            TokenKind::Literal("q".to_string()),
            TokenKind::RightBracket
        );
        scan!(
            "disj { p == 'red | p == 'bean }",
            TokenKind::Disj,
            TokenKind::LeftBrace,
            TokenKind::Literal("p".to_string()),
            TokenKind::DoubleEquals,
            TokenKind::Tick,
            TokenKind::Literal("red".to_string()),
            TokenKind::Pipe,
            TokenKind::Literal("p".to_string()),
            TokenKind::DoubleEquals,
            TokenKind::Tick,
            TokenKind::Literal("bean".to_string()),
            TokenKind::RightBrace
        );
        scan!(
            "conj {
                Female(x), Parent(y, x)
            }",
            TokenKind::Conj,
            TokenKind::LeftBrace,
            TokenKind::Literal("Female".to_string()),
            TokenKind::LeftParen,
            TokenKind::Literal("x".to_string()),
            TokenKind::RightParen,
            TokenKind::Comma,
            TokenKind::Literal("Parent".to_string()),
            TokenKind::LeftParen,
            TokenKind::Literal("y".to_string()),
            TokenKind::Comma,
            TokenKind::Literal("x".to_string()),
            TokenKind::RightParen,
            TokenKind::RightBrace
        );
        scanfails!("=", "Unexpected token while scanning `==`", 1);
        scan!(
            "var (q) { 'olive == q }",
            TokenKind::Var,
            TokenKind::LeftParen,
            TokenKind::Literal("q".to_string()),
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::Tick,
            TokenKind::Literal("olive".to_string()),
            TokenKind::DoubleEquals,
            TokenKind::Literal("q".to_string()),
            TokenKind::RightBrace
        );
        scan!(
            "fn('olive == 'olive)",
            TokenKind::Literal("fn".to_string()),
            TokenKind::LeftParen,
            TokenKind::Tick,
            TokenKind::Literal("olive".to_string()),
            TokenKind::DoubleEquals,
            TokenKind::Tick,
            TokenKind::Literal("olive".to_string()),
            TokenKind::RightParen
        );
    }
}
