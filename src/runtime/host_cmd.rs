//! Host subprocess execution for Deno `Deno.Command` / `run` parity (native).

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub code: i64,
    pub stdout: String,
    pub stderr: String,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_command(program: &str, args: &[String]) -> Result<CommandOutput, String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("run_command failed: {e}"))?;
    Ok(CommandOutput {
        code: output.status.code().unwrap_or(-1) as i64,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

#[cfg(target_arch = "wasm32")]
pub fn run_command(_program: &str, _args: &[String]) -> Result<CommandOutput, String> {
    Err("run_command() is not available on wasm32".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn resolve_dns(host: &str, port: u16) -> Result<Vec<String>, String> {
    use std::net::ToSocketAddrs;
    let addrs: Vec<_> = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("resolve_dns failed: {e}"))?
        .map(|a| a.to_string())
        .collect();
    if addrs.is_empty() {
        return Err(format!("resolve_dns: no addresses for {host}"));
    }
    Ok(addrs)
}

#[cfg(target_arch = "wasm32")]
pub fn resolve_dns(_host: &str, _port: u16) -> Result<Vec<String>, String> {
    Err("resolve_dns() is not available on wasm32".into())
}
