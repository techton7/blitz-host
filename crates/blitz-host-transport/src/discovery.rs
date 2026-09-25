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
    let temp_path = dir.join(format!(
        "{}.tmp.{}",
        descriptor.instance_id,
        std::process::id()
    ));

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
            #[cfg(unix)]
            let _ = fs::remove_file(Path::new(&desc.socket_path));
        }
    }

    Ok(reachable_hosts)
}

/// Disambiguates an active host from a list of discovered reachable hosts according to the selector.
///
/// Policy:
/// - `TargetSelector::Auto`:
///   - 0 hosts: Returns `ErrorKind::NotFound`.
///   - 1 host: Returns the single host (`Ok(host)`).
///   - 2+ hosts: Fails fast with `ErrorKind::InvalidInput` listing the competing PIDs and demanding `--pid`.
/// - `TargetSelector::Pid(target_pid)`:
///   - Matches the host with `host.pid == target_pid`, or returns `ErrorKind::NotFound`.
/// - `TargetSelector::ExplicitPath`:
///   - Not applicable to host lists; handled directly in `discover_target`.
pub fn resolve_target_from_hosts(
    selector: &TargetSelector,
    hosts: Vec<HostDescriptor>,
) -> io::Result<HostDescriptor> {
    match selector {
        TargetSelector::ExplicitPath(path) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Cannot resolve explicit path '{}' from host list",
                path.display()
            ),
        )),
        TargetSelector::Pid(target_pid) => hosts
            .into_iter()
            .find(|d| d.pid == *target_pid)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("No live, reachable Blitz host found with PID {target_pid}"),
                )
            }),
        TargetSelector::Auto => match hosts.len() {
            0 => Err(io::Error::new(
                io::ErrorKind::NotFound,
                "No active blitz-host instances found in $TMPDIR/blitz-host",
            )),
            1 => Ok(hosts.into_iter().next().unwrap()),
            _ => {
                let pids: Vec<u32> = hosts.iter().map(|h| h.pid).collect();
                Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "Multiple active Blitz hosts detected (PIDs: {pids:?}). \
                         Target is ambiguous: please specify --pid <PID> (or connect_pid) to target a specific host."
                    ),
                ))
            }
        },
    }
}

/// Discover a live, reachable Blitz host according to the provided selector.
pub fn discover_target(selector: &TargetSelector) -> io::Result<HostDescriptor> {
    match selector {
        TargetSelector::ExplicitPath(path) => {
            let path_str = path.to_string_lossy();
            if path_str.starts_with(r"\\.\pipe\") {
                let desc = HostDescriptor {
                    protocol_version: blitz_host_protocol::PROTOCOL_VERSION,
                    pid: 0,
                    instance_id: "explicit-pipe".to_string(),
                    socket_path: path_str.to_string(),
                    renderer: "blitz".to_string(),
                    renderer_version: "unknown".to_string(),
                    primary_window_id: None,
                    primary_document_id: None,
                };
                if is_reachable(&desc) {
                    return Ok(desc);
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionRefused,
                        format!(
                            "Explicit Named Pipe at '{}' is unreachable",
                            desc.socket_path
                        ),
                    ));
                }
            }
            if !path.exists() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "Explicit descriptor or socket '{}' does not exist",
                        path.display()
                    ),
                ));
            }
            if path.extension().is_some_and(|ext| ext == "sock") {
                let desc = HostDescriptor {
                    protocol_version: blitz_host_protocol::PROTOCOL_VERSION,
                    pid: 0,
                    instance_id: "explicit-socket".to_string(),
                    socket_path: path_str.to_string(),
                    renderer: "blitz".to_string(),
                    renderer_version: "unknown".to_string(),
                    primary_window_id: None,
                    primary_document_id: None,
                };
                if is_reachable(&desc) {
                    return Ok(desc);
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionRefused,
                        format!("Control socket at '{}' is unreachable", desc.socket_path),
                    ));
                }
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
        TargetSelector::Pid(_) | TargetSelector::Auto => {
            let hosts = list_hosts()?;
            resolve_target_from_hosts(selector, hosts)
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

/// Resolve a platform-appropriate local socket name from a socket_path string.
pub fn socket_name_from_str(s: &str) -> io::Result<interprocess::local_socket::Name<'_>> {
    #[cfg(windows)]
    {
        use interprocess::local_socket::{GenericNamespaced, ToNsName};
        let clean = s.strip_prefix(r"\\.\pipe\").unwrap_or(s);
        clean.to_ns_name::<GenericNamespaced>()
    }
    #[cfg(not(windows))]
    {
        use interprocess::local_socket::{GenericFilePath, ToFsName};
        Path::new(s).to_fs_name::<GenericFilePath>()
    }
}

/// Test whether the socket named in the descriptor accepts connections.
pub fn is_reachable(descriptor: &HostDescriptor) -> bool {
    use interprocess::local_socket::traits::Stream as _;
    let Ok(name) = socket_name_from_str(&descriptor.socket_path) else {
        return false;
    };
    interprocess::local_socket::Stream::connect(name).is_ok()
}

/// Check if a process ID is currently running.
fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // kill(pid, 0) checks if process exists without sending a signal
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };

        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return false;
            }
            let mut exit_code: u32 = 0;
            let success = GetExitCodeProcess(handle, &mut exit_code);
            CloseHandle(handle);
            success != 0 && exit_code == STILL_ACTIVE as u32
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_descriptor(pid: u32) -> HostDescriptor {
        HostDescriptor {
            protocol_version: 1,
            pid,
            instance_id: format!("test-{pid}"),
            socket_path: format!("/tmp/test-{pid}.sock"),
            renderer: "test".to_string(),
            renderer_version: "0.1.0".to_string(),
            primary_window_id: None,
            primary_document_id: None,
        }
    }

    #[test]
    fn test_resolve_target_auto_zero_hosts_returns_not_found() {
        let res = resolve_target_from_hosts(&TargetSelector::Auto, vec![]);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_resolve_target_auto_single_host_succeeds() {
        let host1 = dummy_descriptor(1001);
        let res = resolve_target_from_hosts(&TargetSelector::Auto, vec![host1]);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().pid, 1001);
    }

    #[test]
    fn test_resolve_target_auto_multiple_hosts_blocks_with_ambiguity() {
        let host1 = dummy_descriptor(1001);
        let host2 = dummy_descriptor(1002);
        let res = resolve_target_from_hosts(&TargetSelector::Auto, vec![host1, host2]);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        let msg = err.to_string();
        assert!(msg.contains("Multiple active Blitz hosts detected"));
        assert!(msg.contains("1001"));
        assert!(msg.contains("1002"));
        assert!(msg.contains("--pid"));
    }

    #[test]
    fn test_resolve_target_pid_disambiguation_succeeds() {
        let host1 = dummy_descriptor(1001);
        let host2 = dummy_descriptor(1002);
        let res = resolve_target_from_hosts(&TargetSelector::Pid(1002), vec![host1, host2]);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().pid, 1002);
    }

    #[test]
    fn test_resolve_target_pid_missing_returns_not_found() {
        let host1 = dummy_descriptor(1001);
        let res = resolve_target_from_hosts(&TargetSelector::Pid(9999), vec![host1]);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), io::ErrorKind::NotFound);
    }
}
