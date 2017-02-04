use std::io;
use std::mem;

#[derive(Debug)]
pub struct Command {
    line: Vec<u8>,
}

const MAX_LINE_SIZE: usize = 8 * 1024;


impl Command {
    fn new(line: Vec<u8>) -> Self {
        Command {
            line: line,
        }
    }

    pub fn decode(buf: &mut Vec<u8>) -> io::Result<Option<Self>> {
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
/*
    pub fn command(&self) -> Result<&str, ParseError> {
        
    }
*/
}
