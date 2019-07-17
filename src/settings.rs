use std::error;
use toml::de;
use serde_derive::Deserialize;
use std::fs;
use std::io::prelude::*;
use std::path::Path;



#[derive(Deserialize, PartialEq, Eq)]
pub enum Protocol {
    V4,
    V6,
}

#[derive(Deserialize)]
pub struct Settings {
    pub port: u16,
    pub protocol: u8,
}

impl Settings {
    
    pub fn load<P>(filename: P) -> Result<Self, Box<dyn error::Error>> 
    where
        P: AsRef<Path>,
    {
        let mut file = fs::File::open(filename)?;
        let mut data = String::new();
        file.read_to_string(&mut data)?;
        
        println!("Deserializig {}", data);
        
        match de::from_str(&data) {
            Ok (settings) => Ok(settings),
            Err (err) => Err(Box::new(err)),
        }
    }
    
    pub fn default() -> Self {
        Settings {
            port: 25,
            protocol: 4
        }
    }
}
