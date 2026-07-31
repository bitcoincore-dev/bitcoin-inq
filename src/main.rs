use clap::Parser;
use p2trc::{Cli, run};

fn main() -> anyhow::Result<()> {
    run(Cli::parse())
}
