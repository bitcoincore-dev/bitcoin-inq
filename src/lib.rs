use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SignalKind {
    Term,
    Kill,
}

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Detect the chain the local Bitcoin Core node is running on.
    DetectNetwork(RpcArgs),
    /// Kill a process by PID.
    Kill(ProcessArgs),
}

#[derive(Debug, Args, Clone)]
pub struct RpcArgs {
    /// RPC endpoint, for example http://127.0.0.1:8332
    #[arg(long, default_value = "http://127.0.0.1:8332")]
    pub rpc_url: String,

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
            "provide either --cookie-file or both --rpc-user and --rpc-password"
        )),
    }
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

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::DetectNetwork(args) => {
            let client = Client::new(&args.rpc_url, rpc_auth(&args)?)
                .with_context(|| format!("failed to connect to {}", args.rpc_url))?;
            let network = detect_network_with_override(&client, args.signet_challenge.as_deref())?;
            println!("{network}");
            Ok(())
        }
        Commands::Kill(args) => kill_process(args.pid, args.force),
    }
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
