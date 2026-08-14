use std::net::TcpListener;

/// 绑定 127.0.0.1:0 让 OS 分配空闲端口，读取后立即释放返回。
/// 释放到 sidecar 绑定时存在极小竞争窗口；若 bind 失败，调用方重试即可。
pub fn find_free_port() -> std::io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::find_free_port;

    #[test]
    fn returns_port_in_valid_range() {
        let port = find_free_port().expect("should find a free port");
        assert!((1..=65535).contains(&port));
    }
}
