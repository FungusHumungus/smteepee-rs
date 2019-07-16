use std::path::Path;
use tokio::fs;
use tokio::io;
use tokio::prelude::*;

#[derive(Debug, Clone)]
pub struct Message {
    pub from: Option<String>,
    pub to: Vec<String>,
    pub data: Vec<String>,
    pub saved: Option<String>,
}

impl Message {
    pub fn new() -> Self {
        Message {
            from: None,
            to: Vec::new(),
            data: Vec::new(),
            saved: None,
        }
    }
    
    pub fn get_data(&self) -> String {
        self.data.join("\n")
    }

    /// Save the data of the message to a file at the given path.
    /// Passes the message along to the next future.
    pub fn save_to_file<P>(mut self, path: P) -> impl Future<Item = Self, Error = io::Error>
    where
        P: AsRef<Path> + Send + Clone + 'static,
    {
        let path_str = path.as_ref().to_str().map(String::from);
        fs::write(path, self.get_data()).map(|_| {
            // Ignore the string returned from the future. We only care about errors.
            self.saved = path_str;
            self
        }) 
    }
}
