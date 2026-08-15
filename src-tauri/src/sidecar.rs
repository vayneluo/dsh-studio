use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct SidecarHandle {
    child: Child,
}

#[derive(Debug)]
pub(crate) struct SpawnSpec {
    program: PathBuf,
    args: Vec<OsString>,
    dsh_home: PathBuf,
    creation_flags: u32,
}

pub(crate) fn spawn_spec(
    node: &Path,
    dsh_bin: &Path,
    port: u16,
    dsh_home: &Path,
) -> SpawnSpec {
    SpawnSpec {
        program: node.to_path_buf(),
        args: vec![
            dsh_bin.as_os_str().to_owned(),
            "web".into(),
            "--host".into(),
            "127.0.0.1".into(),
            "--port".into(),
            port.to_string().into(),
        ],
        dsh_home: dsh_home.to_path_buf(),
        creation_flags: CREATE_NO_WINDOW,
    }
}

/// Start the official DSH Web profile with output redirected to one append-only log.
pub fn spawn(
    node: &Path,
    dsh_bin: &Path,
    port: u16,
    dsh_home: &Path,
    log_path: &Path,
) -> std::io::Result<SidecarHandle> {
    let spec = spawn_spec(node, dsh_bin, port, dsh_home);
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    let stderr = stdout.try_clone()?;

    let mut command = Command::new(spec.program);
    command
        .args(spec.args)
        .env("DSH_HOME", spec.dsh_home)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .creation_flags(spec.creation_flags);
    let child = command.spawn()?;
    Ok(SidecarHandle { child })
}

fn probe_http(port: u16, timeout: Duration) -> bool {
    let address = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port));
    let Ok(mut stream) = TcpStream::connect_timeout(&address, timeout) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    if stream
        .write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }

    let mut response = [0_u8; 64];
    let Ok(read) = stream.read(&mut response) else {
        return false;
    };
    let Ok(status_line) = std::str::from_utf8(&response[..read]) else {
        return false;
    };
    status_line
        .split_whitespace()
        .nth(1)
        .and_then(|status| status.parse::<u16>().ok())
        .is_some_and(|status| (200..400).contains(&status))
}

/// Wait until the actual Web page responds successfully, not merely until the port binds.
pub fn wait_ready(port: u16, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if probe_http(port, Duration::from_millis(250)) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

impl SidecarHandle {
    /// Terminate the process tree first, then reap the root child handle.
    pub fn kill(mut self) {
        let pid = self.child.id();
        let mut taskkill = Command::new("taskkill");
        let tree_killed = taskkill
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .is_ok_and(|status| status.success());
        if !tree_killed {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests;
