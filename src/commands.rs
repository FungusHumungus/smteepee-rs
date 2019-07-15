use regex::{Regex, RegexBuilder};

#[derive(PartialEq, Eq, Debug)]
pub enum Command {
    EHLO(String),
    HELO(String),
    MAIL(String),
    RCPT(String),
    DATA,
    RSET,
    NOOP,
    QUIT,
    VRFY,
}

/// Build a case insensitive regex.
fn regex(re: &str) -> Regex {
    RegexBuilder::new(re)
        .case_insensitive(true)
        .build()
        .unwrap()
}

// Setup our regexes in advance.
lazy_static! {
    static ref EHLO: Regex = regex(r"EHLO");
    static ref HELO: Regex = regex(r"HELO");
    static ref MAIL: Regex = regex(r"MAIL FROM\s*:\s*<(.*)>");
    static ref RCPT: Regex = regex(r"RCPT TO\s*:\s*<(.*)>");
    static ref DATA: Regex = regex(r"DATA");
    static ref RSET: Regex = regex(r"RSET");
    static ref NOOP: Regex = regex(r"NOOP");
    static ref QUIT: Regex = regex(r"QUIT");
    static ref VRFY: Regex = regex(r"VRFY");
}

impl Command {
    /// Parses the message from the client.
    pub fn from_str(text: &str) -> Option<Self> {
        if EHLO.is_match(text) {
            // Extended HELLO message.
            let domain = EHLO.replace(text, "");
            Some(Command::EHLO(domain.trim().to_string()))
        } else if HELO.is_match(text) {
            // Normal HELLO message.
            let domain = HELO.replace(text, "");
            Some(Command::HELO(domain.trim().to_string()))
        } else if let Some(capture) = MAIL.captures(text) {
            // Initiate the message transaction with the address of the sender.
            let from = capture.get(1).unwrap().as_str();
            Some(Command::MAIL(from.trim().to_string()))
        } else if let Some(capture) = RCPT.captures(text) {
            // Recipients of the message.
            // TODO Similarly, relay hosts SHOULD strip or ignore source routes, and
            // names MUST NOT be copied into the reverse-path. 
            let to = capture.get(1).unwrap().as_str();
            Some(Command::RCPT(to.trim().to_string()))
        } else if DATA.is_match(text) {
            Some(Command::DATA)
        } else if RSET.is_match(text) {
            Some(Command::RSET)
        } else if NOOP.is_match(text) {
            Some(Command::NOOP)
        } else if QUIT.is_match(text) {
            Some(Command::QUIT)
        } else if VRFY.is_match(text) {
            Some(Command::VRFY)
        } else {
            None
        }
    }
}

#[test]
fn test_mail_command() {
    let command = Command::from_str("MAIL FROM: <ook@onk.com>");
    assert_eq!(Some(Command::MAIL("ook@onk.com".to_string())), command);
}

#[test]
fn test_rcpt_command() {
    let command = Command::from_str("RCPT TO: <ook@onk.com>");
    assert_eq!(Some(Command::RCPT("ook@onk.com".to_string())), command);
}
