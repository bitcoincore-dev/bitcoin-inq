# Bitcoin Inq

`bitcoin-inq` is a small CLI for working with Bitcoin Core / Bitcoin Inquisition releases, local nodes, and chain detection.

## Commands

- `bitcoin-inq detect-network` — detect the active chain from an RPC connection or local cookie files.
- `bitcoin-inq node start` — start `bitcoind` or `bitcoin-qt` with chain-aware defaults.
- `bitcoin-inq node stop` — stop a local node over RPC.
- `bitcoin-inq node mine` — mine spendable **regtest** blocks to a wallet or supplied address.
- `bitcoin-inq inquisition` — list releases or install an Inquisition release asset.
- `bitcoin-inq kill` — terminate a process by name or PID.
- `bitcoin-inq list-processes` — print running processes, optionally filtered by name.

## Notes

- `node start` defaults to **regtest** because Inquisition builds reject unsupported networks like mainnet/testnet.
- `--chain signet` uses `.bitcoin/signet/bitcoin.conf` by default and accepts `--signetchallenge` for custom signets.
- `node mine` is regtest-only; signet still requires a real miner.
- `inquisition --install` accepts a release tag like `v29.4-inq` or `29.4-inq`. Omit the value to print the available releases.
- `inquisition --path` installs `bitcoin-qt-inq`, `bitcoind-inq`, and `bitcoin-cli-inq` into the first writable directory on `PATH`.

## Release flow

This repo uses `cargo-dist` for tagged releases (`v*.*.*`):

1. `plan` runs `dist host --steps=create` to compute the release manifest and matrix.
2. `build-local-artifacts` builds per-platform archives and updater assets.
3. `macos-signing` signs and notarizes the macOS binary, then repacks the macOS `.tar.xz` so the packaged binary stays signed too.
4. `build-global-artifacts` finishes the cross-platform assets and checksums.
5. `host` runs `dist host --steps=upload --steps=release` and publishes the GitHub Release.
6. `publish-homebrew-formula` updates the tap when the release is eligible.

Published release assets are the packaged archives, checksums, update bundles, and installer scripts. The release job edits and re-uploads on reruns, so it is safe to retry a failed release tag without manually deleting the release first.

## Examples

```bash
cargo run -- node start --chain regtest
cargo run -- node start --chain signet --signetchallenge <hex>
cargo run -- node mine --blocks 101
cargo run -- inquisition --install v29.4-inq -f
cargo run -- detect-network
```
