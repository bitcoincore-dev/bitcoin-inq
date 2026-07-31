use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SignalKind {
    Term,
    Kill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ChainSelection {
    Mainnet,
    Testnet,
    Testnet4,
    Signet,
    Regtest,
}

impl ChainSelection {
    pub fn network(self) -> bitcoin::Network {
        match self {
            Self::Mainnet => bitcoin::Network::Bitcoin,
            Self::Testnet => bitcoin::Network::Testnet,
            Self::Testnet4 => bitcoin::Network::Testnet4,
            Self::Signet => bitcoin::Network::Signet,
            Self::Regtest => bitcoin::Network::Regtest,
        }
    }
}

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start a local Bitcoin Core node.
    Node(NodeArgs),
    /// Detect the chain the local Bitcoin Core node is running on.
    DetectNetwork(RpcArgs),
    /// Kill a process by PID.
    Kill(ProcessArgs),
}

#[derive(Debug, Subcommand)]
pub enum NodeCommands {
    /// Start bitcoind with a selected chain.
    Start(NodeStartArgs),
}

#[derive(Debug, Args)]
pub struct NodeArgs {
    #[command(subcommand)]
    pub command: NodeCommands,
}

#[derive(Debug, Args, Clone)]
pub struct RpcArgs {
    /// RPC endpoint, for example http://127.0.0.1:8332
    #[arg(long)]
    pub rpc_url: Option<String>,

    /// RPC username for user/pass auth.
    #[arg(long)]
    pub rpc_user: Option<String>,

    /// RPC password for user/pass auth.
    #[arg(long)]
    pub rpc_password: Option<String>,

    /// Path to the cookie file for cookie auth.
    #[arg(long)]
    pub cookie_file: Option<PathBuf>,

    /// Optional signet challenge marker to distinguish a custom signet chain.
    #[arg(long)]
    pub signet_challenge: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct NodeStartArgs {
    /// Bitcoin Core chain to start.
    #[arg(long, value_enum, default_value_t = ChainSelection::Testnet)]
    pub chain: ChainSelection,

    /// Signet challenge for custom signet networks.
    #[arg(long)]
    pub signetchallenge: Option<String>,

    /// Data directory for bitcoind.
    #[arg(long)]
    pub datadir: Option<PathBuf>,

    /// Run in the foreground instead of daemonizing.
    #[arg(long, default_value_t = false)]
    pub foreground: bool,

    /// Print the command instead of executing it.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args, Clone)]
pub struct ProcessArgs {
    /// Process ID to terminate.
    #[arg(long)]
    pub pid: u32,

    /// Send SIGKILL instead of SIGTERM.
    #[arg(long, default_value_t = false)]
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkKind {
    Mainnet,
    Testnet,
    Testnet4,
    Signet,
    CustomSignet,
    Regtest,
    Unknown(String),
}

impl std::fmt::Display for NetworkKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mainnet => f.write_str("mainnet"),
            Self::Testnet => f.write_str("testnet"),
            Self::Testnet4 => f.write_str("testnet4"),
            Self::Signet => f.write_str("signet"),
            Self::CustomSignet => f.write_str("custom signet"),
            Self::Regtest => f.write_str("regtest"),
            Self::Unknown(chain) => write!(f, "unknown ({chain})"),
        }
    }
}

pub fn rpc_auth(args: &RpcArgs) -> Result<Auth> {
    if let Some(cookie_file) = &args.cookie_file {
        return Ok(Auth::CookieFile(cookie_file.clone()));
    }

    match (&args.rpc_user, &args.rpc_password) {
        (Some(user), Some(password)) => Ok(Auth::UserPass(user.clone(), password.clone())),
        _ => Err(anyhow::anyhow!(
            "provide --cookie-file, both --rpc-user and --rpc-password, or use auto-detect"
        )),
    }
}

fn default_cookie_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        let bitcoin = home.join(".bitcoin");
        paths.extend([
            bitcoin.join(".cookie"),
            bitcoin.join("testnet3").join(".cookie"),
            bitcoin.join("testnet4").join(".cookie"),
            bitcoin.join("signet").join(".cookie"),
            bitcoin.join("regtest").join(".cookie"),
        ]);

        if cfg!(target_os = "macos") {
            let app_support = home.join("Library").join("Application Support").join("Bitcoin");
            paths.extend([
                app_support.join(".cookie"),
                app_support.join("testnet3").join(".cookie"),
                app_support.join("testnet4").join(".cookie"),
                app_support.join("signet").join(".cookie"),
                app_support.join("regtest").join(".cookie"),
            ]);
        }
    }

    paths
}

fn default_rpc_urls() -> Vec<String> {
    [
        "http://127.0.0.1:8332",
        "http://127.0.0.1:18332",
        "http://127.0.0.1:38332",
        "http://127.0.0.1:18443",
        "http://127.0.0.1:48332",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

pub fn detect_network(client: &Client) -> Result<NetworkKind> { detect_network_with_override(client, None) }

pub fn detect_network_with_override(client: &Client, signet_challenge: Option<&str>) -> Result<NetworkKind> {
    let info = client.get_blockchain_info().context("failed to query blockchain info")?;
    let network = match info.chain {
        bitcoin::Network::Bitcoin => NetworkKind::Mainnet,
        bitcoin::Network::Testnet => NetworkKind::Testnet,
        bitcoin::Network::Testnet4 => NetworkKind::Testnet4,
        bitcoin::Network::Signet => {
            if signet_challenge.is_some() {
                NetworkKind::CustomSignet
            } else {
                NetworkKind::Signet
            }
        }
        bitcoin::Network::Regtest => NetworkKind::Regtest,
    };

    Ok(network)
}

pub fn kill_process(pid: u32, force: bool) -> Result<()> {
    let status = if cfg!(windows) {
        let mut cmd = Command::new("taskkill");
        cmd.arg("/PID").arg(pid.to_string());
        if force {
            cmd.arg("/F");
        }
        cmd.status().context("failed to invoke taskkill")?
    } else {
        let signal = if force { "-9" } else { "-15" };
        Command::new("kill")
            .arg(signal)
            .arg(pid.to_string())
            .status()
            .context("failed to invoke kill")?
    };

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("failed to terminate process {pid}"))
    }
}

pub fn bitcoind_binary() -> Result<PathBuf> {
    find_executable(&["bitcoind", "bitcoind.exe", "Bitcoin-Qt"])
        .ok_or_else(|| anyhow::anyhow!("could not find bitcoind on PATH"))
}

pub fn bitcoin_cli_binary() -> Result<PathBuf> {
    find_executable(&["bitcoin-cli", "bitcoin-cli.exe"])
        .ok_or_else(|| anyhow::anyhow!("could not find bitcoin-cli on PATH"))
}

fn find_executable(candidates: &[&str]) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let paths = std::env::split_paths(&path);

    for dir in paths {
        for candidate in candidates {
            let full = dir.join(candidate);
            if is_executable(&full) {
                return Some(full);
            }
        }
    }

    None
}

fn is_executable(path: &Path) -> bool {
    path.is_file()
}

pub fn start_node(args: NodeStartArgs) -> Result<()> {
    if args.chain != ChainSelection::Signet && args.signetchallenge.is_some() {
        return Err(anyhow::anyhow!(
            "--signetchallenge is only valid with --chain=signet"
        ));
    }

    let bitcoind = bitcoind_binary()?;
    let mut command = Command::new(&bitcoind);
    command.arg(format!("-chain={}", args.chain.network().to_core_arg()));

    if let Some(datadir) = &args.datadir {
        command.arg(format!("-datadir={}", datadir.display()));
    }

    if let Some(challenge) = &args.signetchallenge {
        command.arg(format!("-signetchallenge={challenge}"));
    }

    if !args.foreground {
        command.arg("-daemon");
    }

    if args.dry_run {
        println!(
            "{} {}",
            bitcoind.display(),
            command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(" ")
        );
        return Ok(());
    }

    let status = command.status().context("failed to invoke bitcoind")?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("bitcoind exited with a non-zero status"))
    }
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Node(node) => match node.command {
            NodeCommands::Start(args) => start_node(args),
        },
        Commands::DetectNetwork(args) => {
            let network = if let Some(rpc_url) = &args.rpc_url {
                let client = Client::new(rpc_url, rpc_auth(&args)?)
                    .with_context(|| format!("failed to connect to {rpc_url}"))?;
                detect_network_with_override(&client, args.signet_challenge.as_deref())?
            } else {
                detect_network_auto(args.signet_challenge.as_deref())?
            };
            println!("{network}");
            Ok(())
        }
        Commands::Kill(args) => kill_process(args.pid, args.force),
    }
}

pub fn detect_network_auto(signet_challenge: Option<&str>) -> Result<NetworkKind> {
    let cookies = default_cookie_paths();
    let urls = default_rpc_urls();

    for rpc_url in urls {
        for cookie in &cookies {
            if !cookie.exists() {
                continue;
            }

            if let Ok(client) = Client::new(&rpc_url, Auth::CookieFile(cookie.clone())) {
                if let Ok(network) = detect_network_with_override(&client, signet_challenge) {
                    return Ok(network);
                }
            }
        }
    }

    Err(anyhow::anyhow!(
        "could not auto-detect a local Bitcoin Core node; pass --rpc-url and auth flags explicitly"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_network_names() {
        assert_eq!(NetworkKind::Mainnet.to_string(), "mainnet");
        assert_eq!(NetworkKind::CustomSignet.to_string(), "custom signet");
    }
}
