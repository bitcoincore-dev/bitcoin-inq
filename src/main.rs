use p2trc::{run, Cli};

fn main() -> anyhow::Result<()> {
    run(Cli::parse())
}
