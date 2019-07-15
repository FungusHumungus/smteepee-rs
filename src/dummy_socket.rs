use tokio::io;
use tokio::prelude::*;
use std::sync::mpsc;


/// The DummySocket is used in testing to simulate a connection to a server.
/// It implements the AsyncRead and AsyncWrite traits.
/// Data passed to it is the data that is received from the simulated server.
/// Data sent to the server is stored in the recieved variable and optionally sent 
/// through the given mspc channel once the data is flushed.
pub struct DummySocket {
    data: Vec<u8>,
    temp_received: Vec<u8>,
    received: Vec<u8>,
    pos: usize,
    channel: Option<mpsc::Sender<Vec<u8>>>,
}


impl DummySocket {
    /// Create a new server without a channel.
    pub fn new(data: Vec<u8>) -> Self {
        DummySocket {
            data,
            temp_received: Vec::new(),
            received: Vec::new(),
            pos: 0,
            channel: None,
        }
    }
    
    /// Create a new dummy server with a channel through which any data sent to this 
    /// server is sent.
    pub fn new_with_channel(data: Vec<u8>, channel: mpsc::Sender<Vec<u8>>) -> Self {
        DummySocket {
            data,
            temp_received: Vec::new(),
            received: Vec::new(),
            pos: 0,
            channel: Some(channel),
        }
    }
}

impl Read for DummySocket {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        for i in 0..buf.len() {
            if self.pos >= self.data.len() {
                return Ok(i);
            }
            buf[i] = self.data[self.pos];
            self.pos += 1;
        }

        Ok(buf.len())
    }
}

impl Write for DummySocket {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        self.temp_received.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        if let Some(channel) = &self.channel {
            channel.send(self.temp_received.clone()).unwrap();
        }

        self.received.append(&mut self.temp_received);
        
        Ok(())
    }
}

impl AsyncRead for DummySocket {
}

impl AsyncWrite for DummySocket {
    fn shutdown(&mut self) -> Result<Async<()>, io::Error> {
        Ok(Async::Ready(()))
    }
}



#[test]
fn test_read() {
    let mut socket = DummySocket::new("Groove on a lot".into());
    let mut buf = [0; 10];

    assert_eq!(10, socket.read(&mut buf).unwrap());
    assert_eq!("Groove on ", String::from_utf8(buf.to_vec()).unwrap());
    assert_eq!(5, socket.read(&mut buf).unwrap());
    assert_eq!("a lot", String::from_utf8(buf[0..5].to_vec()).unwrap());
}

#[test]
fn test_read_to_string() {
    let mut socket = DummySocket::new("Groove on a lot".into());
    let mut read = String::new();

    assert_eq!(15, socket.read_to_string(&mut read).unwrap());
    assert_eq!("Groove on a lot", read);
}

#[test]
fn test_write() {
    let mut socket = DummySocket::new("Groove on a lot".into());
    write!(socket, "{}", "Rock on!").unwrap();
    
    assert!(socket.flush().is_ok());
    assert_eq!("Rock on!", String::from_utf8(socket.received).unwrap());
}
