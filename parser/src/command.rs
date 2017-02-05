use std::io;
use std::mem;
use std::ops::{Range, Deref};
use std::ascii::{self, AsciiExt};
use std::fmt;
use std::str;

use parser::{Parse, CmdString, AsciiString, CrLf, Sp};

pub use parser::ParseError;

#[derive(Clone, PartialEq, Eq)]
pub struct Cmd {
    buf: [u8; 4],
    len: usize,
}

impl Cmd {
    pub fn parse(slice: &[u8]) -> Result<Self, ParseError> {
        let len = CmdString.parse(&slice)?;
        let mut buf = [0; 4];
        if len >= 3 && len <= 4 {
            for (i, byte) in slice[..len].iter().enumerate() {
                buf[i] = byte.to_ascii_uppercase();
            }
            Ok(Cmd { buf: buf, len: len })
        } else {
            Err(ParseError::CmdTooLong)
        }
    }
    
    pub fn len(&self) -> usize {
        self.len
    }
}

impl Deref for Cmd {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.buf[..self.len]
    }
}

impl fmt::Debug for Cmd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Cmd({:?})", str::from_utf8(&*self).unwrap())
    }
}

const MAX_LINE_SIZE: usize = 8 * 1024;

#[derive(Clone)]
pub struct FtpString {
    buf: Vec<u8>,
    start: usize,
    end: usize,
}

impl FtpString {
    pub fn consume(start: usize, buf: &mut Vec<u8>) -> Result<Self, ParseError> {
        let len = AsciiString.parse(&buf[start..])?;
        let end = CrLf.parse(&buf[start+len..])?;

        // TODO copy data to new buf
        let mut buf = mem::replace(buf, Vec::with_capacity(MAX_LINE_SIZE));
        
        Ok(FtpString { buf: buf, start: start, end: start + len })
    }

    fn as_slice(&self) -> &[u8] {
        &self.buf[self.start..self.end]
    }
}

impl fmt::Debug for FtpString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let escaped = self.as_slice()
            .iter()
            .map(|&b| b)
            .flat_map(ascii::escape_default)
            .collect();

        write!(f, "FtpString({:?})", String::from_utf8(escaped).unwrap())
    }
}

#[derive(Debug)]
pub struct Command {
    line: Vec<u8>,
}

impl Command {
    fn new(line: Vec<u8>) -> Self {
        Command {
            line: line,
        }
    }

    pub fn decode(buf: &mut Vec<u8>) -> Result<Self, ParseError> {
        let cmd = Cmd::parse(&buf)?;
        let sp = Sp.parse(&buf[cmd.len()..])?;
        let arg = FtpString::consume(cmd.len() + sp, buf);
        println!{"{:?}", arg};
        Err(ParseError::MissingInput)
    }

    pub fn decode_old(buf: &mut Vec<u8>) -> io::Result<Option<Self>> {
        // It is a bit unfortunate that we have scan through the whole
        // buffer first, but we want to take ownership over the buffer
        // without parsing it.
        if let Some(pos) = buf.windows(2).position(|b| b == b"\r\n") {
            let mut line = mem::replace(buf, Vec::with_capacity(MAX_LINE_SIZE));
            let next = pos + 2;
            // if we have remaining bytes, move them into the new buffer
            if line.len() > next {
                buf.extend_from_slice(&line[next..]);
            }
            
            // remove trailing <CRLF> and normalize command string
            line.truncate(pos);

            Ok(Some(Command::new(line)))
        } else if buf.len() >= MAX_LINE_SIZE {
            // According to RFC959, a server may reply to such inputs with
            // "500 Syntax error". However, we will assume a broken or malicious
            // client here and just drop the connection.
            Err(io::Error::new(io::ErrorKind::Other, "command line too long"))
        } else {
            // unable to locate <CRLF>, try again with more data
            Ok(None)
        }
    }

}

#[cfg(test)]
mod tests {
    use command::*;

    #[test]
    fn cmd() {
        assert_eq!("USER".as_bytes(), &*Cmd::parse("usEr ".as_bytes()).unwrap());
        assert_eq!(Err(ParseError::CmdTooLong), Cmd::parse("foobar".as_bytes()));
    }
}
