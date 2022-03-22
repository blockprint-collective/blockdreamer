use config::Config;
use eth2::types::{MainnetEthSpec, Slot};
use futures::future::join_all;
use node::Node;
use std::path::Path;

mod config;
mod distance;
mod node;
mod tests;

type E = MainnetEthSpec;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    run().await.unwrap();
}

async fn run() -> Result<(), String> {
    // Load config.
    let config = Config::from_file(Path::new("config.toml")).unwrap();
    println!("{:#?}", config);

    let nodes = config
        .nodes
        .into_iter()
        .map(|config| Node::new(config))
        .collect::<Result<Vec<_>, String>>()?;

    let slot = Slot::new(3417538);
    let handles = nodes
        .iter()
        .map(move |node| {
            let inner = node.clone();
            tokio::spawn(async move { inner.get_block::<E>(slot).await })
        })
        .collect::<Vec<_>>();

    for result in join_all(handles).await {
        let block = result.map_err(|e| format!("Task panicked: {:?}", e))?;

        println!("{:#?}", block);
    }

    Ok(())
}
