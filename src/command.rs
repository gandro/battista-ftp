use std::io;
use std::slice;
use std::mem;
use std::ascii::AsciiExt;
use tokio_core::io::EasyBuf;

macro_rules! try_decode {
    ($e:expr) => (match $e {
        Ok(Some(t)) => t,
        Ok(None) => return Ok(None),
        Err(err) => return Err(err),
    })
}

macro_rules! try_next {
    ($e:expr) => (match $e.next() {
        Some(t) => t,
        None => return Ok(None),
    })
}

struct Bytes<'buf> {
    buf: &'buf [u8],
    iter: slice::Iter<'buf, u8>,
}

impl<'buf> Iterator for Bytes<'buf> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        self.iter.next().map(|&b| b)
    }
}

impl<'buf> Bytes<'buf> {
    fn consume(&mut self) -> &'buf [u8] {
        let start = self.buf.as_ptr() as usize;
        let end = self.iter.as_slice().as_ptr() as usize;

        let slice = mem::replace(&mut self.buf, self.iter.as_slice());
        &slice[..end - start]
    }

    fn as_slice(&self) -> &'buf [u8] {
        self.buf
    }
}

impl<'buf> From<&'buf [u8]> for Bytes<'buf> {
    fn from(buf: &'buf [u8]) -> Self {
        Bytes {
            buf: buf,
            iter: buf.iter(),
        }
    }
}

pub struct Command {
}


fn invalid_token(_token: char) -> io::Error {
    // TODO: create proper decode error
    io::Error::from(io::ErrorKind::InvalidData)
}

fn expect_command<'buf>(bytes: &mut Bytes<'buf>) -> io::Result<Option<&'buf [u8]>> {
    loop {
        match try_next!(bytes) as char {
            // ASCII alphabetic character
            'a'...'z' | 'A'...'Z' => continue,
            // <SP>
            ' ' => return Ok(Some(bytes.consume())),
            // <CRLF>
            '\r' => {
                match try_next!(bytes) as char {
                    '\n' => return Ok(Some(bytes.consume())),
                    token => return Err(invalid_token(token)),
                }
            }
            token => return Err(invalid_token(token)),
        }
    }
}

impl Command {
    pub fn decode(buf: &mut EasyBuf) -> io::Result<Option<Self>> {
        let mut bytes = Bytes::from(buf.as_slice());
        let command = try_decode!(expect_command(&mut bytes));
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_matches {
        ($pat:pat,$e:expr) => {
            match $e {
                $pat => (),
                ref e => panic!("assertion failed: `{:?}` does not match `{}`",
                    e, stringify!($pat))
            }
        };
    }

    #[test]
    fn parse_command() {
        fn parse<'buf>(slice: &'buf str) -> io::Result<Option<&'buf [u8]>> {
            let mut bytes = Bytes::from(slice.as_bytes());
            expect_command(&mut bytes)
        }

        assert_eq!(parse("foo  ").unwrap().unwrap(), "foo ".as_bytes());
    }
}
