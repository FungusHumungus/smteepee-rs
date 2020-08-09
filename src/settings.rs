use std::error::Error;
use toml::de;
use serde_derive::Deserialize;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;



#[derive(Deserialize, PartialEq, Eq)]
pub enum Protocol {
    V4,
    V6,
}

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub port: u16,
    pub protocol: u8,
    pub domain: String,
    pub password: String,
}


impl Settings {
    
    /// Load the settings from the given Toml file.
    pub fn load<P>(filename: P) -> Result<Self, Box<dyn Error>> 
    where
        P: AsRef<Path>,
    {
        let mut file = File::open(filename)?;
        let mut data = String::new();
        file.read_to_string(&mut data)?;
        
        match de::from_str(&data) {
            Ok (settings) => Ok(settings),
            Err (err) => Err(Box::new(err)),
        }
    }
    
    /// Return a default set of settings for when no input file is given.
    pub fn default() -> Self {
        Settings {
            port: 2525,
            protocol: 4,
            domain: String::from("groove.com"),
            password: String::from("password"),
        }
    }
}
