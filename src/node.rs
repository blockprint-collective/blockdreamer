use crate::config::Node as NodeConfig;
use eth2::{
    types::{
        BlindedPayload, BlockContents, ChainSpec, EthSpec,
        ForkVersionedBeaconBlockType, FullPayload, Signature, SignatureBytes,
        SkipRandaoVerification, Slot,
    },
    BeaconNodeHttpClient, Timeouts,
};
use sensitive_url::SensitiveUrl;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct Node {
    pub config: Arc<NodeConfig>,
    pub client: BeaconNodeHttpClient,
    pub spec: Arc<ChainSpec>,
}

impl Node {
    pub fn new(config: Arc<NodeConfig>, spec: Arc<ChainSpec>) -> Result<Self, String> {
        let url = SensitiveUrl::parse(&config.url).map_err(|e| format!("Invalid URL: {:?}", e))?;
        let client = BeaconNodeHttpClient::new(url, Timeouts::set_all(Duration::from_secs(6)));
        Ok(Self {
            config,
            client,
            spec,
        })
    }

    pub async fn get_block_v3<E: EthSpec>(
        &self,
        slot: Slot,
    ) -> Result<ForkVersionedBeaconBlockType<E>, String> {
        let randao_reveal = Signature::infinity().unwrap().into();
        let skip_randao_verification = if self.config.skip_randao_verification {
            SkipRandaoVerification::Yes
        } else {
            SkipRandaoVerification::No
        };
        /*
        if self.config.ssz {
            todo!()
        } else {
            self.get_block_v3_json(slot, &randao_reveal, skip_randao_verification)
                .await
        }
        */
        self.get_block_v3_json::<E>(slot, &randao_reveal, skip_randao_verification)
            .await
    }

    pub async fn get_block_v3_json<E: EthSpec>(
        &self,
        slot: Slot,
        randao_reveal: &SignatureBytes,
        skip_randao_verification: SkipRandaoVerification,
    ) -> Result<ForkVersionedBeaconBlockType<E>, String> {
        self.client
            .get_validator_blocks_v3_modular::<E>(
                slot,
                randao_reveal,
                None,
                skip_randao_verification,
            )
            .await
            .map_err(|e| format!("Error fetching block from {}: {:?}", self.config.url, e))
    }

    pub async fn get_block<E: EthSpec>(
        &self,
        slot: Slot,
    ) -> Result<BlockContents<E, FullPayload<E>>, String> {
        let randao_reveal = Signature::infinity().unwrap().into();
        let skip_randao_verification = if self.config.skip_randao_verification {
            SkipRandaoVerification::Yes
        } else {
            SkipRandaoVerification::No
        };
        if self.config.ssz {
            self.get_block_ssz(slot, &randao_reveal, skip_randao_verification)
                .await
        } else {
            self.get_block_json(slot, &randao_reveal, skip_randao_verification)
                .await
        }
    }

    pub async fn get_block_json<E: EthSpec>(
        &self,
        slot: Slot,
        randao_reveal: &SignatureBytes,
        skip_randao_verification: SkipRandaoVerification,
    ) -> Result<BlockContents<E, FullPayload<E>>, String> {
        self.client
            .get_validator_blocks_modular::<E, _>(
                slot,
                randao_reveal,
                None,
                skip_randao_verification,
            )
            .await
            .map(|res| res.data)
            .map_err(|e| format!("Error fetching block from {}: {:?}", self.config.url, e))
    }

    pub async fn get_block_ssz<E: EthSpec>(
        &self,
        slot: Slot,
        randao_reveal: &SignatureBytes,
        skip_randao_verification: SkipRandaoVerification,
    ) -> Result<BlockContents<E, FullPayload<E>>, String> {
        let bytes = self
            .client
            .get_validator_blocks_modular_ssz::<E, FullPayload<E>>(
                slot,
                randao_reveal,
                None,
                skip_randao_verification,
            )
            .await
            .map_err(|e| format!("Error fetching block from {}: {:?}", self.config.url, e))?
            .ok_or_else(|| {
                format!(
                    "Error fetching block from {}: returned 404",
                    self.config.url
                )
            })?;
        BlockContents::from_ssz_bytes(&bytes, &self.spec)
            .map_err(|e| format!("Error fetching block from {}: {e:?}", self.config.url))
    }

    pub async fn get_block_v3_with_timeout<E: EthSpec>(
        &self,
        slot: Slot,
    ) -> Result<ForkVersionedBeaconBlockType<E>, String> {
        tokio::time::timeout(Duration::from_secs(6), self.get_block_v3(slot))
            .await
            .map_err(|_| format!("request to {} timed out after 6s", self.config.name))?
    }

    pub async fn get_block_with_timeout<E: EthSpec>(
        &self,
        slot: Slot,
    ) -> Result<BlockContents<E, FullPayload<E>>, String> {
        tokio::time::timeout(Duration::from_secs(6), self.get_block(slot))
            .await
            .map_err(|_| format!("request to {} timed out after 6s", self.config.name))?
    }

    pub async fn get_blinded_block<E: EthSpec>(
        &self,
        slot: Slot,
    ) -> Result<BlockContents<E, BlindedPayload<E>>, String> {
        let randao_reveal = Signature::infinity().unwrap().into();
        let skip_randao_verification = if self.config.skip_randao_verification {
            SkipRandaoVerification::Yes
        } else {
            SkipRandaoVerification::No
        };
        if self.config.ssz {
            self.get_blinded_block_ssz(slot, &randao_reveal, skip_randao_verification)
                .await
        } else {
            self.get_blinded_block_json(slot, &randao_reveal, skip_randao_verification)
                .await
        }
    }

    pub async fn get_blinded_block_json<E: EthSpec>(
        &self,
        slot: Slot,
        randao_reveal: &SignatureBytes,
        skip_randao_verification: SkipRandaoVerification,
    ) -> Result<BlockContents<E, BlindedPayload<E>>, String> {
        self.client
            .get_validator_blinded_blocks_modular::<E, _>(
                slot,
                randao_reveal,
                None,
                skip_randao_verification,
            )
            .await
            .map(|res| res.data)
            .map_err(|e| format!("Error fetching block from {}: {:?}", self.config.url, e))
    }

    pub async fn get_blinded_block_ssz<E: EthSpec>(
        &self,
        slot: Slot,
        randao_reveal: &SignatureBytes,
        skip_randao_verification: SkipRandaoVerification,
    ) -> Result<BlockContents<E, BlindedPayload<E>>, String> {
        let bytes = self
            .client
            .get_validator_blinded_blocks_modular_ssz::<E, BlindedPayload<E>>(
                slot,
                randao_reveal,
                None,
                skip_randao_verification,
            )
            .await
            .map_err(|e| format!("Error fetching block from {}: {:?}", self.config.url, e))?
            .ok_or_else(|| {
                format!(
                    "Error fetching block from {}: returned 404",
                    self.config.url
                )
            })?;
        BlockContents::from_ssz_bytes(&bytes, &self.spec)
            .map_err(|e| format!("Error fetching block from {}: {e:?}", self.config.url))
    }

    pub async fn get_blinded_block_with_timeout<E: EthSpec>(
        &self,
        slot: Slot,
    ) -> Result<BlockContents<E, BlindedPayload<E>>, String> {
        tokio::time::timeout(Duration::from_secs(6), self.get_blinded_block(slot))
            .await
            .map_err(|_| format!("request to {} timed out after 6s", self.config.name))?
    }
}
