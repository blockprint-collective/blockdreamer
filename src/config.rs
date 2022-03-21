use serde::Deserialize;
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Deserialize)]
pub struct Node {
    pub name: String,
    pub url: String,
}

impl Config {
    pub fn from_file(path: &Path) -> Result<Self, io::Error> {
        let mut f = File::open(path)?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        Ok(toml::from_str(&s).unwrap())
    }
}
