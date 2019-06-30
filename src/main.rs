use tokio::codec::Decoder;
use tokio::codec::LinesCodec;
use tokio::net::tcp::TcpListener;
use tokio::prelude::*;

enum State {
    SendGreeting,
    ReceiveGreeting,
    Accepted,
    Rejected,
    Accept,
    AcceptData,
    End,
}

#[derive(Debug)]
struct Message {
    from: Option<String>,
    to: Vec<String>,
    data: Option<String>,
    count: u64,
}

impl Message {
    fn new() -> Self {
        Message {
            from: None,
            to: Vec::new(),
            data: None,
            count: 0,
        }
    }
}

fn step_next<T>(state: State, msg: Message, io: T) -> Box<Future<Item = (T, State, Message), Error = ()>>
where
    T: AsyncRead + AsyncWrite,
{
    match state {
        SendGreeting => 
            Box::new(tokio::io::write_all(io, "220 local ESMTP smteepee")
                     .map_err(|e| eprintln!("Error {:?}", e))
                     .map(|(io, _)| (io, State::ReceiveGreeting, msg))),
        
        _ =>
            Box::new(tokio::io::write_all(io, "BYE")
                     .map_err(|e| eprintln!("Error {:?}", e))
                     .map(|(io, _)| (io, State::End, msg))),
    }
}

fn main() {
    let addr = "127.0.0.1:2525".parse().unwrap();
    let listener = TcpListener::bind(&addr).expect("Unable to listen");

    let incoming = listener.incoming();

    let server = incoming
        .map_err(|e| eprintln!("Accept failed = {:?}", e))
        .for_each(|socket| {
            let handle = tokio::io::write_all(socket, "220 local ESMTP smteepee")
                .and_then(|(socket, _)| {
                    let (tx, rx) = LinesCodec::new().framed(socket).split();

                    let writes = rx
                        .take_while(|response| future::ok(response != "BYE"))
                        .fold((tx, Message::new()), |(tx, mut message), response| {
                            message.count += 1;
                            tx.send(response).map(|tx| (tx, message))
                        });

                    writes
                    //tokio::spawn(writes.then(move |_| Ok(())));
                    //Ok(())
                })
                .map(|(_, arg)| println!("Message {:?}", arg))
                .map_err(|e| eprintln!("Err {:?}", e));

            tokio::spawn(handle)
        });

    println!("Listening on 2525");
    tokio::run(server);
}
