#![no_std]

//! This is an event-driven parser for ini files.
//!
//! The file consists of ASCII characters, everything after '#' is a comment, and it
//! is ignored until the '\n' (new line) character is encountered.
//!
//! The file can have sections denoted with the square brackets:
//! ```ignore
//! [BRICKS]
//!```
//!
//! Other well-formed input consists of key-value pairs delimited with '=':
//! ```ignore
//! count = infinity
//!```
//!
//! The users code calls `[Parser::parse()]` until it either returns an error or
//! indicates that the parser is at the end of the input.
//!
//! The semantics checks are done by the user code.

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct Location {
    pub line: usize,
    pub col: usize,
    pub pos: usize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Error {
    ExpectedSectionOrKey(Location),
    ExpectedKey(Location),
    ExpectedValue(Location),
    ExpectedAssign(Location),
    UnexpectedToken(Location),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Token<'a> {
    Unknown(Error),
    Section(&'a [u8]),
    Key(&'a [u8]),
    Value(&'a [u8]),
    Assign,
    StartOfInput,
    EndOfInput,
}

pub enum Clause<'a> {
    KeyValue(&'a [u8], &'a [u8]),
    Section(&'a [u8]),
}

pub struct Parser<'a> {
    location: Location,
    input: &'a [u8],
    token: Token<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            location: Location::default(),
            token: Token::StartOfInput,
            input,
        }
    }

    fn parse_token(&mut self) {
        if matches!(self.token, Token::Unknown(_) | Token::EndOfInput) {
            return;
        }

        if self.location.pos >= self.input.len() {
            self.token = Token::EndOfInput;
            return;
        }
    }

    pub fn parse(&mut self) -> Result<Option<Clause<'a>>, Error> {
        loop {
            match self.token {
                Token::StartOfInput => {
                    self.parse_token();
                    match self.token {
                        Token::Section(token_slice) => {
                            return Ok(Some(Clause::Section(token_slice)))
                        }
                        Token::Key(_) => {}
                        _ => return Err(Error::ExpectedSectionOrKey(self.location)),
                    }
                }
                Token::EndOfInput => return Ok(None),
                Token::Section(_) => {
                    self.parse_token();
                    if !matches!(self.token, Token::Key(_)) {
                        return Err(Error::ExpectedKey(self.location));
                    }
                }
                Token::Key(key_slice) => {
                    self.parse_token();
                    if !matches!(self.token, Token::Assign) {
                        return Err(Error::ExpectedAssign(self.location));
                    }
                    self.parse_token();
                    match self.token {
                        Token::Value(value_slice) => {
                            return Ok(Some(Clause::KeyValue(key_slice, value_slice)))
                        }
                        _ => return Err(Error::UnexpectedToken(self.location)),
                    }
                }
                _ => return Err(Error::UnexpectedToken(self.location)),
            }
        }
    }
}

// mut token = Token::Unknown,
// for &b in bytes {
//     match b {
//         1..=b' ' => continue,
//         b'=' =>
//         _ => break
//     }
// }
mod tests {}
