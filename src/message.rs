use std::path::Path;
use tokio::fs;
use tokio::io;
use tokio::prelude::*;

#[derive(Debug, Clone)]
pub struct Message {
    pub from: Option<String>,
    pub to: Vec<String>,
    pub data: Vec<String>,
}

impl Message {
    pub fn new() -> Self {
        Message {
            from: None,
            to: Vec::new(),
            data: Vec::new(),
        }
    }
    
    pub fn get_data(&self) -> String {
        self.data.join("\n")
    }

    /// Save the data of the message to a file at the given path.
    /// Passes the message along to the next future.
    pub fn save_to_file<P>(self, path: P) -> impl Future<Item = Self, Error = io::Error>
    where
        P: AsRef<Path> + Send + 'static,
    {
        fs::write(path, self.get_data()).map(|_| self) // Ignore the string returned from the future. We only care about errors.
    }
}
