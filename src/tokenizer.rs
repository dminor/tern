use std::error::Error;
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
    Rel,

    // Literals
    Identifier(String),
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
            TokenKind::Identifier(s) => write!(f, "{}", s),
            TokenKind::Disj => write!(f, "disj"),
            TokenKind::Conj => write!(f, "conj"),
            TokenKind::Var => write!(f, "var"),
            TokenKind::Rel => write!(f, "rel"),
        }
    }
}

pub struct Token {
    pub kind: TokenKind,
    pub offset: usize,
}

#[derive(Debug)]
pub struct TokenizerError {
    pub err: String,
    pub offset: usize,
}

impl fmt::Display for TokenizerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TokenizerError: {}", self.err)
    }
}

impl Error for TokenizerError {}

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
                        err: "Unexpected token while scanning `==`".to_string(),
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
                    "rel" => tokens.push(Token {
                        kind: TokenKind::Rel,
                        offset,
                    }),
                    _ => tokens.push(Token {
                        kind: TokenKind::Identifier(s),
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
                    assert_eq!(e.err, $err);
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
            TokenKind::Identifier("olive".to_string())
        );
        scan!(
            "rel () {
                'olive == 'oil
            }({}).next()",
            TokenKind::Rel,
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::Tick,
            TokenKind::Identifier("olive".to_string()),
            TokenKind::DoubleEquals,
            TokenKind::Tick,
            TokenKind::Identifier("oil".to_string()),
            TokenKind::RightBrace,
            TokenKind::LeftParen,
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
            TokenKind::RightParen,
            TokenKind::Dot,
            TokenKind::Identifier("next".to_string()),
            TokenKind::LeftParen,
            TokenKind::RightParen
        );
        scan!(
            "['olive, 'oil] == ['olive, q]",
            TokenKind::LeftBracket,
            TokenKind::Tick,
            TokenKind::Identifier("olive".to_string()),
            TokenKind::Comma,
            TokenKind::Tick,
            TokenKind::Identifier("oil".to_string()),
            TokenKind::RightBracket,
            TokenKind::DoubleEquals,
            TokenKind::LeftBracket,
            TokenKind::Tick,
            TokenKind::Identifier("olive".to_string()),
            TokenKind::Comma,
            TokenKind::Identifier("q".to_string()),
            TokenKind::RightBracket
        );
        scan!(
            "disj { p == 'red | p == 'bean }",
            TokenKind::Disj,
            TokenKind::LeftBrace,
            TokenKind::Identifier("p".to_string()),
            TokenKind::DoubleEquals,
            TokenKind::Tick,
            TokenKind::Identifier("red".to_string()),
            TokenKind::Pipe,
            TokenKind::Identifier("p".to_string()),
            TokenKind::DoubleEquals,
            TokenKind::Tick,
            TokenKind::Identifier("bean".to_string()),
            TokenKind::RightBrace
        );
        scan!(
            "rel Mother(x) {
                var y
                conj {
                  Female(x), Parent(y, x)
                }
              }",
            TokenKind::Rel,
            TokenKind::Identifier("Mother".to_string()),
            TokenKind::LeftParen,
            TokenKind::Identifier("x".to_string()),
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::Var,
            TokenKind::Identifier("y".to_string()),
            TokenKind::Conj,
            TokenKind::LeftBrace,
            TokenKind::Identifier("Female".to_string()),
            TokenKind::LeftParen,
            TokenKind::Identifier("x".to_string()),
            TokenKind::RightParen,
            TokenKind::Comma,
            TokenKind::Identifier("Parent".to_string()),
            TokenKind::LeftParen,
            TokenKind::Identifier("y".to_string()),
            TokenKind::Comma,
            TokenKind::Identifier("x".to_string()),
            TokenKind::RightParen,
            TokenKind::RightBrace,
            TokenKind::RightBrace
        );
        scanfails!("=", "Unexpected token while scanning `==`", 1);
    }
}
