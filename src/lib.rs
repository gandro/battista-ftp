extern crate futures;
extern crate tokio_core;
extern crate tokio_proto;
extern crate tokio_service;

use std::io;
use tokio_core::io::{Codec, EasyBuf};

use self::command::Command;
use self::reply::Reply;

mod reply;
mod command;

pub struct FtpCodec;

impl Codec for FtpCodec {
    type In = Command;
    type Out = Reply;

    fn decode(&mut self, buf: &mut EasyBuf) -> io::Result<Option<Self::In>> {
        Command::decode(buf)
    }

    fn encode(&mut self, msg: Self::Out, buf: &mut Vec<u8>) -> io::Result<()> {
        Ok(())
    }
}

pub fn run() {}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
