use std::mem;
use std::ops::Deref;
use std::ascii::{self, AsciiExt};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::fmt;
use std::str::{self, FromStr, Utf8Error};
use std::num::ParseIntError;

//use parser::{Parse, CmdString, AsciiString, CrLf, Sp};

//pub use parser::ParseError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    MissingInput,
    InvalidCmdLength,
    LineTooLong,
    MissingArgument,
    EmptyArgument,
    UnexpectedData,
    MissingHostNumber,
    MissingPortNumber,
    InvalidUtf8(usize),
    InvalidNumber,
    InvalidTypeCode,
    InvalidFormCode,
}

impl From<Utf8Error> for DecodeError {
    fn from(err: Utf8Error) -> Self {
        DecodeError::InvalidUtf8(err.valid_up_to() + 1)
    }
}

impl From<ParseIntError> for DecodeError {
    fn from(_: ParseIntError) -> Self {
        DecodeError::InvalidNumber
    }
}

const BUFFER_MAX_LINE_LENGTH: usize = 8 * 1024;
const BUFFER_DEFAULT_CAPACITY: usize = BUFFER_MAX_LINE_LENGTH / 4;

#[derive(Debug)]
struct Buffer<'a>(&'a mut Vec<u8>);

impl<'a> Buffer<'a> {
    fn read_line(&mut self) -> Result<Line, DecodeError> {
        let Buffer(ref mut buf) = *self;
        // scan through the whole buffer to find <CRLF> so we can split it
        if let Some(pos) = buf.windows(2).position(|b| b == b"\r\n") {
            // split off parts after <CRLF> into new vector
            let remainder = Vec::with_capacity(BUFFER_DEFAULT_CAPACITY);
            let mut line = mem::replace(*buf, remainder);

            // if we have remaining bytes, move them into the new buffer
            let end = pos + b"\r\n".len();
            if line.len() > end {
                buf.extend_from_slice(&line[end..]);
            }

            // cut the buffer right before <CRLF>
            line.truncate(pos);

            Ok(Line(line))
        } else if buf.len() >= BUFFER_MAX_LINE_LENGTH {
            // According to RFC959, a server may reply to such inputs with
            // "500 Syntax error".
            Err(DecodeError::LineTooLong)
        } else {
            Err(DecodeError::MissingInput)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Line(Vec<u8>);

impl Line {
    fn split(self) -> Result<(Cmd, Option<Arg>), DecodeError> {
        let Line(line) = self;

        // cmd and arg are separated by a single <SP> character
        if let Some(pos) = line.iter().position(|&b| b == b' ') {
            let cmd = Cmd::new(&line[..pos])?;
            let arg = Arg::new(line, pos + 1);
            Ok((cmd, Some(arg)))
        } else {
            let cmd = Cmd::new(&line)?;
            Ok((cmd, None))
        }
    }
}

impl Deref for Line {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Cmd {
    buf: [u8; 4],
    len: usize,
}

impl Cmd {
    fn new(cmd: &[u8]) -> Result<Self, DecodeError> {
        let mut buf = [0; 4];
        if cmd.len() >= 3 && cmd.len() <= 4 {
            for (i, byte) in cmd.iter().enumerate() {
                buf[i] = byte.to_ascii_uppercase();
            }
            Ok(Cmd { buf: buf, len: cmd.len() })
        } else {
            Err(DecodeError::InvalidCmdLength)
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

#[derive(Clone)]
pub struct Arg {
    buf: Vec<u8>,
    start: usize,
}

impl Arg {
    pub fn new(buf: Vec<u8>, start: usize) -> Self {
        Arg { buf: buf, start: start }
    }

    pub fn required(arg: Option<Self>) -> Result<Self, DecodeError> {
        match arg {
            Some(arg) => {
                if !arg.is_empty() {
                    Ok(arg)
                } else {
                    Err(DecodeError::EmptyArgument)
                }
            },
            None => Err(DecodeError::MissingArgument),
        }
    }
    
    pub fn forbidden(arg: Option<Self>) -> Result<(), DecodeError> {
        match arg {
            Some(_) => Err(DecodeError::UnexpectedData),
            None => Ok(()),
        }
    }
}

impl Deref for Arg {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.buf[self.start..]
    }
}

impl fmt::Debug for Arg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let escaped = self
            .iter()
            .map(|&b| b)
            .flat_map(ascii::escape_default)
            .collect();

        write!(f, "Arg({:?})", String::from_utf8(escaped).unwrap())
    }
}

struct Number<'a>(&'a [u8]);

impl<'a> Number<'a> {
    fn parse(self) -> Result<u8, DecodeError> {
        // TODO: avoid conversion to UTF-8
        let s = str::from_utf8(self.0)?;
        Ok(u8::from_str(s)?)
    }
}

struct HostPort(Arg);

impl HostPort {
    fn parse_v4(self) -> Result<SocketAddr, DecodeError> {
        // TODO: deal with too many args

        let HostPort(arg) = self;
        let mut numbers = arg.split(|&b| b == b',');

        // parse IPv4 host address
        let mut b = [0; 4];
        for i in 0..4 {
            let next = numbers.next().ok_or(DecodeError::MissingHostNumber)?;
            b[i] = Number(next).parse()?;
        }

        // parse port
        let mut port = 0u16;
        for multiplier in &[256, 1] {
            let next = numbers.next().ok_or(DecodeError::MissingHostNumber)?;
            port += multiplier * Number(next).parse()? as u16;
        }

        let ipaddr = IpAddr::V4(Ipv4Addr::new(b[0], b[1], b[2], b[2]));
        Ok(SocketAddr::new(ipaddr, port))
    }
}

#[derive(Debug)]
pub enum FormCode {
    NonPrint,
    TelnetFmtControl,
    CarriageControl,
}

impl FormCode {
    fn parse(arg: &[u8]) -> Result<Self, DecodeError> {
        match arg {
            b"N" => Ok(FormCode::NonPrint),
            b"T" => Ok(FormCode::TelnetFmtControl),
            b"C" => Ok(FormCode::CarriageControl),
            _ => Err(DecodeError::InvalidFormCode),
        }
    }
}

#[derive(Debug)]
pub enum TypeCode {
    Ascii(Option<FormCode>),
    Ebcdic(Option<FormCode>),
    Image,
    Local(u8),
}

impl TypeCode {
    fn parse(arg: Arg) -> Result<Self, DecodeError> {
        // TODO: deal with too many args
        let mut args = arg.split(|&b| b == b' ');
        match args.next().ok_or(DecodeError::InvalidTypeCode)? {
            b"A" => {
                let form = if let Some(arg) = args.next() {
                    Some(FormCode::parse(arg)?)
                } else {
                    None
                };
                
                Ok(TypeCode::Ascii(form))
            } 
            b"E" => {
                let form = if let Some(arg) = args.next() {
                    Some(FormCode::parse(arg)?)
                } else {
                    None
                };
                
                Ok(TypeCode::Ebcdic(form))
            }
            b"I" => {
                Ok(TypeCode::Image)
            }
            b"L" => {
                let arg = args.next().ok_or(DecodeError::InvalidTypeCode)?;
                let size = Number(arg).parse()?;
                Ok(TypeCode::Local(size))
            }
            _ => Err(DecodeError::InvalidTypeCode)
        }
    }
}

#[derive(Debug)]
pub enum Command {
    User(Arg),
    Pass(Arg),
    Port(SocketAddr),
    Type(TypeCode),
    Quit,
    Other(Cmd, Option<Arg>),
}

impl Command {
    pub fn decode(buf: &mut Vec<u8>) -> Result<Self, DecodeError> {
        let line = Buffer(buf).read_line()?;
        let (cmd, arg) = line.split()?;

        match &*cmd {
            b"USER" => {
                let username = Arg::required(arg)?;
                Ok(Command::User(username))
            },
            b"PASS" => {
                let password = Arg::required(arg)?;
                Ok(Command::Pass(password))
            },
            b"PORT" => {
                let arg = Arg::required(arg)?;
                let port = HostPort(arg).parse_v4()?;
                Ok(Command::Port(port))
            }
            b"TYPE" => {
                let arg = Arg::required(arg)?;
                let code = TypeCode::parse(arg)?;
                Ok(Command::Type(code))
            }
            b"MODE" => {
                unimplemented!()
            }
            b"STRU" => {
                unimplemented!()
            }
            b"RETR" => {
                unimplemented!()
            }
            b"STOR" => {
                unimplemented!()
            }
            b"NOOP" => {
                unimplemented!()
            }
            b"QUIT" => {
                Arg::forbidden(arg)?;
                Ok(Command::Quit)
            }
            _ => Ok(Command::Other(cmd, arg))
        }
    }

}

#[cfg(test)]
mod tests {
    use command::*;

    #[test]
    fn read_line() {
        let user = "USER foo".as_bytes();
        let pass = "PASS bar".as_bytes();
        let crlf = "\r\n".as_bytes();

        let mut buf = Vec::new();
        assert_eq!(Err(DecodeError::MissingInput), Buffer(&mut buf).read_line());

        let mut buf = user.to_vec();
        assert_eq!(Err(DecodeError::MissingInput), Buffer(&mut buf).read_line());
        buf.extend_from_slice(b"\r\n");
        assert_eq!(Ok(Line(user.to_owned())), Buffer(&mut buf).read_line());
        assert!(buf.is_empty());

        let mut buf = [user, crlf, pass].concat();
        assert_eq!(Ok(Line(user.to_owned())), Buffer(&mut buf).read_line());
        assert_eq!(&*buf, pass);
    }

    #[test]
    fn cmd() {
        assert_eq!("USER".as_bytes(), &*Cmd::new("usEr".as_bytes()).unwrap());
        assert_eq!(Err(DecodeError::InvalidCmdLength), Cmd::new("".as_bytes()));
        assert_eq!(Err(DecodeError::InvalidCmdLength), Cmd::new("foobar".as_bytes()));
    }

    #[test]
    fn split() {
        let mut line = Line(b"USER foo".to_vec());
        let (cmd, arg) = line.split().unwrap();
        assert_eq!("USER".as_bytes(), &*cmd);
        assert_eq!("foo".as_bytes(), &*arg.unwrap());

        let mut line = Line(b"NOOP".to_vec());
        let (cmd, arg) = line.split().unwrap();
        assert_eq!("NOOP".as_bytes(), &*cmd);
        assert!(arg.is_none());
    }
}
