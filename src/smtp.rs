use crate::commands::Command;
use crate::message::Message;
use crate::responses::Response;
use crate::settings::Settings;
use base64;
use futures::sink::*;
use std::error;
use tokio::prelude::*;
use tokio::stream::StreamExt;
use tokio_util::codec::{Framed, LinesCodec};

#[derive(Clone, Copy)]
pub enum Authentication {
    ReceiveAuthCommand,
    ReceivePlainAuth,
}

#[derive(Clone, Copy)]
pub enum State {
    SendGreeting,
    ReceiveGreeting,
    Rejected,
    Accept,
    AcceptData,
    End,
}

async fn respond<'a, T>(
    stream: &mut Framed<T, LinesCodec>,
    response: Response<'a>,
) -> Result<(), Box<dyn error::Error>>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    stream.send(response.as_string()).await?;
    Ok(())
}

async fn authentication<T>(
    mut stream: &mut Framed<T, LinesCodec>,
    settings: &Settings,
) -> Result<bool, Box<dyn error::Error>>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    let mut stage = Authentication::ReceiveAuthCommand;

    loop {
        if let Some(line) = stream.next().await {
            match stage {
                Authentication::ReceiveAuthCommand => match Command::from_str(&line?) {
                    Some(Command::AUTH(_)) => {
                        respond(&mut stream, Response::_334_Authenticate).await?;
                        stage = Authentication::ReceivePlainAuth;
                    }
                    _ => {
                        respond(&mut stream, Response::_503_BadSequence).await?;
                        stage = Authentication::ReceiveAuthCommand;
                    }
                },

                Authentication::ReceivePlainAuth => {
                    if base64::encode(&settings.password) == line? {
                        respond(&mut stream, Response::_235_AuthenticationSuccessful).await?;
                        return Ok(true);
                    } else {
                        respond(&mut stream, Response::_535_FailedAuthentication).await?;
                        stage = Authentication::ReceiveAuthCommand;
                    }
                }
            }
        }
    }
}

pub async fn converse<T: AsyncRead + AsyncWrite + Unpin>(
    mut stream: Framed<T, LinesCodec>,
    settings: &Settings,
) -> Result<Message, Box<dyn error::Error>> {
    let mut message = Message::new();
    let mut state = State::SendGreeting;

    loop {
        match state {
            State::SendGreeting => {
                // Send the initial greeting.
                respond(&mut stream, Response::_220_ServiceReady("smteepee")).await?;
                state = State::ReceiveGreeting;
            }

            State::ReceiveGreeting => {
                if let Some(line) = stream.next().await {
                    // The first command we must recieve must be an EHLO or a HELO command.
                    // Then if it is correct we can get on with the main command loop.
                    match Command::from_str(&line?) {
                        Some(Command::HELO(_)) => {
                            respond(
                                &mut stream,
                                Response::_250_Completed(&format!(
                                    "{}, I hope this day finds you well.",
                                    settings.domain
                                )),
                            )
                            .await?;
                            state = State::Accept;
                        }
                        Some(Command::EHLO(_)) => {
                            respond(
                                &mut stream,
                                Response::_250_Completed(&format!(
                                    "{}, I hope this day finds you well.",
                                    settings.domain
                                )),
                            )
                            .await?;
                            respond(&mut stream, Response::_250_Completed("AUTH PLAIN")).await?;

                            // Authentication must pass before we can get beyond this stage.
                            if authentication(&mut stream, settings).await? == true {
                                state = State::Accept;
                            } else {
                                state = State::End;
                            }
                        }
                        Some(_) => {
                            respond(&mut stream, Response::_503_BadSequence).await?;
                            state = State::ReceiveGreeting;
                        }
                        None => {
                            respond(&mut stream, Response::_502_CommandNotImplemented).await?;
                            state = State::ReceiveGreeting;
                        }
                    }
                }
            }

            State::Accept => {
                if let Some(line) = stream.next().await {
                    // The main command loop over which the email contents are sent.
                    match Command::from_str(&line?) {
                        Some(Command::MAIL(from)) => {
                            message.from = Some(from);
                            respond(&mut stream, Response::_250_Completed("OK")).await?;
                        }
                        Some(Command::RCPT(to)) => {
                            message.to.push(to);
                            respond(&mut stream, Response::_250_Completed("OK")).await?;
                        }
                        Some(Command::VRFY(addr)) => {
                            // Currently we verify all addresses as ok..
                            respond(&mut stream, Response::_250_Completed(&addr)).await?;
                        }
                        Some(Command::DATA) => {
                            respond(&mut stream, Response::_354_StartMailInput).await?;
                            state = State::AcceptData;
                        }
                        Some(Command::QUIT) => {
                            respond(&mut stream, Response::_221_ServiceClosing).await?;
                            state = State::End;
                        }
                        _ => {
                            respond(&mut stream, Response::_503_BadSequence).await?;
                            state = State::Rejected;
                        }
                    }
                }
            }

            State::AcceptData => {
                // In this state we are getting the main body text of the email, one line at a time.
                // When we get a "." by itself we are finished.
                if let Some(msg) = stream.next().await {
                    let msg = msg?;
                    if msg == "." {
                        respond(&mut stream, Response::_250_Completed("OK")).await?;
                        state = State::Accept;
                    } else {
                        message.data.push(msg);
                    }
                }
            }

            State::Rejected => {
                stream.send("Error".to_string()).await?;
                state = State::End;
            }

            State::End => {
                return Ok(message);
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::settings::Settings;
    use crate::smtp::converse;
    use tokio_test::{block_on, io};
    use tokio_util::codec::{Framed, LinesCodec};

    #[test]
    fn test_greeting() {
        let stream = io::Builder::new()
            .write(b"220 local ESMTP smteepee Service Ready\n")
            .read(b"HELO\n")
            .write(b"250 groove.com, I hope this day finds you well.\n")
            .read(b"QUIT\n")
            .write(b"221 Bye\n")
            .build();
        let framed = Framed::new(stream, LinesCodec::new());
        let _message = block_on(converse(framed, &Settings::default()));
    }

    #[test]
    fn test_from() {
        let stream = io::Builder::new()
            .write(b"220 local ESMTP smteepee Service Ready\n")
            .read(b"HELO\n")
            .write(b"250 groove.com, I hope this day finds you well.\n")
            .read(b"MAIL FROM:<onk@ponk.com>\n")
            .write(b"250 OK\n")
            .read(b"QUIT\n")
            .write(b"221 Bye\n")
            .build();
        let framed = Framed::new(stream, LinesCodec::new());
        let message = block_on(converse(framed, &Settings::default()));

        assert_eq!(Some("onk@ponk.com".to_string()), message.unwrap().from);
    }

    #[test]
    fn test_rcpt() {
        let stream = io::Builder::new()
            .write(b"220 local ESMTP smteepee Service Ready\n")
            .read(b"HELO\n")
            .write(b"250 groove.com, I hope this day finds you well.\n")
            .read(b"RCPT TO: <onk@ponk.com>\n")
            .write(b"250 OK\n")
            .read(b"RCPT TO:<pook@ook.co.uk>\n")
            .write(b"250 OK\n")
            .read(b"QUIT\n")
            .write(b"221 Bye\n")
            .build();
        let framed = Framed::new(stream, LinesCodec::new());
        let message = block_on(converse(framed, &Settings::default()));

        assert_eq!(
            vec!["onk@ponk.com".to_string(), "pook@ook.co.uk".to_string()],
            message.unwrap().to
        );
    }
}
