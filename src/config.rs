use serde::Deserialize;
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
    sync::Arc,
};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub network: String,
    pub canonical_bn: String,
    /// URL to push the dreamt blocks to (probably Lighthouse's `block_rewards` POST endpoint).
    pub post_endpoint: Option<String>,
    /// Directory to save post responses to.
    pub post_results_dir: Option<PathBuf>,
    pub nodes: Vec<Arc<Node>>,
}

#[derive(Debug, Deserialize)]
pub struct Node {
    pub name: String,
    pub label: String,
    pub url: String,
    #[serde(default)]
    pub verify_randao: Option<bool>,
}

impl Config {
    pub fn from_file(path: &Path) -> Result<Self, io::Error> {
        let mut f = File::open(path)?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        Ok(toml::from_str(&s).unwrap())
    }
}
