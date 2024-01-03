use crate::vm::Opcode;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct RuntimeError {
    pub msg: String,
    pub ip: usize,
    pub opcode: Opcode,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ParserError: {}", self.msg)
    }
}

impl Error for RuntimeError {}

#[derive(Debug)]
pub struct SyntaxError {
    pub msg: String,
    pub offset: usize,
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ParserError: {}", self.msg)
    }
}

impl Error for SyntaxError {}

#[derive(Debug)]
pub struct TokenizerError {
    pub msg: String,
    pub offset: usize,
}

impl fmt::Display for TokenizerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TokenizerError: {}", self.msg)
    }
}

impl Error for TokenizerError {}
