use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Deserialize;

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
    /// Install Bitcoin Inquisition release assets.
    Inquisition(InquisitionArgs),
    /// List running processes.
    ListProcesses(ProcessListArgs),
    /// Detect the chain the local Bitcoin Core node is running on.
    DetectNetwork(RpcArgs),
    /// Kill a process by PID.
    Kill(ProcessArgs),
}

#[derive(Debug, Subcommand)]
pub enum NodeCommands {
    /// Start bitcoind with a selected chain.
    Start(NodeStartArgs),
    /// Stop a running Bitcoin Core node.
    Stop(RpcArgs),
}

#[derive(Debug, Args)]
pub struct NodeArgs {
    #[command(subcommand)]
    pub command: NodeCommands,
}

#[derive(Debug, Args)]
pub struct InquisitionArgs {
    /// Install the given release tag or version (for example v29.4-inq or 29.4-inq).
    #[arg(long, num_args = 0..=1, default_missing_value = "empty")]
    pub install: Option<String>,

    /// Overwrite an existing downloaded asset.
    #[arg(short = 'f', long)]
    pub force: bool,

    /// Destination directory for the downloaded release asset.
    #[arg(long, default_value = ".")]
    pub dir: PathBuf,

    /// Print the chosen release and asset instead of downloading.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
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
    #[arg(long, value_enum, default_value_t = ChainSelection::Testnet4)]
    pub chain: ChainSelection,

    /// Bitcoin Core config file.
    #[arg(long, default_value = "bitcoin.conf")]
    pub conf: PathBuf,

    /// Signet challenge for custom signet networks.
    #[arg(long)]
    pub signetchallenge: Option<String>,

    /// Data directory for bitcoind.
    #[arg(long, default_value = ".bitcoin")]
    pub datadir: PathBuf,

    /// Run in the foreground instead of daemonizing.
    #[arg(long, default_value_t = false)]
    pub foreground: bool,

    /// Print the command instead of executing it.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args, Clone)]
pub struct ProcessArgs {
    /// Process name to terminate.
    pub name: Option<String>,

    /// Process ID to terminate.
    #[arg(long)]
    pub pid: Option<u32>,

    /// Send SIGKILL instead of SIGTERM.
    #[arg(long, default_value_t = false)]
    pub force: bool,
}

#[derive(Debug, Args, Clone)]
pub struct ProcessListArgs {
    /// Filter by process name.
    #[arg(long)]
    pub name: Option<String>,

    /// Match the process name as a substring instead of exact basename.
    #[arg(long, default_value_t = false)]
    pub contains: bool,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
}

fn process_name_matches(name: &str, filter: &str, contains: bool) -> bool {
    let name = name.to_ascii_lowercase();
    let filter = filter.to_ascii_lowercase();
    if contains { name.contains(&filter) } else { name == filter }
}

fn process_name_from_path(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

#[cfg(not(windows))]
fn list_processes_os() -> Result<Vec<ProcessInfo>> {
    let output = Command::new("ps")
        .args(["-A", "-o", "pid=", "-o", "comm="])
        .output()
        .context("failed to invoke ps")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("ps exited with a non-zero status"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut processes = Vec::new();

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.split_whitespace();
        let pid = match parts.next().and_then(|pid| pid.parse::<u32>().ok()) {
            Some(pid) => pid,
            None => continue,
        };
        let name = parts.next().map(process_name_from_path).unwrap_or(trimmed).to_string();
        processes.push(ProcessInfo { pid, name });
    }

    Ok(processes)
}

#[cfg(windows)]
fn list_processes_os() -> Result<Vec<ProcessInfo>> {
    let output = Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output()
        .context("failed to invoke tasklist")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("tasklist exited with a non-zero status"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut processes = Vec::new();

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("INFO:") {
            continue;
        }

        let cols: Vec<&str> = trimmed.trim_matches('"').split("\",\"").collect();
        if cols.len() < 2 {
            continue;
        }

        let name = cols[0].to_string();
        let pid = match cols[1].replace(',', "").parse::<u32>() {
            Ok(pid) => pid,
            Err(_) => continue,
        };

        processes.push(ProcessInfo { pid, name });
    }

    Ok(processes)
}

pub fn list_processes(name: Option<&str>, contains: bool) -> Result<Vec<ProcessInfo>> {
    let processes = list_processes_os()?;
    Ok(match name {
        Some(filter) => processes
            .into_iter()
            .filter(|process| process_name_matches(&process.name, filter, contains))
            .collect(),
        None => processes,
    })
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

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    #[serde(default)]
    is_latest: bool,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

fn gh_exists() -> bool {
    find_executable(&["gh"]).is_some()
}

fn fetch_release_list() -> Result<Vec<GitHubRelease>> {
    let output = if gh_exists() {
        Command::new("gh")
            .args(["api", "repos/bitcoin-inquisition/bitcoin/releases?per_page=100"])
            .output()
            .context("failed to invoke gh")?
    } else {
        Command::new("curl")
            .args([
                "-fsSL",
                "https://api.github.com/repos/bitcoin-inquisition/bitcoin/releases?per_page=100",
            ])
            .output()
            .context("failed to invoke curl")?
    };

    if !output.status.success() {
        return Err(anyhow::anyhow!("failed to fetch Inquisition release list"));
    }

    Ok(serde_json::from_slice(&output.stdout).context("failed to parse release list")?)
}

fn normalize_tag(tag: &str) -> String {
    tag.strip_prefix('v').unwrap_or(tag).to_owned()
}

fn is_version_list_request(version: Option<&str>) -> bool {
    matches!(version.map(str::trim), Some("") | Some("empty"))
}

fn resolve_release_tag(requested: Option<&str>, releases: &[GitHubRelease]) -> Result<String> {
    if let Some(requested) = requested {
        if requested == "latest" {
            return releases
                .iter()
                .find(|release| release.is_latest)
                .map(|release| release.tag_name.clone())
                .ok_or_else(|| anyhow::anyhow!("could not determine latest Inquisition release"));
        }

        if let Some(release) = releases.iter().find(|release| {
            release.tag_name == requested
                || normalize_tag(&release.tag_name) == requested
                || release.tag_name == format!("v{requested}")
        }) {
            return Ok(release.tag_name.clone());
        }

        return Err(anyhow::anyhow!(
            "unknown Inquisition release tag: {requested}"
        ));
    }

    releases
        .iter()
        .find(|release| release.is_latest)
        .map(|release| release.tag_name.clone())
        .ok_or_else(|| anyhow::anyhow!("could not determine latest Inquisition release"))
}

fn inquisition_asset_name(tag: &str) -> Result<String> {
    let version = normalize_tag(tag);
    let suffix = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "x86_64-linux-gnu.tar.gz",
        ("linux", "aarch64") => "aarch64-linux-gnu.tar.gz",
        ("macos", "x86_64") => "x86_64-apple-darwin-unsigned.tar.gz",
        ("macos", "aarch64") | ("macos", "arm64") => "arm64-apple-darwin-unsigned.tar.gz",
        ("windows", "x86_64") => "win64-codesigning.tar.gz",
        _ => return Err(anyhow::anyhow!("unsupported platform for Inquisition release assets")),
    };

    Ok(format!("bitcoin-{version}-{suffix}"))
}

fn gh_release_download(tag: &str, asset_name: &str, dir: &Path) -> Result<PathBuf> {
    let status = Command::new("gh")
        .args([
            "release",
            "download",
            tag,
            "--repo",
            "bitcoin-inquisition/bitcoin",
            "--pattern",
            asset_name,
            "--dir",
        ])
        .arg(dir)
        .status()
        .context("failed to invoke gh release download")?;

    if !status.success() {
        return Err(anyhow::anyhow!("gh release download failed"));
    }

    Ok(dir.join(asset_name))
}

fn curl_download(url: &str, file: &Path) -> Result<()> {
    let status = Command::new("curl")
        .args(["-fL", url, "-o"])
        .arg(file)
        .status()
        .context("failed to invoke curl")?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("curl download failed"))
    }
}

pub fn install_inquisition(version: Option<&str>, dir: &Path, force: bool, dry_run: bool) -> Result<PathBuf> {
    let releases = fetch_release_list()?;

    if is_version_list_request(version) {
        for release in &releases {
            println!("{}", release.tag_name);
        }
        return Ok(dir.to_path_buf());
    }

    let tag = resolve_release_tag(version, &releases)?;
    let asset_name = inquisition_asset_name(&tag)?;
    let target_dir = dir.to_path_buf();

    let release = releases
        .iter()
        .find(|release| release.tag_name == tag)
        .ok_or_else(|| anyhow::anyhow!("release metadata missing for {tag}"))?;

    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .ok_or_else(|| anyhow::anyhow!("no matching asset found for {asset_name}"))?;
    let target_file = target_dir.join(&asset.name);

    if dry_run {
        println!("{tag} {}", asset.name);
        return Ok(target_file);
    }

    std::fs::create_dir_all(&target_dir).context("failed to create install directory")?;
    if force && target_file.exists() {
        std::fs::remove_file(&target_file).context("failed to remove existing asset")?;
    }

    if gh_exists() {
        gh_release_download(&tag, &asset.name, &target_dir)
    } else {
        curl_download(&asset.browser_download_url, &target_file)?;
        Ok(target_file)
    }
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
    command.arg(format!("-conf={}", args.conf.display()));
    command.arg(format!("-datadir={}", args.datadir.display()));

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

pub fn stop_node(args: RpcArgs) -> Result<()> {
    let bitcoin_cli = bitcoin_cli_binary()?;

    let mut command = Command::new(bitcoin_cli);
    command.arg("stop");

    if let Some(rpc_url) = &args.rpc_url {
        command.arg(format!("-rpcconnect={rpc_url}"));
    }

    if let Some(cookie_file) = &args.cookie_file {
        command.arg(format!("-rpccookiefile={}", cookie_file.display()));
    }

    if let Some(user) = &args.rpc_user {
        command.arg(format!("-rpcuser={user}"));
    }

    if let Some(password) = &args.rpc_password {
        command.arg(format!("-rpcpassword={password}"));
    }

    if let Some(challenge) = &args.signet_challenge {
        command.arg(format!("-signetchallenge={challenge}"));
    }

    let status = command.status().context("failed to invoke bitcoin-cli")?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("bitcoin-cli exited with a non-zero status"))
    }
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Node(node) => match node.command {
            NodeCommands::Start(args) => start_node(args),
            NodeCommands::Stop(args) => stop_node(args),
        },
        Commands::ListProcesses(args) => {
            for process in list_processes(args.name.as_deref(), args.contains)? {
                println!("{}\t{}", process.pid, process.name);
            }
            Ok(())
        }
        Commands::Inquisition(args) => {
            let dry_run = args.dry_run;
            let path = install_inquisition(args.install.as_deref(), &args.dir, args.force, dry_run)?;
            if !dry_run && !is_version_list_request(args.install.as_deref()) {
                println!("{}", path.display());
            }
            Ok(())
        }
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
        Commands::Kill(args) => match (args.name.as_deref(), args.pid) {
            (Some(name), _) => {
                let processes = list_processes(Some(name), false)?;
                if processes.is_empty() {
                    return Err(anyhow::anyhow!("no processes matched {name}"));
                }

                for process in processes {
                    kill_process(process.pid, args.force)?;
                }

                Ok(())
            }
            (None, Some(pid)) => kill_process(pid, args.force),
            (None, None) => {
                for process in list_processes(None, false)? {
                    println!("{}\t{}", process.pid, process.name);
                }
                Ok(())
            }
        },
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

    #[test]
    fn normalizes_release_tags() {
        assert_eq!(normalize_tag("v29.4-inq"), "29.4-inq");
        assert_eq!(normalize_tag("29.4-inq"), "29.4-inq");
    }

    #[test]
    fn prints_available_releases() {
        let releases = fetch_release_list().expect("release list");
        for release in &releases {
            println!("{}", release.tag_name);
        }
        assert!(!releases.is_empty());
    }

    #[test]
    fn matches_process_names() {
        assert!(process_name_matches("bitcoind", "bitcoind", false));
        assert!(process_name_matches("bitcoind", "bitcoin", true));
        assert_eq!(process_name_from_path("/usr/local/bin/bitcoind"), "bitcoind");
    }
}
