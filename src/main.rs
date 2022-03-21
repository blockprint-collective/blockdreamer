use config::Config;
use eth2::{
    types::{AggregateSignature, MainnetEthSpec, SignatureBytes, Slot},
    BeaconNodeHttpClient, Timeouts,
};
use futures::future::join_all;
use sensitive_url::SensitiveUrl;
use std::path::Path;
use std::time::Duration;

mod config;
mod distance;

type E = MainnetEthSpec;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    run().await.unwrap();
}

async fn run() -> Result<(), String> {
    // Load config.
    let config = Config::from_file(Path::new("config.toml")).unwrap();
    println!("{:#?}", config);

    let clients = config
        .nodes
        .into_iter()
        .map(|node| {
            let url =
                SensitiveUrl::parse(&node.url).map_err(|e| format!("Invalid URL: {:?}", e))?;
            let client = BeaconNodeHttpClient::new(url, Timeouts::set_all(Duration::from_secs(6)));
            Ok((node.name, client))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let slot = Slot::new(2518048);
    let handles = clients
        .iter()
        .map(move |(name, client)| {
            let randao_reveal =
                SignatureBytes::deserialize(&AggregateSignature::infinity().serialize()).unwrap();

            let client = client.clone();
            tokio::spawn(async move {
                client
                    .get_validator_blocks::<E>(slot, &randao_reveal, None)
                    .await
            })
        })
        .collect::<Vec<_>>();

    for result in join_all(handles).await {
        let block = result.map_err(|e| format!("Task panicked: {:?}", e))?;

        println!("{:#?}", block);
    }

    Ok(())
}
