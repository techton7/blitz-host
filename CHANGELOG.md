# Changelog

All notable changes to `blitz-host` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-10-14

### Added
- **Replaceable `ServiceWaker` Lifecycle**: Replaced `OnceLock` with `Arc<RwLock<Option<WakeCallback>>>` in `blitz-host-transport::waker::ServiceWaker`.
- **Dynamic Event Loop Re-binding**: Added `set_waker()` and `clear_waker()` to `DebugServer` and runtime host, enabling dynamic waker updates across winit window lifecycles.
- **Contention-Free Callback Dispatch**: Ensured the read lock in `ServiceWaker::wake()` is cloned and dropped before callback invocation to prevent lock contention.
- **Protocol `ServiceResult` Status**: Enhanced IPC protocol envelope in `blitz-host-protocol` with explicit `ServiceResult` diagnostic status.
- **Client Timeout Diagnostics**: Aligned `DebugClient` socket receive timeout to 8 seconds with descriptive diagnostic error mapping on timeout.

### Changed
- **Upstream Blitz Engine Parity**: Synchronized Blitz dependency pins to `techton-main-v0.2.3` across all workspace crates, adopting Blink-aligned stacking context paint ordering.
- **Cleaned Manifest**: Removed unused `wgpu_context` patch from root `Cargo.toml`.
