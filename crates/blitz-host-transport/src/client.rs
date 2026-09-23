use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

use blitz_host_protocol::{
    ActionRequest, ActionResponse, ControlRequest, ControlResponse, HostDescriptor,
    InspectRequest, InspectResponse, SettleRequest, SettleResponse,
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
