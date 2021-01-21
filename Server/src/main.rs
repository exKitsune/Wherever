use std::io::{self};
use std::net::SocketAddr;
use std::thread;

use tiny_http::{Method, Response};

fn main() -> io::Result<()> {
    let addr: SocketAddr = ([0, 0, 0, 0], 8998).into();
    let succ_response = || Response::new(200.into(), vec![], b"Yeet".as_ref(), Some(4), None);
    let fail_response = || Response::new(400.into(), vec![], b"Not Good".as_ref(), Some(8), None);
    let server =
        tiny_http::Server::http(addr).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    server
        .incoming_requests()
        .inspect(|req| {
            println!("{:?}", req);
        })
        .map(|mut req| match (req.method(), req.url()) {
            (Method::Post, "/open") => {
                let mut url = String::new();
                req.as_reader().read_to_string(&mut url)?;
                println!("Opening {:?}", url);
                thread::spawn(|| open::that_in_background(url));
                req.respond(succ_response())
            }
            _ => req.respond(fail_response()),
        })
        .for_each(|res| {
            println!("{:?}", res);
        });

    Ok(())
}
