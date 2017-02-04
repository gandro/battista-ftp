#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    MissingInput,
    UnexpectedToken(u8),
}

pub trait Parse {
    fn parse(&self, s: &[u8]) -> Result<usize, ParseError>;
}

impl<P1: Parse, P2: Parse> Parse for (P1, P2) {
    #[inline(always)]
    fn parse(&self, s: &[u8]) -> Result<usize, ParseError> {
        self.0.parse(s).and_then(|prefix| {
            self.1.parse(&s[prefix..]).map(|suffix| {
                prefix + suffix
            })
        })
    }
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

pub struct AsciiChar;

impl Parse for AsciiChar {
    fn parse(&self, s: &[u8]) -> Result<usize, ParseError> {
        Byte(|b| b < 128 && b != b'\r' && b != b'\n').parse(s)
    }
}

pub struct AsciiString;

impl Parse for AsciiString {
    fn parse(&self, s: &[u8]) -> Result<usize, ParseError> {
        AsciiChar.parse(s).and_then(|prefix| {
            AsciiString.parse(&s[prefix..])
                .map(|suffix| prefix + suffix)
                .or(Ok(prefix))
        })
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
        PrintChar.parse(s).and_then(|prefix| {
            PrintString.parse(&s[prefix..])
                .map(|suffix| prefix + suffix)
                .or(Ok(prefix))
        })
    }
}

#[cfg(test)]
mod tests {
    use grammar::*;

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
        assert_eq!(Err(ParseError::UnexpectedToken(0)), PrintString.parse("\0llo\n".as_bytes()));
    }
    
    #[test]
    fn parse_command() {
        
    }
}
