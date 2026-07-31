use clap::Parser;
use bitcoin_inq::{Cli, run};

fn main() -> anyhow::Result<()> {
    run(Cli::parse())
}
