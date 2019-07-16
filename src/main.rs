use std::io;
use std::time;
use std::net;
use net2::TcpBuilder;
use tokio::codec::{Framed, LinesCodec};
use tokio::net::tcp::TcpListener;
use tokio::prelude::*;
use tokio::reactor::Handle;

#[macro_use]
extern crate futures;
#[macro_use]
extern crate lazy_static;

mod config;
mod message;
mod commands;
mod responses;
mod smtp;


#[cfg(test)]
mod dummy_socket;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let addr4 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 1)), 2525);
    /*let tcp = TcpBuilder::new_v6()?//.reuse_address(true)?
                                   .only_v6(false)?
                                   .bind("[::1]:2525")?
                                   .to_tcp_listener()?;
    let listener = TcpListener::from_std( 
            tcp,
            &Handle::default(),
        ).expect("Unable to listen");
        */

    let addr6 = net::SocketAddr::new(net::IpAddr::V6(net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 2525);
    let listener = TcpListener::bind(&addr6)?;
    let incoming = listener.incoming();

    let server = incoming
        .map_err(|e| eprintln!("Accept failed = {:?}", e))
        .for_each(|socket| {
            
            // TODO : SMTP line endings are CRLF.
            // We are going to need to create our own Codec that can handle this specifically.
            let framed = Framed::new(socket, LinesCodec::new());

            let handle = smtp::Smtp::new(
                config::Config {
                    domain: "groove.com".to_string(),
                },
                framed,
            );

            tokio::spawn(
                handle
                    .and_then(|message| {
                        // Ensure that a message has actually been created.
                        // Error if it hasn't
                        match message {
                            Some(message) => future::ok(message),
                            None => future::err(io::Error::new(
                                io::ErrorKind::Other,
                                "No message created",
                            )),
                        }
                    })
                    .and_then(|message| {
                        // Save the message to a file.
                        let now = time::SystemTime::now();
                        match now.duration_since(time::SystemTime::UNIX_EPOCH) {
                            Ok(n) => future::Either::A(
                                message.save_to_file(format!("./received/{}.eml", n.as_millis())),
                            ),
                            Err(_) => future::Either::B(future::err(io::Error::new(
                                io::ErrorKind::Other,
                                "We have gone back in time!",
                            ))),
                        }
                    })
                    .map(|message| println!("{} : Email sent to {:?}", message.saved.unwrap_or("Err".to_string()), message.to))
                    .map_err(|err| eprintln!("Error {:?}", err)),
            )
        });

    println!("Listening on 2525");
    tokio::run(server);

    Ok(())
}
