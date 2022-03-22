use crate::config::Node as NodeConfig;
use eth2::{
    types::{AggregateSignature, BeaconBlock, EthSpec, SignatureBytes, Slot},
    BeaconNodeHttpClient, Timeouts,
};
use sensitive_url::SensitiveUrl;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct Node {
    pub config: Arc<NodeConfig>,
    pub client: BeaconNodeHttpClient,
}

impl Node {
    pub fn new(config: Arc<NodeConfig>) -> Result<Self, String> {
        let url = SensitiveUrl::parse(&config.url).map_err(|e| format!("Invalid URL: {:?}", e))?;
        let client = BeaconNodeHttpClient::new(url, Timeouts::set_all(Duration::from_secs(6)));
        Ok(Self { config, client })
    }

    pub async fn get_block<E: EthSpec>(&self, slot: Slot) -> Result<BeaconBlock<E>, String> {
        if let Some(verify_randao) = self.config.verify_randao {
            self.get_block_with_verify_randao(slot, verify_randao).await
        } else {
            self.get_block_randao_oblivious(slot).await
        }
        .map_err(|e| format!("Error fetching block from {}: {:?}", self.config.url, e))
    }

    async fn get_block_with_verify_randao<E: EthSpec>(
        &self,
        slot: Slot,
        verify_randao: bool,
    ) -> Result<BeaconBlock<E>, eth2::Error> {
        self.client
            .get_validator_blocks_with_verify_randao(slot, None, None, Some(verify_randao))
            .await
            .map(|res| res.data)
    }

    async fn get_block_randao_oblivious<E: EthSpec>(
        &self,
        slot: Slot,
    ) -> Result<BeaconBlock<E>, eth2::Error> {
        let randao_reveal =
            SignatureBytes::deserialize(&AggregateSignature::infinity().serialize()).unwrap();
        self.client
            .get_validator_blocks::<E>(slot, &randao_reveal, None)
            .await
            .map(|res| res.data)
    }
}
