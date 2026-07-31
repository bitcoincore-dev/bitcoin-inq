# P2TRC

`p2trc` is a small CLI for working with Bitcoin Core / Bitcoin Inquisition releases, local nodes, and chain detection.

## Commands

- `p2trc detect-network` — detect the active chain from an RPC connection or local cookie files.
- `p2trc node start` — start `bitcoind` or `bitcoin-qt` with chain-aware defaults.
- `p2trc node stop` — stop a local node over RPC.
- `p2trc node mine` — mine spendable **regtest** blocks to a wallet or supplied address.
- `p2trc inquisition` — list releases or install an Inquisition release asset.
- `p2trc kill` — terminate a process by name or PID.
- `p2trc list-processes` — print running processes, optionally filtered by name.

## Notes

- `node start` defaults to **regtest** because Inquisition builds reject unsupported networks like mainnet/testnet.
- `--chain signet` uses `.bitcoin/signet/bitcoin.conf` by default and accepts `--signetchallenge` for custom signets.
- `node mine` is regtest-only; signet still requires a real miner.
- `inquisition --install` accepts a release tag like `v29.4-inq` or `29.4-inq`. Omit the value to print the available releases.
- `inquisition --path` installs `bitcoin-qt-inq`, `bitcoind-inq`, and `bitcoin-cli-inq` into the first writable directory on `PATH`.

## Examples

```bash
cargo run -- node start --chain regtest
cargo run -- node start --chain signet --signetchallenge <hex>
cargo run -- node mine --blocks 101
cargo run -- inquisition --install v29.4-inq -f
cargo run -- detect-network
```
