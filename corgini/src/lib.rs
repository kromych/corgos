//! This is the tiniest parser for the simplest form of the ini files possible.
//!
//! The file is expected to consist of ASCII characters. Everything after '#'
//!  is a comment, and it is ignored until the '\n' (new line) character is
//! encountered.
//!
//! Well-formed input consists of lines each of which carries a key-value pair
//! delimited with '=':
//! ```ignore
//! brick_c.o.u.n.t-0 = "infinite infinity"
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

pub trait InputSlice {}
impl InputSlice for &[u8] {}
impl InputSlice for &str {}

pub trait Input<S>
where
    S: InputSlice + Copy,
{
    fn count(self) -> usize;
    fn slice(self, start: usize, end: usize) -> S;
    fn whitespace(self, index: usize) -> bool;
    fn alpha(self, index: usize) -> bool;
    fn digit(self, index: usize) -> bool;
    fn underscore(self, index: usize) -> bool;
    fn dot(self, index: usize) -> bool;
    fn hyphen(self, index: usize) -> bool;
    fn assign(self, index: usize) -> bool;
    fn hash(self, index: usize) -> bool;
    fn null(self, index: usize) -> bool;
    fn newline(self, index: usize) -> bool;
    fn quote(self, index: usize) -> bool;
}

impl<'a> Input<&'a [u8]> for &'a [u8] {
    fn count(self) -> usize {
        self.len()
    }

    fn slice(self, start: usize, end: usize) -> &'a [u8] {
        &self[start..end]
    }

    fn whitespace(self, index: usize) -> bool {
        (b'\t'..=b' ').contains(&self[index])
    }

    fn alpha(self, index: usize) -> bool {
        self[index].is_ascii_alphabetic()
    }

    fn digit(self, index: usize) -> bool {
        self[index].is_ascii_digit()
    }

    fn underscore(self, index: usize) -> bool {
        self[index] == b'_'
    }

    fn dot(self, index: usize) -> bool {
        self[index] == b'.'
    }

    fn hyphen(self, index: usize) -> bool {
        self[index] == b'-'
    }

    fn assign(self, index: usize) -> bool {
        self[index] == b'='
    }

    fn hash(self, index: usize) -> bool {
        self[index] == b'#'
    }

    fn null(self, index: usize) -> bool {
        self[index] == 0
    }

    fn newline(self, index: usize) -> bool {
        self[index] == b'\n'
    }

    fn quote(self, index: usize) -> bool {
        self[index] == b'"'
    }
}

// TODO: slow and broken for non-ASCII
impl<'a> Input<&'a str> for &'a str {
    fn count(self) -> usize {
        self.chars().count()
    }

    fn slice(self, start: usize, end: usize) -> &'a str {
        &self[start..end]
    }

    fn whitespace(self, index: usize) -> bool {
        ('\t'..=' ').contains(&self.chars().nth(index).unwrap())
    }

    fn alpha(self, index: usize) -> bool {
        self.chars().nth(index).unwrap().is_alphabetic()
    }

    fn digit(self, index: usize) -> bool {
        self.chars().nth(index).unwrap().is_numeric()
    }

    fn underscore(self, index: usize) -> bool {
        self.chars().nth(index).unwrap() == '_'
    }

    fn dot(self, index: usize) -> bool {
        self.chars().nth(index).unwrap() == '.'
    }

    fn hyphen(self, index: usize) -> bool {
        self.chars().nth(index).unwrap() == '-'
    }

    fn assign(self, index: usize) -> bool {
        self.chars().nth(index).unwrap() == '='
    }

    fn hash(self, index: usize) -> bool {
        self.chars().nth(index).unwrap() == '#'
    }

    fn null(self, index: usize) -> bool {
        self.chars().nth(index).unwrap() == '\x00'
    }

    fn newline(self, index: usize) -> bool {
        self.chars().nth(index).unwrap() == '\n'
    }

    fn quote(self, index: usize) -> bool {
        self.chars().nth(index).unwrap() == '"'
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
    UnmatchedQuote(Location),
    InvalidKeyName(Location),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Token {
    Unknown(Error),
    Assign(Location),
    Literal(Location, Location),
    Quoted(Location, Location),
    EndOfInput(Location),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct KeyValue<S>
where
    S: InputSlice,
{
    pub key: S,
    pub value: S,
}

pub struct Parser<I>
where
    I: Copy + InputSlice + Input<I>,
{
    location: Location,
    input: I,
    input_len: usize,
}

impl<I> Parser<I>
where
    I: Copy + InputSlice + Input<I>,
{
    pub fn new(input: I) -> Self {
        Self {
            location: Location::default(),
            input,
            input_len: input.count(),
        }
    }

    #[inline]
    fn parse_token(&mut self) -> Token {
        let mut tok = Token::EndOfInput(self.location);
        if self.location.pos >= self.input_len {
            return tok;
        }

        let mut loc = self.location;
        let key_value_valid_input = |index| {
            self.input.alpha(index)
                || self.input.digit(index)
                || self.input.underscore(index)
                || self.input.dot(index)
                || self.input.hyphen(index)
        };

        'outer: while loc.pos < self.input_len {
            if self.input.null(loc.pos) {
                break;
            } else if self.input.newline(loc.pos) {
                loc.new_line();
            } else if self.input.whitespace(loc.pos) {
                loc.advance();
            } else if self.input.assign(loc.pos) {
                loc.advance();
                tok = Token::Assign(self.location);
                break;
            } else if self.input.hash(loc.pos) {
                loc.advance();
                while loc.pos < self.input_len {
                    if self.input.newline(loc.pos) {
                        continue 'outer;
                    }
                    loc.advance();
                }
            } else if self.input.quote(loc.pos) {
                tok = Token::Unknown(Error::UnmatchedQuote(self.location));
                loc.advance();

                let start_loc = loc;
                while loc.pos < self.input_len {
                    // TODO: escaped quotes
                    if self.input.quote(loc.pos) {
                        tok = Token::Quoted(start_loc, loc);
                        loc.advance();
                        break 'outer;
                    }
                    if self.input.newline(loc.pos) {
                        break;
                    }
                    loc.advance();
                }
            } else if key_value_valid_input(loc.pos) {
                let start_loc = loc;

                loc.advance();
                while loc.pos < self.input_len {
                    if key_value_valid_input(loc.pos) {
                        loc.advance();
                    } else {
                        break;
                    }
                }
                tok = Token::Literal(start_loc, loc);
                break;
            } else {
                tok = Token::Unknown(Error::UnexpectedToken(self.location));
                break;
            }
        }

        self.location = loc;
        tok
    }

    pub fn parse(&mut self) -> Result<Option<KeyValue<I>>, Error> {
        match self.parse_token() {
            Token::EndOfInput(_) => Ok(None),
            Token::Literal(start_key, end_key) => {
                if !self.input.alpha(start_key.pos) {
                    return Err(Error::InvalidKeyName(start_key));
                }

                let token = self.parse_token();
                if !matches!(token, Token::Assign(_)) {
                    return Err(Error::ExpectedAssign(self.location));
                }

                let token = self.parse_token();
                match token {
                    Token::Literal(start_value, end_value) => Ok(Some(KeyValue {
                        key: self.input.slice(start_key.pos, end_key.pos),
                        value: self.input.slice(start_value.pos, end_value.pos),
                    })),
                    Token::Quoted(start_value, end_value) => Ok(Some(KeyValue {
                        key: self.input.slice(start_key.pos, end_key.pos),
                        value: self.input.slice(start_value.pos, end_value.pos),
                    })),
                    _ => Err(Error::UnexpectedToken(self.location)),
                }
            }
            _ => Err(Error::UnexpectedToken(self.location)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::KeyValue;
    use crate::Parser;

    #[test]
    fn parse_key_value_ascii() {
        let input = b"br-ick_c.o.u.n.t0 = \"infinite infinity\"".as_slice();
        let mut parser = Parser::new(input);
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: b"br-ick_c.o.u.n.t0".as_slice(),
                value: b"infinite infinity".as_slice()
            }))
        );

        let eoi = parser.parse();
        assert_eq!(eoi, Ok(None))
    }

    #[test]
    fn parse_key_value() {
        let input = "br-ick_c.o.u.n.t0 = \"infinite infinity\"";
        let mut parser = Parser::new(input);
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: "br-ick_c.o.u.n.t0",
                value: "infinite infinity"
            }))
        );

        let eoi = parser.parse();
        assert_eq!(eoi, Ok(None))
    }

    #[test]
    fn parse_key_values_ascii() {
        let input =
            b"foo0 = bar0\nfoo1 = bar1\nfoo2 = bar2\nfoo3 = \"bar3 bar3\"#.....\n#.........\nfoo4 = bar4\n\n".as_slice();
        let mut parser = Parser::new(input);
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: b"foo0".as_slice(),
                value: b"bar0".as_slice()
            }))
        );
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: b"foo1".as_slice(),
                value: b"bar1".as_slice()
            }))
        );
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: b"foo2".as_slice(),
                value: b"bar2".as_slice()
            }))
        );
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: b"foo3".as_slice(),
                value: b"bar3 bar3".as_slice()
            }))
        );
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: b"foo4".as_slice(),
                value: b"bar4".as_slice()
            }))
        );

        let eoi = parser.parse();
        assert_eq!(eoi, Ok(None))
    }

    #[test]
    fn parse_key_values() {
        let input =
            "foo0 = bar0\nfoo1 = bar1\nfoo2 = bar2\nfoo3 = \"bar3 bar3\"#.....\n#..COMMENT.COMMENT......\nfoo4 = bar4\n\n";
        let mut parser = Parser::new(input);
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: "foo0",
                value: "bar0"
            }))
        );
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: "foo1",
                value: "bar1"
            }))
        );
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: "foo2",
                value: "bar2"
            }))
        );
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: "foo3",
                value: "bar3 bar3"
            }))
        );
        let foo_bar = parser.parse();
        assert_eq!(
            foo_bar,
            Ok(Some(KeyValue {
                key: "foo4",
                value: "bar4"
            }))
        );

        let eoi = parser.parse();
        assert_eq!(eoi, Ok(None))
    }
}
