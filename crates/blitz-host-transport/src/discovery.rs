use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use blitz_host_protocol::HostDescriptor;

/// Base directory for blitz-host descriptors and sockets.
pub fn descriptor_dir() -> PathBuf {
    std::env::temp_dir().join("blitz-host")
}

/// Atomically write a descriptor file with owner-only (0o600) permissions.
pub fn write_descriptor(descriptor: &HostDescriptor) -> io::Result<PathBuf> {
    let dir = descriptor_dir();
    fs::create_dir_all(&dir)?;

    let target_path = dir.join(format!("{}.json", descriptor.instance_id));
    let temp_path = dir.join(format!("{}.tmp.{}", descriptor.instance_id, std::process::id()));

    let bytes = serde_json::to_vec_pretty(descriptor).map_err(io::Error::other)?;

    let write_result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            options.mode(0o600);
        }
        let mut file = options.open(&temp_path)?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        fs::rename(&temp_path, &target_path)
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    write_result.map(|()| target_path)
}

/// Selection criteria for targeting an active Blitz host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetSelector {
    /// Auto-discover the latest reachable host.
    Auto,
    /// Connect to a host running with a specific OS process ID.
    Pid(u32),
    /// Connect to a host at an explicit descriptor or socket path.
    ExplicitPath(PathBuf),
}

impl Default for TargetSelector {
    fn default() -> Self {
        Self::Auto
    }
}

/// Enumerate all currently advertised Blitz host descriptors, sorted newest first.
/// Dead hosts with unalive PIDs are pruned automatically.
pub fn list_hosts() -> io::Result<Vec<HostDescriptor>> {
    let dir = descriptor_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&dir)?;
    let mut descriptors = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            if let Ok(metadata) = path.metadata() {
                let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                if let Ok(desc) = read_descriptor(&path) {
                    descriptors.push((modified, path, desc));
                }
            }
        }
    }

    // Sort newest first
    descriptors.sort_by(|a, b| b.0.cmp(&a.0));

    let mut reachable_hosts = Vec::new();
    for (_modified, path, desc) in descriptors {
        if is_reachable(&desc) {
            reachable_hosts.push(desc);
        } else if !is_pid_alive(desc.pid) {
            // Reap stale descriptor and socket for dead PID
            let _ = fs::remove_file(&path);
            let _ = fs::remove_file(Path::new(&desc.socket_path));
        }
    }

    Ok(reachable_hosts)
}

/// Discover a live, reachable Blitz host according to the provided selector.
pub fn discover_target(selector: &TargetSelector) -> io::Result<HostDescriptor> {
    match selector {
        TargetSelector::ExplicitPath(path) => {
            if !path.exists() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Explicit descriptor or socket '{}' does not exist", path.display()),
                ));
            }
            if path.extension().is_some_and(|ext| ext == "sock") {
                return Ok(HostDescriptor {
                    protocol_version: blitz_host_protocol::PROTOCOL_VERSION,
                    pid: 0,
                    instance_id: "explicit-socket".to_string(),
                    socket_path: path.to_string_lossy().to_string(),
                    renderer: "blitz".to_string(),
                    renderer_version: "unknown".to_string(),
                    primary_window_id: None,
                    primary_document_id: None,
                });
            }
            let desc = read_descriptor(path)?;
            if is_reachable(&desc) {
                Ok(desc)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::ConnectionRefused,
                    format!("Control socket at '{}' is unreachable", desc.socket_path),
                ))
            }
        }
        TargetSelector::Pid(target_pid) => {
            let hosts = list_hosts()?;
            hosts
                .into_iter()
                .find(|d| d.pid == *target_pid)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("No live, reachable Blitz host found with PID {target_pid}"),
                    )
                })
        }
        TargetSelector::Auto => {
            let hosts = list_hosts()?;
            hosts.into_iter().next().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "No active blitz-host instances found in $TMPDIR/blitz-host",
                )
            })
        }
    }
}

/// Discover a live, reachable Blitz host on the local machine (backward-compatible).
pub fn discover(explicit: Option<&Path>) -> io::Result<HostDescriptor> {
    match explicit {
        Some(path) => discover_target(&TargetSelector::ExplicitPath(path.to_path_buf())),
        None => discover_target(&TargetSelector::Auto),
    }
}

/// Read and parse a HostDescriptor from a JSON file.
pub fn read_descriptor(path: &Path) -> io::Result<HostDescriptor> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Test whether the socket named in the descriptor accepts connections.
pub fn is_reachable(descriptor: &HostDescriptor) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::net::UnixStream::connect(&descriptor.socket_path).is_ok()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// Check if a process ID is currently running.
fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // kill(pid, 0) checks if process exists without sending a signal
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        true
    }
}
