use std::path::Path;
use tokio::fs;
use tokio::io;

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
    pub async fn save_to_file<P>(self, path: P) -> io::Result<()>
    where
        P: AsRef<Path> + Send + Clone + 'static,
    {
        fs::write(path, self.get_data()).await
    }
    
}
