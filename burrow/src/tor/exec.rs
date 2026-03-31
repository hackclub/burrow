use std::{
    ffi::{OsStr, OsString},
    fs,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    os::unix::process::ExitStatusExt,
    path::PathBuf,
    process::{Command, ExitStatus, Stdio},
    sync::Arc,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use tokio::process::Command as TokioCommand;
use tor_rtcompat::PreferredRuntime;
use tracing::{debug, info};

use super::{
    bootstrap_client,
    dns::{spawn as spawn_dns, TorDnsHandle},
    runtime::{spawn_with_client, TorHandle},
    Config, SystemTcpStackConfig, TcpStackConfig,
};

const CHILD_PREFIX_LEN: u8 = 30;
const CHILD_DNS_PORT: u16 = 53;
const LISTENER_READY_TIMEOUT: Duration = Duration::from_secs(10);
const LISTENER_READY_POLL: Duration = Duration::from_millis(100);

pub async fn run_exec(mut config: Config, command: Vec<String>) -> Result<i32> {
    if command.is_empty() {
        bail!("tor-exec requires a command to run");
    }
    ensure_root()?;
    ensure_host_tool("ip")?;
    ensure_host_tool("iptables")?;
    ensure_host_tool("unshare")?;

    let requested_listener = config.listen_addr()?;
    if requested_listener.port() == 0 {
        bail!("tor-exec requires a fixed listener port");
    }

    let plan = NamespacePlan::new(requested_listener.port());
    let (state_dir, cache_dir) = config.runtime_dirs(std::process::id() as i32);
    config.arti.state_dir = state_dir;
    config.arti.cache_dir = cache_dir;
    config.tcp_stack = TcpStackConfig::System(SystemTcpStackConfig {
        listen: format!("{}:{}", plan.host_ip, plan.listener_port),
    });

    let namespace = NamespaceGuard::create(&plan)?;
    let tor_client = bootstrap_client(&config).await?;
    let tor_handle = spawn_with_client(config, tor_client.clone()).await?;
    wait_for_listener(SocketAddr::new(
        IpAddr::V4(plan.host_ip),
        plan.listener_port,
    ))
    .await?;
    let dns_handle = spawn_dns(
        SocketAddr::new(IpAddr::V4(plan.host_ip), CHILD_DNS_PORT),
        tor_client,
    )
    .await?;

    let status = namespace.run_child(&command).await;
    let dns_shutdown = dns_handle.shutdown().await;
    let tor_shutdown = tor_handle.shutdown().await;

    let status = status?;
    dns_shutdown?;
    tor_shutdown?;
    child_exit_code(status)
}

fn ensure_root() -> Result<()> {
    if unsafe { libc::geteuid() } != 0 {
        bail!("tor-exec currently requires root on linux");
    }
    Ok(())
}

fn ensure_host_tool(tool: &str) -> Result<()> {
    let status = Command::new("sh")
        .args(["-lc", &format!("command -v {tool} >/dev/null")])
        .status()
        .with_context(|| format!("failed to probe required tool '{tool}'"))?;
    if !status.success() {
        bail!("required host tool '{tool}' is not available");
    }
    Ok(())
}

async fn wait_for_listener(addr: SocketAddr) -> Result<()> {
    let deadline = tokio::time::Instant::now() + LISTENER_READY_TIMEOUT;
    loop {
        match tokio::net::TcpStream::connect(addr).await {
            Ok(stream) => {
                drop(stream);
                return Ok(());
            }
            Err(err) if tokio::time::Instant::now() < deadline => {
                debug!(%addr, ?err, "waiting for tor transparent listener");
                tokio::time::sleep(LISTENER_READY_POLL).await;
            }
            Err(err) => return Err(err).with_context(|| format!("timed out waiting for {addr}")),
        }
    }
}

fn child_exit_code(status: ExitStatus) -> Result<i32> {
    if let Some(code) = status.code() {
        return Ok(code);
    }
    if let Some(signal) = status.signal() {
        return Ok(128 + signal);
    }
    bail!("child process terminated without an exit code");
}

#[derive(Debug, Clone)]
struct NamespacePlan {
    netns_name: String,
    host_if: String,
    child_if: String,
    host_ip: Ipv4Addr,
    child_ip: Ipv4Addr,
    listener_port: u16,
}

impl NamespacePlan {
    fn new(listener_port: u16) -> Self {
        let token = std::process::id() % 10_000;
        let segment = ((std::process::id() % 200) as u8) + 20;
        Self {
            netns_name: format!("burrow-tor-{token}"),
            host_if: format!("bth{token}"),
            child_if: format!("btc{token}"),
            host_ip: Ipv4Addr::new(100, 90, segment, 1),
            child_ip: Ipv4Addr::new(100, 90, segment, 2),
            listener_port,
        }
    }

    fn host_cidr(&self) -> String {
        format!("{}/{}", self.host_ip, CHILD_PREFIX_LEN)
    }

    fn child_cidr(&self) -> String {
        format!("{}/{}", self.child_ip, CHILD_PREFIX_LEN)
    }
}

struct NamespaceGuard {
    plan: NamespacePlan,
    resolv_conf: PathBuf,
    nat_rule_installed: bool,
    forward_rule_installed: bool,
    netns_created: bool,
    host_link_created: bool,
}

impl NamespaceGuard {
    fn create(plan: &NamespacePlan) -> Result<Self> {
        let mut guard = Self {
            plan: plan.clone(),
            resolv_conf: write_resolv_conf(plan.host_ip)?,
            nat_rule_installed: false,
            forward_rule_installed: false,
            netns_created: false,
            host_link_created: false,
        };

        let setup = (|| -> Result<()> {
            run_host_command(["ip", "netns", "add", &guard.plan.netns_name])?;
            guard.netns_created = true;

            run_host_command([
                "ip",
                "link",
                "add",
                &guard.plan.host_if,
                "type",
                "veth",
                "peer",
                "name",
                &guard.plan.child_if,
            ])?;
            guard.host_link_created = true;

            run_host_command([
                "ip",
                "addr",
                "add",
                &guard.plan.host_cidr(),
                "dev",
                &guard.plan.host_if,
            ])?;
            run_host_command(["ip", "link", "set", &guard.plan.host_if, "up"])?;
            run_host_command([
                "ip",
                "link",
                "set",
                &guard.plan.child_if,
                "netns",
                &guard.plan.netns_name,
            ])?;
            run_host_command([
                "ip",
                "netns",
                "exec",
                &guard.plan.netns_name,
                "ip",
                "link",
                "set",
                "lo",
                "up",
            ])?;
            run_host_command([
                "ip",
                "netns",
                "exec",
                &guard.plan.netns_name,
                "ip",
                "addr",
                "add",
                &guard.plan.child_cidr(),
                "dev",
                &guard.plan.child_if,
            ])?;
            run_host_command([
                "ip",
                "netns",
                "exec",
                &guard.plan.netns_name,
                "ip",
                "link",
                "set",
                &guard.plan.child_if,
                "up",
            ])?;
            run_host_command([
                "ip",
                "netns",
                "exec",
                &guard.plan.netns_name,
                "ip",
                "route",
                "add",
                "default",
                "via",
                &guard.plan.host_ip.to_string(),
                "dev",
                &guard.plan.child_if,
            ])?;
            run_host_command([
                "iptables",
                "-t",
                "nat",
                "-A",
                "PREROUTING",
                "-i",
                &guard.plan.host_if,
                "-p",
                "tcp",
                "-j",
                "DNAT",
                "--to-destination",
                &format!("{}:{}", guard.plan.host_ip, guard.plan.listener_port),
            ])?;
            guard.nat_rule_installed = true;

            run_host_command([
                "iptables",
                "-A",
                "FORWARD",
                "-i",
                &guard.plan.host_if,
                "-j",
                "REJECT",
            ])?;
            guard.forward_rule_installed = true;
            Ok(())
        })();

        if let Err(err) = setup {
            guard.cleanup();
            return Err(err);
        }

        Ok(guard)
    }

    async fn run_child(&self, command: &[String]) -> Result<ExitStatus> {
        let mut args = vec![
            OsString::from("netns"),
            OsString::from("exec"),
            OsString::from(&self.plan.netns_name),
            OsString::from("unshare"),
            OsString::from("--user"),
            OsString::from("--map-root-user"),
            OsString::from("--mount"),
            OsString::from("--pid"),
            OsString::from("--fork"),
            OsString::from("--kill-child"),
            OsString::from("sh"),
            OsString::from("-ceu"),
            OsString::from(CHILD_SCRIPT),
            OsString::from("sh"),
            self.resolv_conf.as_os_str().to_os_string(),
        ];
        args.extend(command.iter().map(OsString::from));

        let status = TokioCommand::new("ip")
            .args(args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
            .context("failed to execute child in tor namespace")?;
        Ok(status)
    }

    fn cleanup(&mut self) {
        if self.forward_rule_installed {
            let _ = run_host_command([
                "iptables",
                "-D",
                "FORWARD",
                "-i",
                &self.plan.host_if,
                "-j",
                "REJECT",
            ]);
            self.forward_rule_installed = false;
        }
        if self.nat_rule_installed {
            let _ = run_host_command([
                "iptables",
                "-t",
                "nat",
                "-D",
                "PREROUTING",
                "-i",
                &self.plan.host_if,
                "-p",
                "tcp",
                "-j",
                "DNAT",
                "--to-destination",
                &format!("{}:{}", self.plan.host_ip, self.plan.listener_port),
            ]);
            self.nat_rule_installed = false;
        }
        if self.host_link_created {
            let _ = run_host_command(["ip", "link", "delete", &self.plan.host_if]);
            self.host_link_created = false;
        }
        if self.netns_created {
            let _ = run_host_command(["ip", "netns", "delete", &self.plan.netns_name]);
            self.netns_created = false;
        }
        let _ = fs::remove_file(&self.resolv_conf);
    }
}

impl Drop for NamespaceGuard {
    fn drop(&mut self) {
        self.cleanup();
    }
}

fn write_resolv_conf(nameserver: Ipv4Addr) -> Result<PathBuf> {
    let path = std::env::temp_dir().join(format!("burrow-tor-resolv-{}.conf", std::process::id()));
    fs::write(&path, format!("nameserver {nameserver}\noptions ndots:1\n"))
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}

fn run_host_command<const N: usize>(args: [&str; N]) -> Result<()> {
    let (program, rest) = args
        .split_first()
        .expect("run_host_command requires a program and arguments");
    let status = Command::new(program)
        .args(rest)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .with_context(|| format!("failed to start host command {}", shell_words(&args)))?;
    if status.success() {
        Ok(())
    } else {
        bail!("host command failed: {}", shell_words(&args));
    }
}

fn shell_words(args: &[&str]) -> String {
    args.iter()
        .map(|arg| shlex_escape(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shlex_escape(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "-_./:=+".contains(ch))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

const CHILD_SCRIPT: &str = r#"
mount -t proc proc /proc
mount --bind "$1" /etc/resolv.conf
shift
exec "$@"
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_plan_uses_short_interface_names() {
        let plan = NamespacePlan::new(9040);
        assert!(plan.host_if.len() <= 15);
        assert!(plan.child_if.len() <= 15);
    }

    #[test]
    fn signal_exit_code_uses_shell_convention() {
        let status = ExitStatus::from_raw(libc::SIGTERM);
        assert_eq!(child_exit_code(status).unwrap(), 128 + libc::SIGTERM);
    }
}
