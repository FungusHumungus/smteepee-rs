use std::io;
use std::time;
use std::net;
use std::env;
use std::path;
use tokio::codec::{Framed, LinesCodec};
use tokio::net::tcp::TcpListener;
use tokio::prelude::*;

#[macro_use]
extern crate futures;
#[macro_use]
extern crate lazy_static;

mod settings;
mod config;
mod message;
mod commands;
mod responses;
mod smtp;


#[cfg(test)]
mod dummy_socket;

/// Get the address to listen to.
fn get_listen_address(protocol: u8, port: u16) -> net::SocketAddr {
    match protocol {
        4 => {
            net::SocketAddr::new(net::IpAddr::V4(net::Ipv4Addr::new(0, 0, 0, 0)), port)
        }
        6 => {
            net::SocketAddr::new(net::IpAddr::V6(net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), port)
        }
        _ => panic!("Protocol must be either 4 or 6")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
   
    // Load the settings from the file specified in the first argument.
    let args: Vec<_> = env::args().collect();
    let settings = if args.len() > 1 {
        let path = args[1].clone();
        settings::Settings::load(path::Path::new(&path))?
    } else {
        settings::Settings::default()
    };

    // Setup the socket.
    let addr = get_listen_address(settings.protocol, settings.port);
    let listener = TcpListener::bind(&addr)?;
    let incoming = listener.incoming();

    // Run up the server.
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

    println!("Listening on {}", settings.port);
    tokio::run(server);

    Ok(())
}
