use super::{spawn_spec, wait_ready, CREATE_NO_WINDOW, CREATE_SUSPENDED};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn wait_ready_requires_a_successful_http_response() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 256];
        let _ = stream.read(&mut request);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
            .unwrap();
    });
    assert!(wait_ready(port, Duration::from_secs(2)));
    handle.join().unwrap();
}

#[test]
fn wait_ready_rejects_a_listening_server_that_returns_503() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let running = Arc::new(AtomicBool::new(true));
    let server_running = Arc::clone(&running);
    let handle = thread::spawn(move || {
        while server_running.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut request = [0_u8; 256];
                    let _ = stream.read(&mut request);
                    let _ = stream.write_all(
                        b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n",
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("test server failed: {error}"),
            }
        }
    });

    assert!(!wait_ready(port, Duration::from_millis(250)));
    running.store(false, Ordering::Relaxed);
    handle.join().unwrap();
}

#[test]
fn spawn_spec_is_hidden_and_targets_the_official_web_profile() {
    let spec = spawn_spec(
        Path::new("C:/app/runtime/node/node.exe"),
        Path::new("C:/app/runtime/host/dsh/lib/bin.js"),
        4312,
        Path::new("C:/Users/test/AppData/dsh-studio"),
    );

    assert_eq!(
        spec.creation_flags,
        CREATE_NO_WINDOW | CREATE_SUSPENDED,
        "the sidecar must remain suspended until it belongs to the kill-on-close Job"
    );
    assert_eq!(spec.args[0], "C:/app/runtime/host/dsh/lib/bin.js");
    assert_eq!(
        &spec.args[1..],
        ["web", "--host", "127.0.0.1", "--port", "4312"]
    );
    assert_eq!(spec.dsh_home, Path::new("C:/Users/test/AppData/dsh-studio"));
}
