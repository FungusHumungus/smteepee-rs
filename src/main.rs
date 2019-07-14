use tokio::codec::{Framed, LinesCodec};
use tokio::net::tcp::TcpListener;
use tokio::prelude::*;

mod smtp;
mod config;
mod message;



fn main() {
    let addr = "127.0.0.1:2525".parse().unwrap();
    let listener = TcpListener::bind(&addr).expect("Unable to listen");

    let incoming = listener.incoming();

    let server = incoming
        .map_err(|e| eprintln!("Accept failed = {:?}", e))
        .for_each(|socket| {
            let framed = Framed::new(socket, LinesCodec::new());

            let handle = smtp::Smtp {
                config: config::Config {
                    domain: "groove.com".to_string(),
                },
                socket: framed,
                state: (true, smtp::State::SendGreeting),
                message: None,
            };

            tokio::spawn(
                handle
                    .map(|message| println!("{:?}", message))
                    .map_err(|err| eprintln!("Error {:?}", err)),
            )
        });

    println!("Listening on 2525");
    tokio::run(server);
}
