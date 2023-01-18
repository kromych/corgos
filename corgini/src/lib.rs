//! This is the tiniest parser for the simplest form of the ini files possible.
//!
//! The file is expected to consist of ASCII characters. Everything after '#'
//!  is a comment, and it is ignored until the '\n' (new line) character is
//! encountered.
//!
//! Well-formed input consists of lines each of which carries a key-value pair
//! delimited with '=':
//! ```ignore
//! brick_count = infinity
//! brick_density = 1000e10
//!```
//!
//! The users code calls `[Parser::parse()]` until it either returns an error or
//! indicates that the parser is at the end of the input returning `Ok(None)`.
//!
//! The semantics checks are done by the calling code.
//!
//! Example:
//! ```ignore
//! let mut parser = corg_ini::Parser::new(bytes);
//! while let Ok(Some(corg_ini::KeyValue { key, value })) = parser.parse() {
//!     match key {
//!         b"log_device" => match value {
//!             b"serial" => config.log_device = LogDevice::Serial,
//!             b"stdout" => config.log_device = LogDevice::StdOut,
//!             _ => continue,
//!         },
//!         b"log_level" => match value {
//!             b"info" => config.log_level = LevelFilter::Info,
//!             b"warn" => config.log_level = LevelFilter::Warn,
//!             b"error" => config.log_level = LevelFilter::Error,
//!             b"debug" => config.log_level = LevelFilter::Debug,
//!             b"trace" => config.log_level = LevelFilter::Trace,
//!             _ => continue,
//!         },
//!         _ => continue,
//!     }
//! }
//! ```

#![cfg_attr(not(test), no_std)]

// Marker trait for ASCII and UTF-8 characters.
pub trait Char {}
impl Char for u8 {}
impl Char for char {}

pub trait CharKind<C: Char> {
    fn is_whitespace(&self) -> bool;
    fn is_alpha(&self) -> bool;
    fn is_digit(&self) -> bool;
    fn is_underscore(&self) -> bool;
    fn is_minus(&self) -> bool;
    fn is_plus(&self) -> bool;
    fn is_assign(&self) -> bool;
    fn is_hash(&self) -> bool;
    fn is_null(&self) -> bool;
    fn is_newline(&self) -> bool;
    fn is_quote(&self) -> bool;
}

impl CharKind<u8> for u8 {
    fn is_whitespace(&self) -> bool {
        (b'\t'..=b' ').contains(self)
    }

    fn is_alpha(&self) -> bool {
        self.is_ascii_alphabetic()
    }

    fn is_digit(&self) -> bool {
        self.is_ascii_digit()
    }

    fn is_underscore(&self) -> bool {
        *self == b'_'
    }

    fn is_plus(&self) -> bool {
        *self == b'-'
    }

    fn is_minus(&self) -> bool {
        *self == b'='
    }

    fn is_assign(&self) -> bool {
        *self == b'='
    }

    fn is_hash(&self) -> bool {
        *self == b'#'
    }

    fn is_null(&self) -> bool {
        *self == 0
    }

    fn is_newline(&self) -> bool {
        *self == b'\n'
    }

    fn is_quote(&self) -> bool {
        *self == b'"'
    }
}

impl CharKind<char> for char {
    fn is_whitespace(&self) -> bool {
        ('\t'..=' ').contains(self)
    }

    fn is_alpha(&self) -> bool {
        self.is_alphabetic()
    }

    fn is_digit(&self) -> bool {
        self.is_numeric()
    }

    fn is_underscore(&self) -> bool {
        *self == '_'
    }

    fn is_minus(&self) -> bool {
        *self == '-'
    }

    fn is_plus(&self) -> bool {
        *self == '+'
    }

    fn is_assign(&self) -> bool {
        *self == '='
    }

    fn is_hash(&self) -> bool {
        *self == '#'
    }

    fn is_null(&self) -> bool {
        *self == '\x00'
    }

    fn is_newline(&self) -> bool {
        *self == '\n'
    }

    fn is_quote(&self) -> bool {
        *self == '"'
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Location {
    line: usize,
    col: usize,
    pos: usize,
}

impl Default for Location {
    fn default() -> Self {
        Self {
            line: 1,
            col: 1,
            pos: 0,
        }
    }
}

impl Location {
    pub fn new_line(&mut self) {
        self.col = 1;
        self.line += 1;
        self.pos += 1;
    }

    pub fn advance(&mut self) {
        self.col += 1;
        self.pos += 1;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Error {
    ExpectedKey(Location),
    ExpectedValue(Location),
    ExpectedAssign(Location),
    UnexpectedToken(Location),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Token<'a, C>
where
    C: Char,
{
    Unknown(Error),
    Assign,
    Literal(&'a [C]),
    EndOfInput,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct KeyValue<'a, C>
where
    C: Char,
{
    pub key: &'a [C],
    pub value: &'a [C],
}

pub struct Parser<'a, C>
where
    C: Char,
{
    location: Location,
    input: &'a [C],
}

impl<'a, C> Parser<'a, C>
where
    C: Char + CharKind<C> + Copy,
{
    pub fn new(input: &'a [C]) -> Self {
        Self {
            location: Location::default(),
            input,
        }
    }

    fn parse_token(&mut self) -> Token<'a, C> {
        if self.location.pos >= self.input.len() {
            return Token::EndOfInput;
        }

        let mut loc = self.location;
        let mut tok = Token::EndOfInput;

        'outer: while loc.pos < self.input.len() {
            let b = self.input[loc.pos];
            if b.is_null() {
                tok = Token::EndOfInput;
                break;
            } else if b.is_newline() {
                loc.new_line();
            } else if b.is_whitespace() {
                loc.advance();
            } else if b.is_assign() {
                loc.advance();
                tok = Token::Assign;
                break;
            } else if b.is_hash() {
                loc.advance();
                while loc.pos < self.input.len() {
                    if self.input[loc.pos].is_newline() {
                        continue 'outer;
                    }
                    loc.advance();
                }
            } else if b.is_alpha() || b.is_digit() {
                let start_loc = loc;

                loc.advance();
                while loc.pos < self.input.len() {
                    let b = self.input[loc.pos];
                    if b.is_alpha() || b.is_digit() {
                        loc.advance();
                    } else {
                        break;
                    }
                }
                tok = Token::Literal(&self.input[start_loc.pos..loc.pos]);
                break;
            } else {
                tok = Token::Unknown(Error::UnexpectedToken(self.location));
                break;
            }
        }

        self.location = loc;
        tok
    }

    pub fn parse(&mut self) -> Result<Option<KeyValue<'a, C>>, Error> {
        match self.parse_token() {
            Token::EndOfInput => Ok(None),
            Token::Literal(key) => {
                let token = self.parse_token();
                if !matches!(token, Token::Assign) {
                    return Err(Error::ExpectedAssign(self.location));
                }
                let token = self.parse_token();
                match token {
                    Token::Literal(value) => Ok(Some(KeyValue { key, value })),
                    _ => Err(Error::UnexpectedToken(self.location)),
                }
            }
            _ => Err(Error::UnexpectedToken(self.location)),
        }
    }
}

#[cfg(test)]
mod tests {
    #![cfg(test)]

    use crate::KeyValue;
    use crate::Parser;

    #[test]
    fn parse_key_value_ascii() {
        let input = b"foo = bar";
        let mut parser = Parser::new(input);
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: b"foo",
                value: b"bar"
            }))
        );

        let eoi = parser.parse();
        assert_eq!(eoi, Ok(None))
    }
    /*
        fn parse_key_value() {
            let input = "foo = bar";
            let mut parser = Parser::new(input);
            let foo_bar = parser.parse();
            assert_eq!(
                foo_bar,
                Ok(Some(KeyValue {
                    key: "foo",
                    value: "bar"
                }))
            );

            let eoi = parser.parse();
            assert_eq!(eoi, Ok(None))
        }
    */

    #[test]
    fn parse_key_values_ascii() {
        let input =
            b"foo0 = bar0\nfoo1 = bar1\nfoo2 = bar2\nfoo3 = bar3#.....\n#.........\nfoo4 = bar4\n\n";
        let mut parser = Parser::new(input);
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: b"foo0",
                value: b"bar0"
            }))
        );
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: b"foo1",
                value: b"bar1"
            }))
        );
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: b"foo2",
                value: b"bar2"
            }))
        );
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: b"foo3",
                value: b"bar3"
            }))
        );
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: b"foo4",
                value: b"bar4"
            }))
        );

        let eoi = parser.parse();
        assert_eq!(eoi, Ok(None))
    }
}
