use std::fmt;
use std::str;
use std::ops::Deref;
use std::ascii::AsciiExt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    MissingInput,
    UnexpectedToken(u8),
    CmdTooLong,
}

pub trait Parse {
    fn parse(&self, s: &[u8]) -> Result<usize, ParseError>;
}

pub struct Byte<F>(pub F);

impl<F: Fn(u8) -> bool> Parse for Byte<F> {
    fn parse(&self, s: &[u8]) -> Result<usize, ParseError> {
        match s.first() {
            Some(b) if self.0(*b) => Ok(1),
            Some(b) => Err(ParseError::UnexpectedToken(*b)),
            None => Err(ParseError::MissingInput),
        }
    }
}

pub struct OneOrMany<P>(pub P);

impl<P: Parse> Parse for OneOrMany<P> {
    fn parse(&self, s: &[u8]) -> Result<usize, ParseError> {   
        let mut len = self.0.parse(s)?;
        while let Ok(suffix) = self.0.parse(&s[len..]) {
            len += suffix;
        }
    
        Ok(len)
    }
}

pub struct AsciiChar;

impl Parse for AsciiChar {
    fn parse(&self, s: &[u8]) -> Result<usize, ParseError> {
        Byte(|b| b < 128 && b != b'\r' && b != b'\n').parse(s)
    }
}

pub struct AsciiString;

impl Parse for AsciiString {
    fn parse(&self, s: &[u8]) -> Result<usize, ParseError> {
        OneOrMany(AsciiChar).parse(s)
    }
}

pub struct PrintChar;

impl Parse for PrintChar {
    fn parse(&self, s: &[u8]) -> Result<usize, ParseError> {
        Byte(|b| b >= 33 && b <= 126).parse(s)
    }
}

pub struct PrintString;

impl Parse for PrintString {
    fn parse(&self, s: &[u8]) -> Result<usize, ParseError> {
        OneOrMany(PrintChar).parse(s)
    }
}

pub struct CmdChar;

impl Parse for CmdChar {
    fn parse(&self, s: &[u8]) -> Result<usize, ParseError> {
        Byte(|b| b >= b'A' && b <= b'Z' || b >= b'a' && b <= b'z').parse(s)
    }
}

pub struct CmdString;

impl Parse for CmdString {
    fn parse(&self, s: &[u8]) -> Result<usize, ParseError> {
        OneOrMany(CmdChar).parse(s)
    }
}

pub struct Sp;

impl Parse for Sp {
    fn parse(&self, s: &[u8]) -> Result<usize, ParseError> {
        Byte(|b| b == b' ').parse(s)
    }
}

pub struct CrLf;

impl Parse for CrLf {
    fn parse(&self, s: &[u8]) -> Result<usize, ParseError> {
        let cr = Byte(|b| b == b'\r').parse(&s[0..])?;
        let lf = Byte(|b| b == b'\n').parse(&s[1..])?;
        Ok(2)
    }
}

#[cfg(test)]
mod tests {
    use parser::*;

    #[test]
    fn parse_byte() {
        assert_eq!(Ok(1), Byte(|b| b == 32).parse(&[32]));
        assert_eq!(Err(ParseError::UnexpectedToken(31)), Byte(|b| b == 32).parse(&[31]));
        assert_eq!(Err(ParseError::MissingInput), Byte(|b| b == 32).parse(&[]));
    }

    #[test]
    fn parse_string() {
        assert_eq!(Ok(1), AsciiString.parse("H".as_bytes()));
        assert_eq!(Ok(5), AsciiString.parse("Hello".as_bytes()));
        assert_eq!(Ok(6), AsciiString.parse("\0Hello\n".as_bytes()));
        assert_eq!(Err(ParseError::MissingInput), AsciiString.parse("".as_bytes()));
    
        assert_eq!(Ok(1), PrintString.parse("H".as_bytes()));
        assert_eq!(Ok(5), PrintString.parse("Hello".as_bytes()));
        assert_eq!(Err(ParseError::UnexpectedToken(b'\0')), PrintString.parse("\0llo\n".as_bytes()));
    }

    #[test]
    fn parse_command() {
        assert_eq!(Ok(4), CmdString.parse("RETR".as_bytes()));
        assert_eq!(Ok(4), CmdString.parse("Retr".as_bytes()));
        assert_eq!(Ok(4), CmdString.parse("retr".as_bytes()));
        assert_eq!(Ok(4), CmdString.parse("ReTr".as_bytes()));
        assert_eq!(Ok(4), CmdString.parse("rETr".as_bytes()));
        assert_eq!(Ok(4), CmdString.parse("USER foo".as_bytes()));
        assert_eq!(Ok(4), CmdString.parse("NOOP\r\n".as_bytes()));
        assert_eq!(Err(ParseError::UnexpectedToken(b' ')), CmdString.parse(" RETR".as_bytes()));
        assert_eq!(Err(ParseError::UnexpectedToken(b'1')), CmdString.parse("1RETR".as_bytes()));
        assert_eq!(Err(ParseError::MissingInput), CmdString.parse("".as_bytes()));
    }
}
