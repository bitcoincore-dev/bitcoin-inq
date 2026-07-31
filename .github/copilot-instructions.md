# Repository overview

This repository is a Bitcoin Script research and examples collection, with most of the content living in Markdown documents under `coins/bitcoin-scripts/`. The root Rust crate in `src/` is only a minimal scaffold.

`coins/bitcoin-scripts` and `mr-zwets/Composite-Arithmetic-Opcodes` are git submodules. Treat them as external upstream content unless you explicitly intend to update the submodule contents or pointer.

# Build, test, and lint

- Build: `cargo build`
- Test all: `cargo test`
- Run one test: `cargo test it_works`
- Format check: `cargo fmt --check`
- Lint: `cargo clippy`

# High-level architecture

- The repo is mostly a documentation corpus of Bitcoin Script techniques, transaction formats, and protocol ideas.
- Many files are standalone writeups with embedded script fragments, raw transaction hex, btcdeb sessions, images, and external references.
- `coins/bitcoin-scripts/` contains the main script examples and explanations.
- `mr-zwets/Composite-Arithmetic-Opcodes/` contains the CHIP/specification material and CashScript examples for composite arithmetic opcodes.
- `src/` is a placeholder Rust crate and is not the main product of the repo.

# Key conventions

- Prefer `bitcoin` and `bitcoincore-rpc` crate types and terminology when modeling networks, RPC auth, and chain selection.
- Preserve the existing Markdown style: short section headings, inline links, fenced code blocks, and explicit warning/disclaimer text.
- Keep Bitcoin Script examples and transaction hex verbatim unless the task explicitly requires changing them.
- Maintain the existing tone of the docs: technical, example-driven, and focused on concrete script behavior.
- Keep relative links, image references, and file names consistent with the current lowercase, hyphenated naming style.
- If you edit content inside a submodule, stay within that submodule’s structure and conventions.
