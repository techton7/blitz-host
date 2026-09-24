use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use blitz_host_protocol::{
    ActionRequest, ActionResponse, CaptureRequest, CaptureResponse, ControlRequest,
    ControlResponse, HostDescriptor, InspectRequest, InspectResponse, KeyModifiers,
    SettleRequest, SettleResponse,
};

/// Client used by agents or test runners to interact with a live running Blitz host.
pub struct DebugClient {
    stream: UnixStream,
    reader: BufReader<UnixStream>,
    descriptor: HostDescriptor,
}

impl DebugClient {
    /// Connect to a running Blitz host identified by its discovery descriptor.
    pub fn connect(descriptor: HostDescriptor) -> io::Result<Self> {
        let stream = UnixStream::connect(&descriptor.socket_path)?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        let reader = BufReader::new(stream.try_clone()?);

        Ok(Self {
            stream,
            reader,
            descriptor,
        })
    }

    /// Connect to a live Blitz host matching the given TargetSelector.
    pub fn connect_target(selector: &crate::discovery::TargetSelector) -> io::Result<Self> {
        let descriptor = crate::discovery::discover_target(selector)?;
        Self::connect(descriptor)
    }

    /// Convenience helper to connect to a host running with a specific OS process ID.
    pub fn connect_pid(pid: u32) -> io::Result<Self> {
        Self::connect_target(&crate::discovery::TargetSelector::Pid(pid))
    }

    /// Automatically discover a live host on the local machine and connect to it.
    pub fn connect_discovered(explicit_path: Option<&Path>) -> io::Result<Self> {
        let selector = match explicit_path {
            Some(p) => crate::discovery::TargetSelector::ExplicitPath(p.to_path_buf()),
            None => crate::discovery::TargetSelector::Auto,
        };
        Self::connect_target(&selector)
    }

    /// Access the host descriptor.
    pub fn descriptor(&self) -> &HostDescriptor {
        &self.descriptor
    }

    /// Send a low-level control request and receive the response.
    pub fn send_request(&mut self, request: ControlRequest) -> io::Result<ControlResponse> {
        let mut line = serde_json::to_string(&request).map_err(io::Error::other)?;
        line.push('\n');
        self.stream.write_all(line.as_bytes())?;
        self.stream.flush()?;

        let mut resp_line = String::new();
        let bytes_read = self.reader.read_line(&mut resp_line)?;
        if bytes_read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Control server disconnected without responding",
            ));
        }

        let resp: ControlResponse =
            serde_json::from_str(resp_line.trim()).map_err(io::Error::other)?;
        Ok(resp)
    }

    /// High-level inspect command: inspects the live host's DOM/semantic tree.
    pub fn inspect(&mut self, req: InspectRequest) -> io::Result<InspectResponse> {
        match self.send_request(ControlRequest::Inspect(req))? {
            ControlResponse::InspectSuccess(inspect_resp) => Ok(inspect_resp),
            ControlResponse::Error(err) => Err(io::Error::other(err)),
            other => Err(io::Error::other(format!(
                "Unexpected response variant for inspect: {other:?}"
            ))),
        }
    }

    /// Execute a UI action on the host.
    pub fn act(&mut self, action: ActionRequest) -> io::Result<ActionResponse> {
        match self.send_request(ControlRequest::Act(action))? {
            ControlResponse::ActionSuccess(action_resp) => Ok(action_resp),
            ControlResponse::Error(err) => Err(io::Error::other(err)),
            other => Err(io::Error::other(format!(
                "Unexpected response variant for act: {other:?}"
            ))),
        }
    }

    /// Convenience helper to click on a specific node by ID on the default/fallback window.
    pub fn click(&mut self, node_id: u64) -> io::Result<ActionResponse> {
        self.click_window(None, node_id)
    }

    /// Click on a specific node by ID on a targeted window (or primary fallback if None).
    pub fn click_window(&mut self, window_id: Option<u64>, node_id: u64) -> io::Result<ActionResponse> {
        self.act(ActionRequest::Click { window_id, node_id })
    }

    /// Convenience helper to focus a specific node by ID on the default/fallback window.
    pub fn focus(&mut self, node_id: u64) -> io::Result<ActionResponse> {
        self.focus_window(None, node_id)
    }

    /// Focus a specific node by ID on a targeted window (or primary fallback if None).
    pub fn focus_window(&mut self, window_id: Option<u64>, node_id: u64) -> io::Result<ActionResponse> {
        self.act(ActionRequest::Focus { window_id, node_id })
    }

    /// Convenience helper to set the value of an input node by ID on the default/fallback window.
    pub fn set_value(&mut self, node_id: u64, value: impl Into<String>) -> io::Result<ActionResponse> {
        self.set_value_window(None, node_id, value)
    }

    /// Set the value of an input node by ID on a targeted window (or primary fallback if None).
    pub fn set_value_window(
        &mut self,
        window_id: Option<u64>,
        node_id: u64,
        value: impl Into<String>,
    ) -> io::Result<ActionResponse> {
        self.act(ActionRequest::SetValue {
            window_id,
            node_id,
            value: value.into(),
        })
    }

    /// Dispatch synthetic key event with optional modifiers to an optional targeted node.
    pub fn key_with_modifiers(
        &mut self,
        window_id: Option<u64>,
        node_id: Option<u64>,
        key: impl Into<String>,
        modifiers: Option<KeyModifiers>,
    ) -> io::Result<ActionResponse> {
        self.act(ActionRequest::Key {
            window_id,
            node_id,
            key: key.into(),
            modifiers,
        })
    }

    /// Dispatch synthetic key event to an optional targeted node.
    pub fn key(
        &mut self,
        window_id: Option<u64>,
        node_id: Option<u64>,
        key: impl Into<String>,
    ) -> io::Result<ActionResponse> {
        self.key_with_modifiers(window_id, node_id, key, None)
    }

    /// Focus next node via Tab key.
    pub fn tab(&mut self) -> io::Result<ActionResponse> {
        self.key(None, None, "Tab")
    }

    /// Focus previous node via Shift+Tab.
    pub fn shift_tab(&mut self) -> io::Result<ActionResponse> {
        self.key_with_modifiers(None, None, "Tab", Some(KeyModifiers::SHIFT))
    }

    /// Activate focused node or submit via Enter key.
    pub fn enter(&mut self) -> io::Result<ActionResponse> {
        self.key(None, None, "Enter")
    }

    /// Activate focused node via Space key.
    pub fn space(&mut self) -> io::Result<ActionResponse> {
        self.key(None, None, "Space")
    }

    /// Clear focus or dismiss via Escape key.
    pub fn escape(&mut self) -> io::Result<ActionResponse> {
        self.key(None, None, "Escape")
    }

    /// Delete preceding character via Backspace.
    pub fn backspace(&mut self, node_id: Option<u64>) -> io::Result<ActionResponse> {
        self.key(None, node_id, "Backspace")
    }

    /// Delete following character via Delete.
    pub fn delete(&mut self, node_id: Option<u64>) -> io::Result<ActionResponse> {
        self.key(None, node_id, "Delete")
    }

    /// Select all text via platform modifier (Ctrl+A on Linux/Windows, Cmd+A on macOS).
    pub fn select_all(&mut self, node_id: Option<u64>) -> io::Result<ActionResponse> {
        self.key_with_modifiers(None, node_id, "a", Some(KeyModifiers::action_modifier()))
    }

    /// Move pointer / mouse to a target node or specific coordinates with optional modifiers.
    pub fn mouse_move(
        &mut self,
        window_id: Option<u64>,
        node_id: Option<u64>,
        coords: Option<(f32, f32)>,
        modifiers: Option<KeyModifiers>,
    ) -> io::Result<ActionResponse> {
        let (x, y) = match coords {
            Some((cx, cy)) => (Some(cx), Some(cy)),
            None => (None, None),
        };
        self.act(ActionRequest::MouseMove {
            window_id,
            node_id,
            x,
            y,
            modifiers,
        })
    }

    /// Hover over a target node on the primary window.
    pub fn hover(&mut self, node_id: u64) -> io::Result<ActionResponse> {
        self.mouse_move(None, Some(node_id), None, None)
    }

    /// Move pointer to a target node on the primary window.
    pub fn move_to(&mut self, node_id: u64) -> io::Result<ActionResponse> {
        self.mouse_move(None, Some(node_id), None, None)
    }

    /// Move pointer to explicit viewport coordinates on the primary window.
    pub fn move_to_coords(&mut self, x: f32, y: f32) -> io::Result<ActionResponse> {
        self.mouse_move(None, None, Some((x, y)), None)
    }

    /// Press mouse button down on a target node or coordinates.
    pub fn mouse_down(
        &mut self,
        window_id: Option<u64>,
        node_id: Option<u64>,
        coords: Option<(f32, f32)>,
        button: Option<&str>,
        modifiers: Option<KeyModifiers>,
    ) -> io::Result<ActionResponse> {
        let (x, y) = match coords {
            Some((cx, cy)) => (Some(cx), Some(cy)),
            None => (None, None),
        };
        self.act(ActionRequest::MouseDown {
            window_id,
            node_id,
            x,
            y,
            button: button.map(|s| s.to_string()),
            modifiers,
        })
    }

    /// Release mouse button on a target node or coordinates.
    pub fn mouse_up(
        &mut self,
        window_id: Option<u64>,
        node_id: Option<u64>,
        coords: Option<(f32, f32)>,
        button: Option<&str>,
        modifiers: Option<KeyModifiers>,
    ) -> io::Result<ActionResponse> {
        let (x, y) = match coords {
            Some((cx, cy)) => (Some(cx), Some(cy)),
            None => (None, None),
        };
        self.act(ActionRequest::MouseUp {
            window_id,
            node_id,
            x,
            y,
            button: button.map(|s| s.to_string()),
            modifiers,
        })
    }

    /// Execute a drag sequence from `from_node_id` to `to_node_id` by composing move -> down -> move -> up.
    pub fn drag(&mut self, from_node_id: u64, to_node_id: u64) -> io::Result<ActionResponse> {
        self.mouse_move(None, Some(from_node_id), None, None)?;
        self.mouse_down(None, Some(from_node_id), None, Some("left"), None)?;
        self.mouse_move(None, Some(to_node_id), None, None)?;
        self.mouse_up(None, Some(to_node_id), None, Some("left"), None)
    }

    /// Dispatch mouse wheel / scroll event on a target node or coordinates.
    pub fn wheel(
        &mut self,
        window_id: Option<u64>,
        node_id: Option<u64>,
        coords: Option<(f32, f32)>,
        delta_x: f64,
        delta_y: f64,
        modifiers: Option<KeyModifiers>,
    ) -> io::Result<ActionResponse> {
        let (x, y) = match coords {
            Some((cx, cy)) => (Some(cx), Some(cy)),
            None => (None, None),
        };
        self.act(ActionRequest::Wheel {
            window_id,
            node_id,
            x,
            y,
            delta_x,
            delta_y,
            modifiers,
        })
    }

    /// Scroll a target node by vertical delta on the primary window.
    pub fn scroll(&mut self, node_id: u64, delta_y: f64) -> io::Result<ActionResponse> {
        self.wheel(None, Some(node_id), None, 0.0, delta_y, None)
    }

    /// Synchronize execution by waiting for `frames` VSync / render ticks on the default window.
    pub fn settle(&mut self, frames: u32) -> io::Result<SettleResponse> {
        self.settle_window(None, frames)
    }

    /// Synchronize execution by waiting for `frames` VSync / render ticks on a targeted window.
    pub fn settle_window(&mut self, window_id: Option<u64>, frames: u32) -> io::Result<SettleResponse> {
        match self.send_request(ControlRequest::Settle(SettleRequest { window_id, frames }))? {
            ControlResponse::SettleSuccess(settle_resp) => Ok(settle_resp),
            ControlResponse::Error(err) => Err(io::Error::other(err)),
            other => Err(io::Error::other(format!(
                "Unexpected response variant for settle: {other:?}"
            ))),
        }
    }

    fn resolve_output_path(path: impl AsRef<Path>) -> String {
        let p = path.as_ref();
        if p.is_absolute() {
            p.to_string_lossy().to_string()
        } else {
            match std::env::current_dir() {
                Ok(cwd) => cwd.join(p).to_string_lossy().to_string(),
                Err(_) => p.to_string_lossy().to_string(),
            }
        }
    }

    /// Capture visual screenshot of the document/window on the default/primary window,
    /// writing the PNG artifact directly to `output_path`.
    pub fn capture(&mut self, output_path: impl AsRef<Path>) -> io::Result<CaptureResponse> {
        self.capture_window(None, output_path)
    }

    /// Capture visual screenshot with explicit [`CaptureRequest`] parameters.
    pub fn capture_request(&mut self, request: CaptureRequest) -> io::Result<CaptureResponse> {
        match self.send_request(ControlRequest::Capture(request))? {
            ControlResponse::CaptureSuccess(cap_resp) => Ok(cap_resp),
            ControlResponse::Error(err) => Err(io::Error::other(err)),
            other => Err(io::Error::other(format!(
                "Unexpected response variant for capture: {other:?}"
            ))),
        }
    }

    /// Capture visual screenshot of the document/window on a targeted window (or primary if None),
    /// writing the PNG artifact directly to `output_path`.
    pub fn capture_window(
        &mut self,
        window_id: Option<u64>,
        output_path: impl AsRef<Path>,
    ) -> io::Result<CaptureResponse> {
        let resolved = Self::resolve_output_path(output_path);
        self.capture_request(CaptureRequest {
            window_id,
            node_id: None,
            output_path: resolved,
        })
    }

    /// Capture visual screenshot cropped to a specific target node's visual bounds,
    /// writing the PNG artifact directly to `output_path`.
    pub fn capture_node(
        &mut self,
        node_id: u64,
        output_path: impl AsRef<Path>,
    ) -> io::Result<CaptureResponse> {
        let resolved = Self::resolve_output_path(output_path);
        self.capture_request(CaptureRequest {
            window_id: None,
            node_id: Some(node_id),
            output_path: resolved,
        })
    }

    /// Capture visual screenshot of a specific node on a targeted window,
    /// writing the PNG artifact directly to `output_path`.
    pub fn capture_window_node(
        &mut self,
        window_id: Option<u64>,
        node_id: Option<u64>,
        output_path: impl AsRef<Path>,
    ) -> io::Result<CaptureResponse> {
        let resolved = Self::resolve_output_path(output_path);
        self.capture_request(CaptureRequest {
            window_id,
            node_id,
            output_path: resolved,
        })
    }

    /// Capture visual screenshot and write directly to `path`.
    ///
    /// Returns `(width, height, path)` on success.
    pub fn capture_to_file(&mut self, path: impl AsRef<Path>) -> io::Result<(u32, u32, PathBuf)> {
        let resp = self.capture(&path)?;
        if !resp.success {
            return Err(io::Error::other(
                resp.message.unwrap_or_else(|| "Capture failed without message".into()),
            ));
        }
        Ok((resp.width, resp.height, PathBuf::from(resp.file_path)))
    }

    /// Capture visual screenshot of a specific node and write directly to `path`.
    ///
    /// Returns `(width, height, path)` on success.
    pub fn capture_node_to_file(
        &mut self,
        node_id: u64,
        path: impl AsRef<Path>,
    ) -> io::Result<(u32, u32, PathBuf)> {
        let resp = self.capture_node(node_id, &path)?;
        if !resp.success {
            return Err(io::Error::other(
                resp.message
                    .unwrap_or_else(|| format!("Capture of node #{node_id} failed without message")),
            ));
        }
        Ok((resp.width, resp.height, PathBuf::from(resp.file_path)))
    }

    /// Settle and inspect iteratively until `condition` is satisfied or `timeout` expires.
    ///
    /// This proves state change rather than synthetic delay: it advances frame by frame
    /// and returns as soon as the target state is observed in the semantic tree.
    pub fn settle_until<F>(
        &mut self,
        timeout: Duration,
        mut condition: F,
    ) -> io::Result<InspectResponse>
    where
        F: FnMut(&InspectResponse) -> bool,
    {
        let start = Instant::now();
        loop {
            let snapshot = self.inspect(InspectRequest::default())?;
            if condition(&snapshot) {
                return Ok(snapshot);
            }
            if start.elapsed() >= timeout {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("Condition not satisfied within {timeout:?}"),
                ));
            }
            // Settle 1 frame before probing again
            let _ = self.settle(1)?;
        }
    }
}
