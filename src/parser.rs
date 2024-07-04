use crate::errors::SyntaxError;
use crate::tokenizer::{Token, TokenKind};
use std::fmt;
use std::iter::Peekable;

pub enum AST {
    Conj(Vec<AST>),
    Disj(Vec<AST>),
    Equals(Box<AST>, Box<AST>),
    Var(Vec<AST>, Box<AST>),
    Atom(String),
    Variable(String),
    FnCall(String, Vec<AST>, usize),
    Program(Vec<AST>),
    Table(Vec<AST>),
    LetBinding(String, Box<AST>),
    BindingRef(String),
    Relation(Vec<AST>, Box<AST>),
}

impl fmt::Display for AST {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AST::Conj(terms) => {
                write!(f, "conj {{ ")?;
                let mut first = true;
                for term in terms {
                    if !first {
                        write!(f, " , {}", term)?;
                    } else {
                        first = false;
                        write!(f, "{}", term)?;
                    }
                }
                write!(f, " }}")
            }
            AST::Disj(terms) => {
                write!(f, "disj {{ ")?;
                let mut first = true;
                for term in terms {
                    if !first {
                        write!(f, " | {}", term)?;
                    } else {
                        first = false;
                        write!(f, "{}", term)?;
                    }
                }
                write!(f, " }}")
            }
            AST::Equals(left, right) => write!(f, "{} == {}", left, right),
            AST::Var(declarations, body) => {
                write!(f, "var (")?;
                let mut first = true;
                for declaration in declarations {
                    if !first {
                        write!(f, ", {}", declaration)?;
                    } else {
                        first = false;
                        write!(f, "{}", declaration)?;
                    }
                }
                write!(f, ") {{ {} }}", body)
            }
            AST::Atom(atom) => write!(f, "'{}", atom),
            AST::Variable(name) => write!(f, "{}", name),
            AST::FnCall(name, arguments, _) => {
                write!(f, "{}(", name)?;
                let mut first = true;
                for argument in arguments {
                    if !first {
                        write!(f, ", {}", argument)?;
                    } else {
                        first = false;
                        write!(f, "{}", argument)?;
                    }
                }
                write!(f, ")")
            }
            AST::Program(statements) => {
                for statement in statements {
                    write!(f, "{}", statement)?;
                }
                Ok(())
            }
            AST::Table(members) => {
                write!(f, "{{")?;
                let mut i = 0;
                while i < members.len() {
                    write!(f, "{}", members[i])?;
                    write!(f, ": ")?;
                    if i + 2 != members.len() {
                        write!(f, "{}, ", members[i + 1])?;
                    } else {
                        write!(f, "{}", members[i + 1])?;
                    }
                    i += 2;
                }
                write!(f, "}}")
            }
            AST::LetBinding(name, expr) => {
                write!(f, "let {} = {}", name, expr)
            }
            AST::BindingRef(name) => {
                write!(f, "{}", name)
            }
            AST::Relation(parameters, body) => {
                write!(f, "rel(")?;
                let mut first = true;
                for parameter in parameters {
                    if !first {
                        write!(f, ", {}", parameter)?;
                    } else {
                        first = false;
                        write!(f, "{}", parameter)?;
                    }
                }
                write!(f, ") {{ {} }}", body)
            }
        }
    }
}

struct ParseState {
    offset: usize,
}

fn program(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, SyntaxError> {
    let mut statements: Vec<AST> = Vec::new();
    while tokens.peek().is_some() {
        statements.push(statement(state, tokens)?);
    }
    Ok(AST::Program(statements))
}

fn statement(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, SyntaxError> {
    if let Some(token) = tokens.peek() {
        if token.kind == TokenKind::Let {
            letbinding(state, tokens)
        } else {
            expression(state, tokens)
        }
    } else {
        Err(SyntaxError {
            msg: "Unexpected end of input while parsing statement.".to_string(),
            offset: state.offset,
        })
    }
}

fn letbinding(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, SyntaxError> {
    if let Some(token) = tokens.next() {
        if token.kind != TokenKind::Let {
            return Err(SyntaxError {
                msg: "Expected `let`.".to_string(),
                offset: state.offset,
            });
        }
        state.offset = token.offset;
        if let AST::Variable(name) = variable(state, tokens)? {
            if let Some(token) = tokens.next() {
                if token.kind != TokenKind::Equals {
                    return Err(SyntaxError {
                        msg: "Expected `=` while parsing let binding.".to_string(),
                        offset: state.offset,
                    });
                }
                state.offset = token.offset;
            } else {
                return Err(SyntaxError {
                    msg: "Unexpected end of input while parsing let binding.".to_string(),
                    offset: state.offset,
                });
            }
            let value = expression(state, tokens)?;
            Ok(AST::LetBinding(name, Box::new(value)))
        } else {
            Err(SyntaxError {
                msg: "Expected variable name while parsing let binding.".to_string(),
                offset: state.offset,
            })
        }
    } else {
        Err(SyntaxError {
            msg: "Unexpected end of input while parsing let binding.".to_string(),
            offset: state.offset,
        })
    }
}

fn expression(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, SyntaxError> {
    if let Some(token) = tokens.peek() {
        match &token.kind {
            TokenKind::LeftBrace => table(state, tokens),
            TokenKind::Rel => relation(state, tokens),
            TokenKind::Literal(name) => {
                let name = name.to_string();
                let offset = token.offset;
                state.offset = token.offset;
                tokens.next();
                if let Some(token) = tokens.peek() {
                    if token.kind == TokenKind::LeftParen {
                        let arglist = arglist(state, tokens)?;
                        Ok(AST::FnCall(name, arglist, offset))
                    } else {
                        Ok(AST::BindingRef(name))
                    }
                } else {
                    Ok(AST::BindingRef(name))
                }
            }
            _ => goal(state, tokens),
        }
    } else {
        Err(SyntaxError {
            msg: "Unexpected end of input while parsing expression.".to_string(),
            offset: state.offset,
        })
    }
}

fn table(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, SyntaxError> {
    if let Some(token) = tokens.next() {
        if token.kind != TokenKind::LeftBrace {
            return Err(SyntaxError {
                msg: "Unexpected end of input while parsing.".to_string(),
                offset: state.offset,
            });
        }
        state.offset = token.offset;
        let mut members: Vec<AST> = Vec::new();
        loop {
            if let Some(token) = tokens.peek() {
                if token.kind == TokenKind::RightBrace {
                    state.offset = token.offset;
                    tokens.next();
                    break;
                }
            }
            members.push(term(state, tokens)?);
            if let Some(token) = tokens.next() {
                if token.kind != TokenKind::Colon {
                    return Err(SyntaxError {
                        msg: "Expected `:` while parsing table.".to_string(),
                        offset: state.offset,
                    });
                }
                state.offset = token.offset;
            }
            members.push(term(state, tokens)?);
            if let Some(token) = tokens.next() {
                if token.kind == TokenKind::Comma {
                    state.offset = token.offset;
                } else if token.kind == TokenKind::RightBrace {
                    state.offset = token.offset;
                    break;
                } else {
                    return Err(SyntaxError {
                        msg: "Expected `,` or `}` while parsing table.".to_string(),
                        offset: state.offset,
                    });
                }
            } else {
                return Err(SyntaxError {
                    msg: "Unexpected end of input while parsing table.".to_string(),
                    offset: state.offset,
                });
            }
        }
        Ok(AST::Table(members))
    } else {
        Err(SyntaxError {
            msg: "Unexpected end of input while parsing table.".to_string(),
            offset: state.offset,
        })
    }
}

fn relation(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, SyntaxError> {
    if let Some(token) = tokens.next() {
        if token.kind != TokenKind::Rel {
            return Err(SyntaxError {
                msg: "Expected `rel`.".to_string(),
                offset: state.offset,
            });
        }
        state.offset = token.offset;
        let parameters = varlist(state, tokens)?;
        if let Some(token) = tokens.next() {
            if token.kind != TokenKind::LeftBrace {
                return Err(SyntaxError {
                    msg: "Expected `{`.".to_string(),
                    offset: state.offset,
                });
            }
        } else {
            return Err(SyntaxError {
                msg: "Unexpected end of input while parsing `rel`.".to_string(),
                offset: state.offset,
            });
        }
        let body = goal(state, tokens)?;
        if let Some(token) = tokens.next() {
            if token.kind != TokenKind::RightBrace {
                return Err(SyntaxError {
                    msg: "Expected `}`.".to_string(),
                    offset: state.offset,
                });
            }
        } else {
            return Err(SyntaxError {
                msg: "Unexpected end of input while parsing `rel`.".to_string(),
                offset: state.offset,
            });
        }
        Ok(AST::Relation(parameters, Box::new(body)))
    } else {
        Err(SyntaxError {
            msg: "Unexpected end of input while parsing `rel`.".to_string(),
            offset: state.offset,
        })
    }
}

fn goal(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, SyntaxError> {
    if let Some(token) = tokens.peek() {
        match token.kind {
            TokenKind::Conj => {
                state.offset = token.offset;
                tokens.next();
                if let Some(token) = tokens.next() {
                    if token.kind != TokenKind::LeftBrace {
                        Err(SyntaxError {
                            msg: "Expected { after conj.".to_string(),
                            offset: state.offset,
                        })
                    } else {
                        state.offset = token.offset;
                        conj(state, tokens)
                    }
                } else {
                    Err(SyntaxError {
                        msg: "Unexpected end of input while parsing conj.".to_string(),
                        offset: state.offset,
                    })
                }
            }
            TokenKind::Disj => {
                state.offset = token.offset;
                tokens.next();
                if let Some(token) = tokens.next() {
                    if token.kind != TokenKind::LeftBrace {
                        Err(SyntaxError {
                            msg: "Expected { after disj.".to_string(),
                            offset: state.offset,
                        })
                    } else {
                        state.offset = token.offset;
                        disj(state, tokens)
                    }
                } else {
                    Err(SyntaxError {
                        msg: "Unexpected end of input while parsing disj.".to_string(),
                        offset: state.offset,
                    })
                }
            }
            TokenKind::Tick | TokenKind::Literal(_) => equals(state, tokens),
            TokenKind::Var => {
                state.offset = token.offset;
                tokens.next();
                var(state, tokens)
            }
            _ => Err(SyntaxError {
                msg: "Expected conj, disj, equals or var while parsing goal.".to_string(),
                offset: state.offset,
            }),
        }
    } else {
        Err(SyntaxError {
            msg: "Unexpected end of input while parsing goal.".to_string(),
            offset: state.offset,
        })
    }
}

fn conj(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, SyntaxError> {
    let mut goals: Vec<AST> = Vec::new();
    while tokens.peek().is_some() {
        goals.push(goal(state, tokens)?);
        if let Some(token) = tokens.peek() {
            match token.kind {
                TokenKind::Comma => {
                    state.offset = token.offset;
                    tokens.next();
                }
                TokenKind::RightBrace => {
                    state.offset = token.offset;
                    tokens.next();
                    break;
                }
                _ => {
                    return Err(SyntaxError {
                        msg: "Expected `,` or `}` while parsing conj.".to_string(),
                        offset: state.offset,
                    });
                }
            }
        } else {
            return Err(SyntaxError {
                msg: "Unexpected end of input while parsing conj.".to_string(),
                offset: state.offset,
            });
        }
    }
    match goals.len() {
        0 => Err(SyntaxError {
            msg: "Empty conj expression.".to_string(),
            offset: state.offset,
        }),
        1 => Ok(goals.remove(0)),
        _ => Ok(AST::Conj(goals)),
    }
}

fn disj(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, SyntaxError> {
    let mut goals: Vec<AST> = Vec::new();
    while tokens.peek().is_some() {
        goals.push(goal(state, tokens)?);
        if let Some(token) = tokens.peek() {
            match token.kind {
                TokenKind::Pipe => {
                    state.offset = token.offset;
                    tokens.next();
                }
                TokenKind::RightBrace => {
                    state.offset = token.offset;
                    tokens.next();
                    break;
                }
                _ => {
                    return Err(SyntaxError {
                        msg: "Expected `|` or `}` while parsing disj.".to_string(),
                        offset: state.offset,
                    });
                }
            }
        } else {
            return Err(SyntaxError {
                msg: "Unexpected end of input while parsing disj.".to_string(),
                offset: state.offset,
            });
        }
    }

    match goals.len() {
        0 => Err(SyntaxError {
            msg: "Empty disj expression.".to_string(),
            offset: state.offset,
        }),
        1 => Ok(goals.remove(0)),
        _ => Ok(AST::Disj(goals)),
    }
}

fn equals(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, SyntaxError> {
    let left = term(state, tokens)?;
    if let Some(token) = tokens.next() {
        if token.kind == TokenKind::DoubleEquals {
            state.offset = token.offset;
            let right = term(state, tokens)?;
            Ok(AST::Equals(Box::new(left), Box::new(right)))
        } else {
            Err(SyntaxError {
                msg: "Expected `==` while parsing equals.".to_string(),
                offset: state.offset,
            })
        }
    } else {
        Err(SyntaxError {
            msg: "Unexpected end of input while parsing equals.".to_string(),
            offset: state.offset,
        })
    }
}

fn term(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, SyntaxError> {
    if let Some(token) = tokens.peek() {
        if token.kind == TokenKind::Tick {
            atom(state, tokens)
        } else {
            variable(state, tokens)
        }
    } else {
        Err(SyntaxError {
            msg: "Unexpected end of input while parsing term.".to_string(),
            offset: state.offset,
        })
    }
}

fn atom(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, SyntaxError> {
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
                            Err(SyntaxError {
                                msg: "Expected identifier while parsing atom.".to_string(),
                                offset: state.offset,
                            })
                        }
                    }
                    None => Err(SyntaxError {
                        msg: "Unexpected end of input while parsing atom.".to_string(),
                        offset: state.offset,
                    }),
                }
            } else {
                Err(SyntaxError {
                    msg: "Expected `'` while parsing atom.".to_string(),
                    offset: state.offset,
                })
            }
        }
        None => Err(SyntaxError {
            msg: "Unexpected end of input while parsing atom.".to_string(),
            offset: state.offset,
        }),
    }
}

fn var(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, SyntaxError> {
    let declarations = varlist(state, tokens)?;
    if let Some(token) = tokens.next() {
        if token.kind == TokenKind::LeftBrace {
            state.offset = token.offset;
            let body = goal(state, tokens)?;
            if let Some(token) = tokens.next() {
                if token.kind == TokenKind::RightBrace {
                    state.offset = token.offset;
                    Ok(AST::Var(declarations, Box::new(body)))
                } else {
                    Err(SyntaxError {
                        msg: "Expected `}}` while parsing var.".to_string(),
                        offset: state.offset,
                    })
                }
            } else {
                Err(SyntaxError {
                    msg: "Unexpected end of input while parsing var.".to_string(),
                    offset: state.offset,
                })
            }
        } else {
            Err(SyntaxError {
                msg: "Expected `{{` while parsing var.".to_string(),
                offset: state.offset,
            })
        }
    } else {
        Err(SyntaxError {
            msg: "Unexpected end of input while parsing var.".to_string(),
            offset: state.offset,
        })
    }
}

fn arglist(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<Vec<AST>, SyntaxError> {
    let mut arguments: Vec<AST> = Vec::new();
    if let Some(token) = tokens.next() {
        if token.kind != TokenKind::LeftParen {
            return Err(SyntaxError {
                msg: "Expected `,` or `)` while parsing argument list.".to_string(),
                offset: state.offset,
            });
        }
    } else {
        return Err(SyntaxError {
            msg: "Unexpected end of input while parsing argument list.".to_string(),
            offset: state.offset,
        });
    }

    // Allow for no arguments
    if let Some(token) = tokens.peek() {
        if token.kind == TokenKind::RightParen {
            state.offset = token.offset;
            tokens.next();
            return Ok(arguments);
        }
    } else {
        return Err(SyntaxError {
            msg: "Unexpected end of input while parsing argument list.".to_string(),
            offset: state.offset,
        });
    }

    while tokens.peek().is_some() {
        arguments.push(expression(state, tokens)?);
        if let Some(token) = tokens.peek() {
            match token.kind {
                TokenKind::Comma => {
                    state.offset = token.offset;
                    tokens.next();
                }
                TokenKind::RightParen => {
                    state.offset = token.offset;
                    tokens.next();
                    break;
                }
                _ => {
                    return Err(SyntaxError {
                        msg: "Expected `,` or `)` while parsing argument list.".to_string(),
                        offset: state.offset,
                    });
                }
            }
        } else {
            return Err(SyntaxError {
                msg: "Unexpected end of input while parsing argument list.".to_string(),
                offset: state.offset,
            });
        }
    }

    Ok(arguments)
}

fn varlist(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<Vec<AST>, SyntaxError> {
    let mut declarations: Vec<AST> = Vec::new();
    if let Some(token) = tokens.next() {
        if token.kind != TokenKind::LeftParen {
            return Err(SyntaxError {
                msg: "Expected `,` or `)` while parsing variable list.".to_string(),
                offset: state.offset,
            });
        }
    } else {
        return Err(SyntaxError {
            msg: "Unexpected end of input while parsing variable list.".to_string(),
            offset: state.offset,
        });
    }

    while tokens.peek().is_some() {
        declarations.push(variable(state, tokens)?);
        if let Some(token) = tokens.peek() {
            match token.kind {
                TokenKind::Comma => {
                    state.offset = token.offset;
                    tokens.next();
                }
                TokenKind::RightParen => {
                    state.offset = token.offset;
                    tokens.next();
                    break;
                }
                _ => {
                    return Err(SyntaxError {
                        msg: "Expected `,` or `)` while parsing variable list.".to_string(),
                        offset: state.offset,
                    });
                }
            }
        } else {
            return Err(SyntaxError {
                msg: "Unexpected end of input while parsing variable list.".to_string(),
                offset: state.offset,
            });
        }
    }

    match declarations.len() {
        0 => Err(SyntaxError {
            msg: "Empty variable list.".to_string(),
            offset: state.offset,
        }),
        _ => Ok(declarations),
    }
}

fn variable(
    state: &mut ParseState,
    tokens: &mut Peekable<std::vec::IntoIter<Token>>,
) -> Result<AST, SyntaxError> {
    if let Some(token) = tokens.next() {
        if let TokenKind::Literal(name) = token.kind {
            state.offset = token.offset;
            Ok(AST::Variable(name))
        } else {
            Err(SyntaxError {
                msg: "Expected literal while parsing variable.".to_string(),
                offset: state.offset,
            })
        }
    } else {
        Err(SyntaxError {
            msg: "Unexpected end of input while parsing var.".to_string(),
            offset: state.offset,
        })
    }
}

pub fn parse(tokens: Vec<Token>) -> Result<AST, SyntaxError> {
    let mut state = ParseState { offset: 0 };
    let mut iter = tokens.into_iter().peekable();
    let ast = program(&mut state, &mut iter);
    if iter.next().is_none() || ast.is_err() {
        ast
    } else {
        Err(SyntaxError {
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
        parse!("olive", "olive");
        parsefails!(
            "'olive ==",
            "Unexpected end of input while parsing term.",
            7
        );
        parsefails!("'olive", "Unexpected end of input while parsing equals.", 5);
        parsefails!("'olive 'oil", "Expected `==` while parsing equals.", 5);
        parse!(
            "conj { 'red == 'red , 'bean == 'bean }",
            "conj { 'red == 'red , 'bean == 'bean }"
        );
        parse!(
            "disj { 'red == 'red | 'bean == 'bean }",
            "disj { 'red == 'red | 'bean == 'bean }"
        );
        parse!(
            "disj { 'red == 'red | conj { 'red == 'red , 'bean == 'bean } }",
            "disj { 'red == 'red | conj { 'red == 'red , 'bean == 'bean } }"
        );
        parse!(
            "conj { 'red == 'red , disj { 'red == 'red | 'bean == 'bean } }",
            "conj { 'red == 'red , disj { 'red == 'red | 'bean == 'bean } }"
        );
        parse!("conj { 'red == 'red  }", "'red == 'red");
        parse!("disj { 'red == 'red  }", "'red == 'red");
        parsefails!(
            "conj {}",
            "Expected conj, disj, equals or var while parsing goal.",
            5
        );
        parsefails!(
            "disj {}",
            "Expected conj, disj, equals or var while parsing goal.",
            5
        );
        parse!("var (q) { 'olive == q }", "var (q) { 'olive == q }");
        parse!("var (q) { q == 'olive }", "var (q) { q == 'olive }");
        parse!("var (p, q) { p == q }", "var (p, q) { p == q }");
        parse!("fn()", "fn()");
        parse!("fn('olive == 'olive)", "fn('olive == 'olive)");
        parse!("fn(var (p, q) { p == q })", "fn(var (p, q) { p == q })");
        parsefails!(
            "fn(var (p, q) { p == q }",
            "Unexpected end of input while parsing argument list.",
            23
        );
        parse!("{}", "{}");
        parse!("{x: 'olive}", "{x: 'olive}");
        parse!("{x: 'olive, 'olive: 'oil}", "{x: 'olive, 'olive: 'oil}");
        parsefails!("{x 'olive}", "Expected `:` while parsing table.", 1);
        parsefails!(
            "{x: 'olive 'olive: 'oil}",
            "Expected `,` or `}` while parsing table.",
            9
        );
        parsefails!(
            "{x: 'olive, 'olive: 'oil",
            "Unexpected end of input while parsing table.",
            23
        );
        parse!("let x = {}", "let x = {}");
        parse!("let x = 'olive == 'olive", "let x = 'olive == 'olive");
        parse!(
            "let x = disj { 'red == 'red | 'bean == 'bean }",
            "let x = disj { 'red == 'red | 'bean == 'bean }"
        );
        parsefails!("let", "Unexpected end of input while parsing var.", 2);
        parsefails!(
            "let x",
            "Unexpected end of input while parsing let binding.",
            4
        );
        parsefails!(
            "let x =",
            "Unexpected end of input while parsing expression.",
            6
        );
        parsefails!("let 'x = {}", "Expected literal while parsing variable.", 2);
        parsefails!("let {} = {}", "Expected literal while parsing variable.", 2);
        parse!("x", "x");
        parse!("rel(x) { x == 'olive }", "rel(x) { x == 'olive }");
        parse!(
            "let y = rel(x) { disj { x == 'red | x == 'bean } }",
            "let y = rel(x) { disj { x == 'red | x == 'bean } }"
        );
        parsefails!(
            "rel(x) { x == 'olive ",
            "Unexpected end of input while parsing `rel`.",
            19
        );
        parsefails!(
            "rel(x) { }",
            "Expected conj, disj, equals or var while parsing goal.",
            5
        );
        parsefails!(
            "rel() { 'olive == 'oil }",
            "Expected literal while parsing variable.",
            2
        );
    }
}
