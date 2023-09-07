use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author = "Blockprint Collective", version, long_about = None)]
#[command(about = "Ethereum block hallucinator.")]
pub struct CliConfig {
    /// Path to a TOML configuration file. See docs for examples
    #[arg(long, value_name = "PATH")]
    pub config: PathBuf,
}
