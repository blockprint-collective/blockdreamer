use serde::Deserialize;
use std::path::PathBuf;
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
    sync::Arc,
};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub network: String,
    pub canonical_bn: String,
    /// URL to push the dreamt blocks to (probably Lighthouse's `block_rewards` POST endpoint).
    pub post_endpoint: Option<String>,
    /// Directory to save post responses to.
    pub post_results_dir: Option<PathBuf>,
    /// Whether to compare attestation rewards after POSTing to the endpoint.
    ///
    /// Assumes the `post_endpoint` is Lighthouse's `block_rewards` endpoint.
    #[serde(default)]
    pub compare_rewards: bool,
    /// Only post blocks if all endpoints return a block.
    #[serde(default = "default_true")]
    pub post_require_all: bool,
    /// Only post blocks if all blocks have the same parent.
    #[serde(default = "default_true")]
    pub post_require_same_parent: bool,
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

fn default_true() -> bool {
    true
}
