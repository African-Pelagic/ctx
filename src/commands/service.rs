use std::{
    env, fs,
    path::PathBuf,
    process::Command,
};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::{
    cli::{ServiceAction, ServiceArgs},
    output::OutputMode,
};

// ── Unit names ─────────────────────────────────────────────────────────────────
//
// We use socket activation with Accept=yes so that systemd spins up a fresh
// `ctx serve` process for every client connection, wiring the unix socket fd
// directly to the process's stdin/stdout.  The MCP stdio transport (rmcp's
// transport-io) reads fd 0 and writes fd 1 — exactly what systemd provides.
//
// Clients that support unix-socket MCP (e.g. Claude Desktop on Linux) can
// point directly at the socket path.  The socket path is printed by `install`.

const SOCKET_UNIT: &str = "ctx-mcp.socket";
const SERVICE_UNIT: &str = "ctx-mcp@.service";
// The socket unit also implicitly controls ctx-mcp@.service instances.

// ── Unit file templates ────────────────────────────────────────────────────────

fn socket_unit_contents() -> String {
    format!(
        r#"[Unit]
Description=ctx MCP server socket
Documentation=https://github.com/African-Pelagic/ctx

[Socket]
ListenStream=%t/ctx-mcp.sock
Accept=yes

[Install]
WantedBy=sockets.target
"#
    )
}

fn service_unit_contents(binary: &str, workdir: &str) -> String {
    format!(
        r#"[Unit]
Description=ctx MCP server (per-connection instance)
Documentation=https://github.com/African-Pelagic/ctx

[Service]
Type=simple
ExecStart={binary} serve
WorkingDirectory={workdir}
StandardInput=socket
StandardOutput=socket
StandardError=journal
"#,
        binary = binary,
        workdir = workdir,
    )
}

// ── Paths ──────────────────────────────────────────────────────────────────────

fn unit_dir() -> Result<PathBuf> {
    let home = env::var("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("systemd")
        .join("user"))
}

fn socket_unit_path() -> Result<PathBuf> {
    Ok(unit_dir()?.join(SOCKET_UNIT))
}

fn service_unit_path() -> Result<PathBuf> {
    Ok(unit_dir()?.join(SERVICE_UNIT))
}

/// The runtime socket path: /run/user/<uid>/ctx-mcp.sock
fn socket_runtime_path() -> Result<PathBuf> {
    let uid = nix_uid();
    Ok(PathBuf::from(format!("/run/user/{uid}/ctx-mcp.sock")))
}

fn nix_uid() -> u32 {
    // SAFETY: getuid() is always safe to call.
    unsafe { libc::getuid() }
}

fn binary_path() -> Result<String> {
    // Prefer the resolved path of the running binary so install + remove work
    // even when ctx is invoked via a shell wrapper.
    if let Ok(current) = env::current_exe() {
        return Ok(current.to_string_lossy().into_owned());
    }
    which_ctx()
}

fn which_ctx() -> Result<String> {
    let output = Command::new("which")
        .arg("ctx")
        .output()
        .context("failed to run `which ctx`")?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(path);
        }
    }
    bail!(
        "cannot locate ctx binary; make sure it is on PATH or re-run after `cargo install --path .`"
    )
}

// ── systemctl helper ───────────────────────────────────────────────────────────

fn systemctl(args: &[&str]) -> Result<String> {
    let output = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .with_context(|| format!("failed to run `systemctl --user {}`", args.join(" ")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        bail!(
            "`systemctl --user {}` failed (exit {})\n{}{}",
            args.join(" "),
            output.status.code().unwrap_or(-1),
            stdout,
            stderr,
        );
    }

    Ok(stdout)
}

// ── Output type ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ServiceResult {
    action: String,
    socket_unit: String,
    service_unit: String,
    socket_path: Option<String>,
    service_unit_path: Option<String>,
    socket_unit_path: Option<String>,
    binary: Option<String>,
    workdir: Option<String>,
}

// ── Actions ────────────────────────────────────────────────────────────────────

fn install(args: &ServiceArgs, output_mode: OutputMode) -> Result<()> {
    let binary = binary_path()?;

    let workdir = match &args.workdir {
        Some(w) => w.clone(),
        None => env::current_dir()
            .context("failed to determine current directory")?
            .to_string_lossy()
            .into_owned(),
    };

    let dir = unit_dir()?;
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create {}", dir.display()))?;

    let socket_path = socket_unit_path()?;
    let service_path = service_unit_path()?;

    fs::write(&socket_path, socket_unit_contents())
        .with_context(|| format!("failed to write {}", socket_path.display()))?;
    fs::write(&service_path, service_unit_contents(&binary, &workdir))
        .with_context(|| format!("failed to write {}", service_path.display()))?;

    systemctl(&["daemon-reload"])?;
    systemctl(&["enable", SOCKET_UNIT])?;

    let runtime_socket = socket_runtime_path()?;
    let result = ServiceResult {
        action: "install".into(),
        socket_unit: SOCKET_UNIT.into(),
        service_unit: SERVICE_UNIT.into(),
        socket_path: Some(runtime_socket.to_string_lossy().into_owned()),
        socket_unit_path: Some(socket_path.to_string_lossy().into_owned()),
        service_unit_path: Some(service_path.to_string_lossy().into_owned()),
        binary: Some(binary),
        workdir: Some(workdir),
    };

    match output_mode {
        OutputMode::Human => {
            println!("Installed ctx MCP service (socket-activated)");
            println!("  socket unit:  {}", result.socket_unit_path.as_deref().unwrap_or(""));
            println!("  service unit: {}", result.service_unit_path.as_deref().unwrap_or(""));
            println!("  binary:       {}", result.binary.as_deref().unwrap_or(""));
            println!("  workdir:      {}", result.workdir.as_deref().unwrap_or(""));
            println!("  socket path:  {}", result.socket_path.as_deref().unwrap_or(""));
            println!();
            println!("Run `ctx service start` to activate the socket.");
            println!("Run `journalctl --user -u ctx-mcp@*.service` to see connection logs.");
        }
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&result)?),
        OutputMode::Porcelain => {
            println!("install");
            println!("socket_unit {}", result.socket_unit_path.as_deref().unwrap_or(""));
            println!("service_unit {}", result.service_unit_path.as_deref().unwrap_or(""));
            println!("socket_path {}", result.socket_path.as_deref().unwrap_or(""));
            println!("binary {}", result.binary.as_deref().unwrap_or(""));
            println!("workdir {}", result.workdir.as_deref().unwrap_or(""));
        }
    }

    Ok(())
}

fn remove(output_mode: OutputMode) -> Result<()> {
    // Stop all running instances and the socket.
    let _ = systemctl(&["stop", SOCKET_UNIT]);
    let _ = systemctl(&["stop", "ctx-mcp@*.service"]);
    let _ = systemctl(&["disable", SOCKET_UNIT]);

    let socket_path = socket_unit_path()?;
    let service_path = service_unit_path()?;

    if socket_path.exists() {
        fs::remove_file(&socket_path)
            .with_context(|| format!("failed to remove {}", socket_path.display()))?;
    }
    if service_path.exists() {
        fs::remove_file(&service_path)
            .with_context(|| format!("failed to remove {}", service_path.display()))?;
    }

    systemctl(&["daemon-reload"])?;

    let result = ServiceResult {
        action: "remove".into(),
        socket_unit: SOCKET_UNIT.into(),
        service_unit: SERVICE_UNIT.into(),
        socket_path: None,
        socket_unit_path: Some(socket_path.to_string_lossy().into_owned()),
        service_unit_path: Some(service_path.to_string_lossy().into_owned()),
        binary: None,
        workdir: None,
    };

    match output_mode {
        OutputMode::Human => println!("Removed ctx MCP service units"),
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&result)?),
        OutputMode::Porcelain => {
            println!("remove");
            println!("socket_unit {}", result.socket_unit_path.as_deref().unwrap_or(""));
            println!("service_unit {}", result.service_unit_path.as_deref().unwrap_or(""));
        }
    }

    Ok(())
}

fn start(output_mode: OutputMode) -> Result<()> {
    if !socket_unit_path()?.exists() {
        bail!("unit files not found; run `ctx service install` first");
    }

    systemctl(&["start", SOCKET_UNIT])?;

    let runtime_socket = socket_runtime_path()?;
    let result = ServiceResult {
        action: "start".into(),
        socket_unit: SOCKET_UNIT.into(),
        service_unit: SERVICE_UNIT.into(),
        socket_path: Some(runtime_socket.to_string_lossy().into_owned()),
        socket_unit_path: None,
        service_unit_path: None,
        binary: None,
        workdir: None,
    };

    match output_mode {
        OutputMode::Human => {
            println!("Started {SOCKET_UNIT}");
            println!("  socket: {}", result.socket_path.as_deref().unwrap_or(""));
            println!();
            println!("ctx serve will be launched automatically on each connection.");
            println!("Run `journalctl --user -u ctx-mcp@*.service` to see connection logs.");
        }
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&result)?),
        OutputMode::Porcelain => {
            println!("start {SOCKET_UNIT}");
            println!("socket_path {}", result.socket_path.as_deref().unwrap_or(""));
        }
    }

    Ok(())
}

fn stop(output_mode: OutputMode) -> Result<()> {
    // Stop all active service instances, then the socket.
    let _ = systemctl(&["stop", "ctx-mcp@*.service"]);
    systemctl(&["stop", SOCKET_UNIT])?;

    let result = ServiceResult {
        action: "stop".into(),
        socket_unit: SOCKET_UNIT.into(),
        service_unit: SERVICE_UNIT.into(),
        socket_path: None,
        socket_unit_path: None,
        service_unit_path: None,
        binary: None,
        workdir: None,
    };

    match output_mode {
        OutputMode::Human => println!("Stopped {SOCKET_UNIT} and all active instances"),
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&result)?),
        OutputMode::Porcelain => println!("stop {SOCKET_UNIT}"),
    }

    Ok(())
}

// ── Entry point ────────────────────────────────────────────────────────────────

pub fn run(args: ServiceArgs, output_mode: OutputMode) -> Result<()> {
    match args.action {
        ServiceAction::Install => install(&args, output_mode),
        ServiceAction::Remove => remove(output_mode),
        ServiceAction::Start => start(output_mode),
        ServiceAction::Stop => stop(output_mode),
    }
}
