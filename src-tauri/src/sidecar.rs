use std::net::TcpStream;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

pub struct SidecarHandle {
    child: Child,
    port: u16,
}

/// 启动 node <dsh_bin> web，绑定 loopback，设置 DSH_HOME。
pub fn spawn(node: &str, dsh_bin: &str, port: u16, dsh_home: &str) -> std::io::Result<SidecarHandle> {
    let child = Command::new(node)
        .arg(dsh_bin)
        .arg("web")
        .arg("--host").arg("127.0.0.1")
        .arg("--port").arg(port.to_string())
        .env("DSH_HOME", dsh_home)
        .spawn()?;
    Ok(SidecarHandle { child, port })
}

/// TCP 连接探测：端口开始接受连接即认为就绪。
pub fn wait_ready(port: u16, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

impl SidecarHandle {
    pub fn port(&self) -> u16 { self.port }

    /// 终止 sidecar 及其整棵子进程树（agent 会 spawn bash/pwsh/工具子进程）。
    pub fn kill(self) {
        let pid = self.child.id();
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .spawn();
        let mut child = self.child;
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::wait_ready;
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn wait_ready_true_when_server_listens() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                if let Ok((_s, _)) = listener.accept() { break }
            }
        });
        assert!(wait_ready(port, Duration::from_secs(2)));
        handle.join().unwrap();
    }

    #[test]
    fn wait_ready_false_when_nothing_listens() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        drop(l);
        assert!(!wait_ready(port, Duration::from_millis(300)));
    }
}
