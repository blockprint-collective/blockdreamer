`blockdreamer`
=============

Blockdreamer is a microservice for triggering block production on Ethereum consensus clients.

Each slot it hits the `/eth/v2/validator/blocks/slot` endpoint for each node in its `config.toml`.

This is useful for several tasks:

- Benchmarking/stress testing
- Generating training data for [blockprint](https://github.com/sigp/blockprint)
- Computing similarity scores for blocks (an alternative to blockprint, see `src/distance.rs`)

## Installation

A Docker image is available on the GitHub container registry:

```
docker pull ghcr.io/blockprint-collective/blockdreamer
```

Or you can build from source:

```
cargo build --release
```

## Configuration

Blockdreamer is configured by a `config.toml` provided to the `--config` flag.

```
Ethereum block hallucinator.

Usage: blockdreamer --config <PATH>

Options:
      --config <PATH>  Path to a TOML configuration file. See docs for examples
  -h, --help           Print help
  -V, --version        Print version
```

An example configuration file can be found at [`example.toml`](./example.toml).

A full list of configuration options can be found in the source: [`src/config.rs`](./src/config.rs).
