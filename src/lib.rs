extern crate futures;
extern crate tokio_core;
extern crate tokio_proto;
extern crate tokio_service;
extern crate battista_ftp_parser as parser;

use std::io;
use futures::future::FutureResult;
use tokio_core::io::{Codec, EasyBuf, Framed, Io};
use tokio_proto::TcpServer;
use tokio_proto::pipeline::ServerProto;
use tokio_service::Service;

use parser::command::{Command, DecodeError};
use parser::reply::Reply;

pub struct FtpCodec;

impl Codec for FtpCodec {
    type In = Command;
    type Out = Reply;

    fn decode(&mut self, buf: &mut EasyBuf) -> io::Result<Option<Self::In>> {
        match Command::decode(&mut buf.get_mut()) {
            Ok(cmd) => Ok(Some(cmd)),
            Err(DecodeError::MissingInput) => Ok(None),
            Err(err) => Err(io::ErrorKind::Other.into())
        }
    }

    fn encode(&mut self, reply: Self::Out, buf: &mut Vec<u8>) -> io::Result<()> {
        Ok(())
    }
}

pub struct FtpProto;

impl<T: Io + 'static> ServerProto<T> for FtpProto {
    type Request = Command;

    /// For this protocol style, `Response` matches the coded `Out` type
    type Response = Reply;

    /// A bit of boilerplate to hook in the codec:
    type Transport = Framed<T, FtpCodec>;
    type BindTransport = Result<Self::Transport, io::Error>;
    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(io.framed(FtpCodec))
    }
}

pub struct FtpService;

impl Service for FtpService {
    type Request = Command;
    type Response = Reply;

    // For non-streaming protocols, service errors are always io::Error
    type Error = io::Error;

    // The future for computing the response; box it for simplicity.
    type Future = FutureResult<Self::Response, Self::Error>;

    // Produce a future for computing a response from a request.
    fn call(&self, req: Self::Request) -> Self::Future {
        println!("{:?}", req);
        futures::future::ok(Reply)
    }
}

pub fn run() {
    let addr = "0.0.0.0:2121".parse().unwrap();
    let server = TcpServer::new(FtpProto, addr);
    server.serve(|| Ok(FtpService));
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
