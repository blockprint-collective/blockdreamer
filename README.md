`blockdreamer`
=============

Blockdreamer is a microservice for triggering block production on Ethereum consensus clients.

Each slot it hits the `/eth/v2/validator/blocks/slot` endpoint for each node in its `config.toml`.

This is useful for several tasks:

- Benchmarking/stress testing
- Generating training data for [blockprint](https://github.com/sigp/blockprint)
- Computing similarity scores for blocks (an alternative to blockprint, see `src/distance.rs`)
