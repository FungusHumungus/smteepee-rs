use std::{env, net, path, time};
use tokio_util::codec::{Framed, LinesCodec};
use tokio::{net::TcpListener, stream::StreamExt};

#[macro_use]
extern crate lazy_static;

mod commands;
mod message;
mod responses;
mod settings;
mod smtp;

/// Get the address to listen to.
fn get_listen_address(protocol: u8, port: u16) -> net::SocketAddr {
    match protocol {
        4 => net::SocketAddr::new(net::IpAddr::V4(net::Ipv4Addr::new(0, 0, 0, 0)), port),
        6 => net::SocketAddr::new(
            net::IpAddr::V6(net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
            port,
        ),
        _ => panic!("Protocol must be either 4 or 6"),
    }
}

/// Load the settings from the file specified in the first argument.
fn load_settings() -> Result<settings::Settings, Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() > 1 {
        let path = args[1].clone();
        settings::Settings::load(path::Path::new(&path))
    } else {
        Ok(settings::Settings::default())
    }
}

// Load settings from a toml file if it has beet specified.
// Else use the defaults.
lazy_static! {
    static ref SETTINGS:settings::Settings = load_settings().unwrap();
}


/// The main function.
/// Sets up the socket and handles incoming requests.
#[tokio::main]
async fn main() {

    // Setup the socket.
    let addr = get_listen_address(SETTINGS.protocol, SETTINGS.port);
    let mut listener = TcpListener::bind(&addr).await.unwrap();

    while let Some(stream) = listener.next().await {
        match stream {
            Ok(stream) => {
                let framed = Framed::new(stream, LinesCodec::new());
                match smtp::converse(framed, &SETTINGS).await {
                    Ok (message) => { 
                        let now = time::SystemTime::now();
                        match now.duration_since(time::SystemTime::UNIX_EPOCH) {
                            Ok(n) => {
                                message.save_to_file(format!("./received/{}.eml", n.as_millis())).await.unwrap();
                                println!("New email: {}", n.as_millis());
                            },
                            Err(_) => {
                                // TODO Insert some kind of McFly joke...
                                eprintln!("We have gone back in time!");
                            },
                        }
                    }
                    Err (err) => eprintln!("{}", err)
                }
            }

            Err(e) => eprintln!("Connection failed {}", e)
        }
    }
}
