use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use blitz_host_protocol::{ControlRequest, ControlResponse, HostDescriptor, PROTOCOL_VERSION};

use crate::discovery::{descriptor_dir, write_descriptor};
use crate::waker::ServiceWaker;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// A request received over the transport that must be serviced by the host/UI thread.
#[derive(Debug)]
pub struct ControlBridgeRequest {
    pub request: ControlRequest,
    pub reply: SyncSender<ControlResponse>,
}

impl ControlBridgeRequest {
    /// Send response back to the client.
    pub fn respond(self, response: ControlResponse) {
        let _ = self.reply.send(response);
    }
}

/// Running debug control server listening on a local Unix Domain Socket.
pub struct DebugServer {
    descriptor: HostDescriptor,
    descriptor_path: PathBuf,
    socket_path: PathBuf,
    waker: ServiceWaker,
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl DebugServer {
    /// Bind a local Unix Domain Socket and announce the host descriptor in $TMPDIR/blitz-host.
    pub fn start(
        renderer: impl Into<String>,
        renderer_version: impl Into<String>,
    ) -> std::io::Result<(Self, Receiver<ControlBridgeRequest>)> {
        let dir = descriptor_dir();
        fs::create_dir_all(&dir)?;

        let instance_id = format!("{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        let socket_path = dir.join(format!("{}.sock", instance_id));
        let _ = fs::remove_file(&socket_path);

        let listener = UnixListener::bind(&socket_path)?;
        fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o600))?;

        let descriptor = HostDescriptor {
            protocol_version: PROTOCOL_VERSION,
            pid: std::process::id(),
            instance_id: instance_id.clone(),
            socket_path: socket_path.to_string_lossy().to_string(),
            renderer: renderer.into(),
            renderer_version: renderer_version.into(),
        };

        let descriptor_path = write_descriptor(&descriptor)?;

        let (command_tx, command_rx) = mpsc::sync_channel(32);
        let shutdown = Arc::new(AtomicBool::new(false));
        let waker = ServiceWaker::default();

        let thread_shutdown = Arc::clone(&shutdown);
        let thread_waker = waker.clone();
        let thread_listener = listener.try_clone()?;

        let thread = thread::Builder::new()
            .name("blitz-host-server".into())
            .spawn(move || {
                server_loop(thread_listener, command_tx, thread_waker, thread_shutdown);
            })?;

        Ok((
            Self {
                descriptor,
                descriptor_path,
                socket_path,
                waker,
                shutdown,
                thread: Some(thread),
            },
            command_rx,
        ))
    }

    /// Access the handle used to wake the UI thread upon incoming requests.
    pub fn waker(&self) -> ServiceWaker {
        self.waker.clone()
    }

    /// Access the published host descriptor.
    pub fn descriptor(&self) -> &HostDescriptor {
        &self.descriptor
    }

    /// Socket path on disk.
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }

    /// Explicitly stop the server and unpublish descriptor/socket files.
    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        // Connect dummy stream to unblock listener.accept()
        let _ = UnixStream::connect(&self.socket_path);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = fs::remove_file(&self.socket_path);
        let _ = fs::remove_file(&self.descriptor_path);
    }
}

impl Drop for DebugServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn server_loop(
    listener: UnixListener,
    command_tx: SyncSender<ControlBridgeRequest>,
    waker: ServiceWaker,
    shutdown: Arc<AtomicBool>,
) {
    for stream in listener.incoming() {
        if shutdown.load(Ordering::Acquire) {
            break;
        }

        match stream {
            Ok(mut stream) => {
                let command_tx = command_tx.clone();
                let waker = waker.clone();
                let _ = thread::Builder::new()
                    .name("blitz-host-conn".into())
                    .spawn(move || {
                        handle_connection(&mut stream, command_tx, waker);
                    });
            }
            Err(_) if shutdown.load(Ordering::Acquire) => break,
            Err(_) => continue,
        }
    }
}

fn handle_connection(
    stream: &mut UnixStream,
    command_tx: SyncSender<ControlBridgeRequest>,
    waker: ServiceWaker,
) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();

    while reader.read_line(&mut line).unwrap_or(0) > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        let response = match serde_json::from_str::<ControlRequest>(trimmed) {
            Ok(request) => {
                let (reply_tx, reply_rx) = mpsc::sync_channel(1);
                let bridge_req = ControlBridgeRequest {
                    request,
                    reply: reply_tx,
                };

                if command_tx.try_send(bridge_req).is_ok() {
                    waker.wake();
                    match reply_rx.recv_timeout(REQUEST_TIMEOUT) {
                        Ok(resp) => resp,
                        Err(RecvTimeoutError::Timeout) => {
                            ControlResponse::Error("Host UI thread request timed out after 5s".into())
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            ControlResponse::Error("Host UI thread bridge disconnected".into())
                        }
                    }
                } else {
                    ControlResponse::Error("Host request queue is full".into())
                }
            }
            Err(e) => ControlResponse::Error(format!("Malformed request JSON: {e}")),
        };

        let mut out = serde_json::to_string(&response).unwrap();
        out.push('\n');
        if stream.write_all(out.as_bytes()).is_err() || stream.flush().is_err() {
            break;
        }

        line.clear();
    }
}
