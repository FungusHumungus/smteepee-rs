use crate::config::Config;
use crate::message::Message;
use std::io;
use tokio::codec::{Framed, LinesCodec};
use tokio::prelude::*;

#[derive(Clone, Copy)]
pub enum State {
    SendGreeting,
    ReceiveGreeting,
    Accepted,
    Rejected,
    Accept,
    AcceptData,
    End,
}

/// A struct that implements Future that is responsible for the dialog
/// between client and server to receive an SMTP message.
pub struct Smtp<T> {
    pub config: Config,
    pub socket: Framed<T, LinesCodec>,
    pub state: (bool, State),
    pub message: Option<Message>,
}

impl<T> Smtp<T> {
    /// Creates a message if there isn't one.
    fn set_message(&mut self) {
        if self.message.is_none() {
            self.message = Some(Message::new());
        }
    }

    fn set_from(&mut self, from: String) {
        self.set_message();

        match self.message.as_mut() {
            Some(m) => m.from = Some(from),
            None => {}
        }
    }

    fn set_rcpt(&mut self, to: String) {
        self.set_message();

        match self.message.as_mut() {
            Some(m) => m.to.push(to),
            None => {}
        }
    }

    fn set_body(&mut self, data: String) {
        self.set_message();

        match self.message.as_mut() {
            Some(m) => m.data.push(data),
            None => {}
        }
    }
}

impl<T> Future for Smtp<T>
where
    T: AsyncRead + AsyncWrite,
{
    type Item = Option<Message>;
    type Error = io::Error;

    /// poll implements a state machine that handles the various states that
    /// occur whilst receiving an SMTP message.
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match &self.state {
                (true, State::SendGreeting) => {
                    self.socket
                        .start_send("220 local ESMTP smteepee".to_string())?;
                    self.state = (false, State::ReceiveGreeting);
                }
                (true, State::ReceiveGreeting) => match try_ready!(self.socket.poll()) {
                    Some(ref msg) if msg.starts_with("HELO") => {
                        self.state = (false, State::Accepted);
                    }
                    _ => self.state = (false, State::Rejected),
                },
                (true, State::Accepted) => {
                    let message = format!(
                        "250 {}, I hope this day finds you well.",
                        self.config.domain
                    );
                    self.socket.start_send(message)?;
                    self.state = (false, State::Accept);
                }
                (true, State::Accept) => match try_ready!(self.socket.poll()) {
                    Some(msg) => {
                        if msg.starts_with("MAIL FROM:") {
                            self.set_from(msg);
                            self.socket.start_send("250 OK".to_string())?;
                            self.state = (false, State::Accept);
                        } else if msg.starts_with("RCPT TO:") {
                            self.set_rcpt(msg);
                            self.socket.start_send("250 OK".to_string())?;
                            self.state = (false, State::Accept);
                        } else if msg.starts_with("DATA") {
                            self.socket
                                .start_send("354 End data with <CR><LF>.<CR><LF>".to_string())?;
                            self.state = (false, State::AcceptData);
                        } else if msg.starts_with("QUIT") {
                            self.socket.start_send("221 Bye".to_string())?;
                            self.state = (false, State::End);
                        } else {
                            self.state = (false, State::Rejected);
                        }
                    }
                    _ => self.state = (false, State::Rejected),
                },
                (true, State::AcceptData) => match try_ready!(self.socket.poll()) {
                    Some(msg) => {
                        if msg == "." {
                            self.socket
                                .start_send("250 Ok: queued as plork".to_string())?;
                            self.state = (false, State::Accept);
                        } else {
                            self.set_body(msg);
                        }
                    }
                    _ => {}
                },
                (true, State::Rejected) => {
                    self.socket.start_send("Error".to_string())?;
                    self.state = (false, State::End);
                }
                (true, State::End) => {
                    return Ok(Async::Ready(self.message.take()));
                }
                (false, state) => {
                    try_ready!(self.socket.poll_complete());
                    self.state = (true, state.clone());
                }
            }
        }
    }
}
