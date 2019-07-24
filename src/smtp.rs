use crate::commands::Command;
use crate::message::Message;
use crate::responses::Response;
use crate::settings::Settings;
use base64;
use std::io;
use tokio::codec::{Framed, LinesCodec};
use tokio::prelude::*;

#[derive(Clone, Copy)]
pub enum Authentication {
    ReceiveAuthCommand,
    ReceivePlainAuth,
}

#[derive(Clone, Copy)]
pub enum State {
    SendGreeting,
    ReceiveGreeting,
    Authenticate(Authentication),
    Rejected,
    Accept,
    AcceptData,
    End,
}

#[derive(Clone)]
pub enum PollComplete {
    Yes,
    No,
}

/// A struct that implements Future that is responsible for the dialog
/// between client and server to receive an SMTP message.
pub struct Smtp<'a, T> {
    pub settings: &'a Settings,
    pub socket: Framed<T, LinesCodec>,
    pub state: (PollComplete, State),
    pub message: Option<Message>,
}

impl<'a, T> Smtp<'a, T>
where
    T: AsyncRead + AsyncWrite,
{
    pub fn new(settings: &'a Settings, socket: Framed<T, LinesCodec>) -> Self {
        Smtp {
            settings,
            socket,
            state: (PollComplete::No, State::SendGreeting),
            message: None,
        }
    }

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

    fn respond(&mut self, response: Response) -> Result<AsyncSink<String>, io::Error>
    where
        T: AsyncRead + AsyncWrite,
    {
        self.socket.start_send(response.as_string())
    }

    fn authentication_poll(&mut self, stage: &Authentication) -> Poll<(), io::Error> {
        match try_ready!(self.socket.poll()) {
            Some(ref msg) => match stage {
                Authentication::ReceiveAuthCommand => match Command::from_str(msg) {
                    Some(Command::AUTH(_)) => {
                        self.respond(Response::_334_Authenticate)?;
                        self.state = (
                            PollComplete::Yes,
                            State::Authenticate(Authentication::ReceivePlainAuth),
                        );
                    }
                    _ => {
                        self.respond(Response::_503_BadSequence)?;
                        self.state = (
                            PollComplete::Yes,
                            State::Authenticate(Authentication::ReceiveAuthCommand),
                        );
                    }
                },

                Authentication::ReceivePlainAuth => {
                    if &base64::encode(&self.settings.password) == msg {
                        self.respond(Response::_235_AuthenticationSuccessful)?;
                        self.state = (PollComplete::Yes, State::Accept);
                    } else {
                        self.respond(Response::_535_FailedAuthentication)?;
                        self.state = (
                            PollComplete::Yes,
                            State::Authenticate(Authentication::ReceiveAuthCommand),
                        );
                    }
                }
            },
            None => {
                self.respond(Response::_502_CommandNotImplemented)?;
                self.state = (PollComplete::Yes, State::Authenticate(stage.clone()));
            }
        }

        Ok(Async::Ready(()))
    }
}

impl<'a, T> Future for Smtp<'a, T>
where
    T: AsyncRead + AsyncWrite,
{
    type Item = Option<Message>;
    type Error = io::Error;

    /// poll implements a state machine that handles the various states that
    /// occur whilst receiving an SMTP message.
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match &self.state.clone() {
                (PollComplete::No, State::SendGreeting) => {
                    // Send the initial greeting.
                    self.respond(Response::_220_ServiceReady("smteepee"))?;
                    self.state = (PollComplete::Yes, State::ReceiveGreeting);
                }

                (PollComplete::No, State::ReceiveGreeting) => {
                    // The first command we must recieve must be an EHLO or a HELO command.
                    // Then if it is correct we can get on with the main command loop.
                    match try_ready!(self.socket.poll()) {
                        Some(ref msg) => match Command::from_str(msg) {
                            Some(Command::HELO(_)) => {
                                self.respond(Response::_250_Completed(&format!(
                                    "{}, I hope this day finds you well.",
                                    self.settings.domain
                                )))?;
                                self.state = (PollComplete::Yes, State::Accept);
                            }
                            Some(Command::EHLO(_)) => {
                                self.respond(Response::_250_Completed(&format!(
                                    "{}, I hope this day finds you well.",
                                    self.settings.domain
                                )))?;
                                self.respond(Response::_250_Completed("AUTH PLAIN"))?;
                                self.state = (
                                    PollComplete::Yes,
                                    State::Authenticate(Authentication::ReceiveAuthCommand),
                                );
                            }
                            Some(_) => {
                                self.respond(Response::_503_BadSequence)?;
                                self.state = (PollComplete::Yes, State::ReceiveGreeting);
                            }
                            None => {
                                self.respond(Response::_502_CommandNotImplemented)?;
                                self.state = (PollComplete::Yes, State::ReceiveGreeting);
                            }
                        },
                        _ => self.state = (PollComplete::No, State::Rejected),
                    }
                }

                (PollComplete::No, State::Authenticate(stage)) => {
                    try_ready!(self.authentication_poll(stage))
                }

                (PollComplete::No, State::Accept) => match try_ready!(self.socket.poll()) {
                    // The main command loop over which the email contents are sent.
                    Some(msg) => match Command::from_str(&msg) {
                        Some(Command::MAIL(from)) => {
                            self.set_from(from);
                            self.respond(Response::_250_Completed("OK"))?;
                            self.state = (PollComplete::Yes, State::Accept);
                        }
                        Some(Command::RCPT(to)) => {
                            self.set_rcpt(to);
                            self.respond(Response::_250_Completed("OK"))?;
                            self.state = (PollComplete::Yes, State::Accept);
                        }
                        Some(Command::VRFY(addr)) => {
                            // Currently we verify all addresses as ok..
                            self.respond(Response::_250_Completed(&addr))?;
                            self.state = (PollComplete::Yes, State::Accept);
                        }
                        Some(Command::DATA) => {
                            self.respond(Response::_354_StartMailInput)?;
                            self.state = (PollComplete::Yes, State::AcceptData);
                        }
                        Some(Command::QUIT) => {
                            self.respond(Response::_221_ServiceClosing)?;
                            self.state = (PollComplete::Yes, State::End);
                        }
                        _ => {
                            self.respond(Response::_503_BadSequence)?;
                            self.state = (PollComplete::Yes, State::Rejected);
                        }
                    },
                    _ => self.state = (PollComplete::No, State::Rejected),
                },

                (PollComplete::No, State::AcceptData) => match try_ready!(self.socket.poll()) {
                    // In this state we are getting the main body text of the email, one line at a time.
                    // When we get a "." by itself we are finished.
                    Some(msg) => {
                        if msg == "." {
                            self.respond(Response::_250_Completed("OK"))?;
                            self.state = (PollComplete::Yes, State::Accept);
                        } else {
                            self.set_body(msg);
                        }
                    }
                    _ => {}
                },

                (PollComplete::No, State::Rejected) => {
                    self.socket.start_send("Error".to_string())?;
                    self.state = (PollComplete::Yes, State::End);
                }

                (PollComplete::No, State::End) => {
                    return Ok(Async::Ready(self.message.take()));
                }

                (PollComplete::Yes, state) => {
                    try_ready!(self.socket.poll_complete());
                    self.state = (PollComplete::No, state.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::dummy_socket::DummySocket;
    use crate::settings::Settings;
    use crate::smtp::Smtp;
    use std::sync::mpsc;
    use tokio::codec::{Framed, LinesCodec};
    use tokio::prelude::*;

    /// Convenience function to create the Smtp future that receives
    /// the given data from the socket.
    fn create_socket_with(data: &str, sender: mpsc::Sender<Vec<u8>>) -> Smtp<DummySocket> {
        let socket = DummySocket::new_with_channel(data.into(), sender);
        let framed = Framed::new(socket, LinesCodec::new());
        Smtp::new(Settings::default(), framed)
    }

    #[test]
    fn test_greeting() {
        let (sender, receiver) = mpsc::channel();
        let smtp = create_socket_with("HELO\n", sender);
        tokio::run(
            smtp.map(move |_| {
                assert_eq!(
                    "220 local ESMTP smteepee Service Ready\n",
                    String::from_utf8(receiver.recv().unwrap()).unwrap()
                )
            })
            .map_err(|err| eprintln!("{:?}", err)),
        );
    }

    #[test]
    fn test_from() {
        let (sender, receiver) = mpsc::channel();
        let smtp = create_socket_with("HELO\nMAIL FROM:<onk@ponk.com >", sender);
        tokio::run(
            smtp.map(move |msg| {
                // The initial greeting message.
                assert_eq!(
                    "220 local ESMTP smteepee Service Ready\n",
                    String::from_utf8(receiver.recv().unwrap()).unwrap()
                );
                // The accept message.
                assert_eq!(
                    "250 groove.com, I hope this day finds you well.\n",
                    String::from_utf8(receiver.recv().unwrap()).unwrap()
                );
                // 250 OK is returned when we send a from.
                assert_eq!(
                    "250 OK\n",
                    String::from_utf8(receiver.recv().unwrap()).unwrap()
                );
                assert_eq!(Some(Some("onk@ponk.com".to_string())), msg.map(|m| m.from));
            })
            .map_err(|err| eprintln!("{:?}", err)),
        );
    }

    #[test]
    fn test_rcpt() {
        let (sender, receiver) = mpsc::channel();
        let smtp = create_socket_with(
            "HELO\nRCPT TO: <onk@ponk.com>\nRCPT TO:<pook@ook.co.uk>\n",
            sender,
        );
        tokio::run(
            smtp.map(move |msg| {
                assert_eq!(
                    "220 local ESMTP smteepee Service Ready\n",
                    String::from_utf8(receiver.recv().unwrap()).unwrap()
                );
                // The accept message.
                assert_eq!(
                    "250 groove.com, I hope this day finds you well.\n",
                    String::from_utf8(receiver.recv().unwrap()).unwrap()
                );
                // 250 OK is returned when we send a recipient.
                // We get two of them as there are two recipients.
                assert_eq!(
                    "250 OK\n",
                    String::from_utf8(receiver.recv().unwrap()).unwrap()
                );
                assert_eq!(
                    "250 OK\n",
                    String::from_utf8(receiver.recv().unwrap()).unwrap()
                );

                assert_eq!(
                    Some(vec![
                        "onk@ponk.com".to_string(),
                        "pook@ook.co.uk".to_string(),
                    ]),
                    msg.map(|m| m.to)
                );
            })
            .map_err(|err| eprintln!("{:?}", err)),
        );
    }

}
