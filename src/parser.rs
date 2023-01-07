use crate::tokenizer::{Token, TokenKind};
use std::error::Error;
use std::fmt;
use std::iter::Peekable;

pub enum AST {
    Conj(Vec<AST>),
    Disj(Vec<AST>),
    Equals(Box<AST>, Box<AST>),
    Atom(String),
}

impl fmt::Display for AST {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AST::Conj(terms) => {
                write!(f, "Conj {{")?;
                let mut first = true;
                for term in terms {
                    if !first {
                        write!(f, ", {}", term)?;
                    } else {
                        first = false;
                        write!(f, "{}", term)?;
                    }
                }
                write!(f, "}}")
            }
            AST::Disj(terms) => {
                write!(f, "Disj {{")?;
                let mut first = true;
                for term in terms {
                    if !first {
                        write!(f, ", {}", term)?;
                    } else {
                        first = false;
                        write!(f, "{}", term)?;
                    }
                }
                write!(f, "}}")
            }
            AST::Equals(left, right) => write!(f, "{} == {}", left, right),
            AST::Atom(atom) => write!(f, "'{}", atom),
        }
    }
}

#[derive(Debug)]
pub struct ParseError {
    pub msg: String,
    pub offset: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ParserError: {}", self.msg)
    }
}

impl Error for ParseError {}

struct ParseState {
    offset: usize,
}

fn goal(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, ParseError> {
    if let Some(token) = tokens.peek() {
        match token.kind {
            TokenKind::Conj => {
                todo!()
            }
            TokenKind::Disj => {
                todo!()
            }
            TokenKind::Tick => equals(state, tokens),
            _ => Err(ParseError {
                msg: "Expected conj, disj or equals while parsing goal.".to_string(),
                offset: state.offset,
            }),
        }
    } else {
        Err(ParseError {
            msg: "Unexpected end of input while parsing goal.".to_string(),
            offset: state.offset,
        })
    }
}

fn equals(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, ParseError> {
    let left = term(state, tokens)?;
    if let Some(token) = tokens.next() {
        if token.kind == TokenKind::DoubleEquals {
            state.offset = token.offset;
            let right = term(state, tokens)?;
            Ok(AST::Equals(Box::new(left), Box::new(right)))
        } else {
            Err(ParseError {
                msg: "Expected `==` while parsing equals.".to_string(),
                offset: state.offset,
            })
        }
    } else {
        Err(ParseError {
            msg: "Unexpected end of input while parsing equals.".to_string(),
            offset: state.offset,
        })
    }
}

fn term(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, ParseError> {
    atom(state, tokens)
}

fn atom(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, ParseError> {
    match tokens.next() {
        Some(token) => {
            if token.kind == TokenKind::Tick {
                state.offset = token.offset;
                match tokens.next() {
                    Some(token) => {
                        state.offset = token.offset;
                        if let TokenKind::Literal(id) = token.kind {
                            Ok(AST::Atom(id))
                        } else {
                            Err(ParseError {
                                msg: "Expected identifier while parsing atom.".to_string(),
                                offset: state.offset,
                            })
                        }
                    }
                    None => Err(ParseError {
                        msg: "Unexpected end of input while parsing atom.".to_string(),
                        offset: state.offset,
                    }),
                }
            } else {
                Err(ParseError {
                    msg: "Expected `'` while parsing atom.".to_string(),
                    offset: state.offset,
                })
            }
        }
        None => Err(ParseError {
            msg: "Unexpected end of input while parsing atom.".to_string(),
            offset: state.offset,
        }),
    }
}

pub fn parse(tokens: Vec<Token>) -> Result<AST, ParseError> {
    let mut state = ParseState { offset: 0 };
    let mut iter = tokens.into_iter().peekable();
    let ast = goal(&mut state, &mut iter);
    if iter.next().is_none() || ast.is_err() {
        ast
    } else {
        Err(ParseError {
            msg: "Trailing input after parsing...".to_string(),
            offset: state.offset,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::parser;
    use crate::tokenizer;

    macro_rules! parse {
        ($input:expr, $value:expr) => {{
            match tokenizer::scan($input) {
                Ok(tokens) => match parser::parse(tokens) {
                    Ok(ast) => {
                        assert_eq!(ast.to_string(), $value);
                    }
                    Err(err) => assert_eq!("parse failed", err.msg),
                },
                _ => assert!(false),
            }
        }};
    }

    macro_rules! parsefails {
        ($input:expr, $msg:tt, $offset:expr) => {{
            match tokenizer::scan($input) {
                Ok(tokens) => match parser::parse(tokens) {
                    Ok(_) => assert!(false),
                    Err(e) => {
                        assert_eq!(e.msg, $msg);
                        assert_eq!(e.offset, $offset);
                    }
                },
                _ => assert!(false),
            }
        }};
    }

    #[test]
    fn parsing() {
        parse!("'olive == 'olive", "'olive == 'olive");
        parse!("'0live == '0live", "'0live == '0live");
        parsefails!("'", "Unexpected end of input while parsing atom.", 0);
        parsefails!(
            "olive",
            "Expected conj, disj or equals while parsing goal.",
            0
        );
        parsefails!(
            "'olive ==",
            "Unexpected end of input while parsing atom.",
            7
        );
        parsefails!("'olive", "Unexpected end of input while parsing equals.", 5);
        parsefails!("'olive 'oil", "Expected `==` while parsing equals.", 5);
    }
}
