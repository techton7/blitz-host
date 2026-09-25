//! # blitz-host-protocol
//!
//! Typed agent-control protocol vocabulary for live Blitz window inspection.
//!
//! Designed as an inert, lightweight data crate with zero heavy dependencies (no Blitz, no Winit, no Tokio).
//! Both the host server and external agents/test harnesses share this vocabulary.

use serde::{Deserialize, Serialize};

/// Current protocol wire version.
pub const PROTOCOL_VERSION: u32 = 1;

/// Ephemeral discovery descriptor published by a running host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostDescriptor {
    /// Protocol version spoken by the host.
    pub protocol_version: u32,
    /// Operating system process ID of the running host.
    pub pid: u32,
    /// Unique instance identifier for this session.
    pub instance_id: String,
    /// Path to the local Unix domain socket.
    pub socket_path: String,
    /// Host renderer identifier (e.g. "blitz").
    pub renderer: String,
    /// Host renderer version or build revision.
    pub renderer_version: String,
    /// Primary window handle if exposed by the host.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_window_id: Option<u64>,
    /// Primary document identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_document_id: Option<usize>,
}

/// Target specifier for an element: either a direct ephemeral node ID or a CSS selector string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ElementTarget {
    /// Ephemeral Blitz node ID.
    Id(u64),
    /// CSS selector string (e.g. "#test-input", "button.primary", "input[type='text']").
    Selector(String),
}

impl std::fmt::Display for ElementTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Id(id) => write!(f, "#{id}"),
            Self::Selector(sel) => write!(f, "{sel}"),
        }
    }
}

impl From<u64> for ElementTarget {
    fn from(id: u64) -> Self {
        Self::Id(id)
    }
}

impl From<&str> for ElementTarget {
    fn from(s: &str) -> Self {
        let trimmed = s.trim();
        if let Ok(id) = trimmed.parse::<u64>() {
            Self::Id(id)
        } else {
            Self::Selector(trimmed.to_string())
        }
    }
}

impl From<String> for ElementTarget {
    fn from(s: String) -> Self {
        let trimmed = s.trim();
        if let Ok(id) = trimmed.parse::<u64>() {
            Self::Id(id)
        } else {
            Self::Selector(trimmed.to_string())
        }
    }
}

/// Request to inspect the running host's DOM / semantic tree.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectRequest {
    /// Optional target window handle (falls back to primary window if None).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_id: Option<u64>,
    /// Optional starting root node id. If `None`, inspection starts at document root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_node_id: Option<u64>,
    /// Optional CSS selector to target the root node of the inspection subtree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    /// Optional polymorphic element target (node ID or CSS selector).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<ElementTarget>,
    /// Optional maximum traversal depth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
}

impl InspectRequest {
    /// Resolves the intended target if specified either via `target`, `selector`, or `root_node_id`.
    pub fn target(&self) -> Option<ElementTarget> {
        if let Some(ref t) = self.target {
            Some(t.clone())
        } else if let Some(ref s) = self.selector {
            Some(ElementTarget::from(s.as_str()))
        } else {
            self.root_node_id.map(ElementTarget::Id)
        }
    }
}

/// A node in the inspected semantic tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticNode {
    /// Ephemeral Blitz node handle.
    pub id: u64,
    /// Parent node handle if not the root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<u64>,
    /// HTML tag name (e.g. "div", "button", "h1") or node kind ("#text", "#document").
    pub tag: String,
    /// Authored HTML `id` attribute (e.g. `id="submit-btn"`), if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dom_id: Option<String>,
    /// Accessible WAI-ARIA role (e.g. "button", "heading"), if specified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Textual content for text nodes or labeled elements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Layout border box [x, y, width, height] in CSS pixels, if computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds: Option<[f32; 4]>,
    /// Whether this node currently holds active keyboard focus.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused: Option<bool>,
    /// Whether this node currently has mouse hover state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hovered: Option<bool>,
    /// Whether this node currently has active (pressed) state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    /// Scroll offset [x, y] in CSS pixels, if scrolled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scroll_offset: Option<[f64; 2]>,
    /// Ordered list of child node IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<u64>,
}

/// Response returned from an inspect query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectResponse {
    /// Document identifier assigned by the host runtime.
    pub document_id: usize,
    /// Root node ID of the inspected subtree.
    pub root_id: u64,
    /// Total number of nodes included in this snapshot.
    pub node_count: usize,
    /// Optional current frame counter observed when the snapshot was taken.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_frame: Option<u64>,
    /// Optional node ID of the currently focused element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_node_id: Option<u64>,
    /// Optional node ID of the currently hovered element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover_node_id: Option<u64>,
    /// Document viewport scroll offset [x, y] in CSS pixels, if non-zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewport_scroll: Option<[f64; 2]>,
    /// Flattened pre-order list of semantic nodes.
    pub nodes: Vec<SemanticNode>,
    /// Optional status or failure message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Request to perform an action on a target node in the host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum ActionRequest {
    /// Synthetic click on a node or selector.
    Click {
        /// Optional target window handle (falls back to primary window if None).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        window_id: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        node_id: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selector: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<ElementTarget>,
    },
    /// Focus a target node or selector.
    Focus {
        /// Optional target window handle (falls back to primary window if None).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        window_id: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        node_id: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selector: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<ElementTarget>,
    },
    /// Set the text/input value of an editable node or selector.
    SetValue {
        /// Optional target window handle (falls back to primary window if None).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        window_id: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        node_id: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selector: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<ElementTarget>,
        value: String,
    },
    /// Inject a keypress action.
    Key {
        /// Optional target window handle (falls back to primary window if None).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        window_id: Option<u64>,
        /// Optional target node handle (falls back to currently focused element or root if None).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        node_id: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selector: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<ElementTarget>,
        /// Key name or character to inject (e.g. "Tab", "Enter", "Space", "Backspace", "a").
        key: String,
        /// Modifier keys held during the keypress.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        modifiers: Option<KeyModifiers>,
    },
    /// Pointer / Mouse move (hover).
    MouseMove {
        /// Optional target window handle (falls back to primary window if None).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        window_id: Option<u64>,
        /// Optional target node handle (its center is used if x/y are omitted).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        node_id: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selector: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<ElementTarget>,
        /// Explicit X coordinate in viewport CSS pixels.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x: Option<f32>,
        /// Explicit Y coordinate in viewport CSS pixels.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y: Option<f32>,
        /// Modifier keys held during the move.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        modifiers: Option<KeyModifiers>,
    },
    /// Pointer / Mouse button press (down).
    MouseDown {
        /// Optional target window handle (falls back to primary window if None).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        window_id: Option<u64>,
        /// Optional target node handle.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        node_id: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selector: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<ElementTarget>,
        /// Explicit X coordinate in viewport CSS pixels.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x: Option<f32>,
        /// Explicit Y coordinate in viewport CSS pixels.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y: Option<f32>,
        /// Button: "left", "right", "middle" (defaults to "left").
        #[serde(default, skip_serializing_if = "Option::is_none")]
        button: Option<String>,
        /// Modifier keys held during the press.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        modifiers: Option<KeyModifiers>,
    },
    /// Pointer / Mouse button release (up).
    MouseUp {
        /// Optional target window handle (falls back to primary window if None).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        window_id: Option<u64>,
        /// Optional target node handle.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        node_id: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selector: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<ElementTarget>,
        /// Explicit X coordinate in viewport CSS pixels.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x: Option<f32>,
        /// Explicit Y coordinate in viewport CSS pixels.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y: Option<f32>,
        /// Button: "left", "right", "middle" (defaults to "left").
        #[serde(default, skip_serializing_if = "Option::is_none")]
        button: Option<String>,
        /// Modifier keys held during the release.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        modifiers: Option<KeyModifiers>,
    },
    /// Mouse wheel / scroll.
    Wheel {
        /// Optional target window handle (falls back to primary window if None).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        window_id: Option<u64>,
        /// Optional target node handle (its center is used if x/y are omitted).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        node_id: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selector: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<ElementTarget>,
        /// Explicit X coordinate in viewport CSS pixels.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x: Option<f32>,
        /// Explicit Y coordinate in viewport CSS pixels.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y: Option<f32>,
        /// Horizontal scroll delta in pixels.
        #[serde(default, alias = "deltaX")]
        delta_x: f64,
        /// Vertical scroll delta in pixels.
        #[serde(default, alias = "deltaY")]
        delta_y: f64,
        /// Modifier keys held during the scroll.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        modifiers: Option<KeyModifiers>,
    },
}

impl ActionRequest {
    /// Resolves the intended target if specified either via `target`, `selector`, or `node_id`.
    pub fn target(&self) -> Option<ElementTarget> {
        match self {
            Self::Click { target, selector, node_id, .. }
            | Self::Focus { target, selector, node_id, .. }
            | Self::SetValue { target, selector, node_id, .. }
            | Self::Key { target, selector, node_id, .. }
            | Self::MouseMove { target, selector, node_id, .. }
            | Self::MouseDown { target, selector, node_id, .. }
            | Self::MouseUp { target, selector, node_id, .. }
            | Self::Wheel { target, selector, node_id, .. } => {
                if let Some(t) = target {
                    Some(t.clone())
                } else if let Some(s) = selector {
                    Some(ElementTarget::from(s.as_str()))
                } else {
                    node_id.map(ElementTarget::Id)
                }
            }
        }
    }

    /// Access raw `node_id` field if directly specified.
    pub fn raw_node_id(&self) -> Option<u64> {
        match self {
            Self::Click { node_id, .. }
            | Self::Focus { node_id, .. }
            | Self::SetValue { node_id, .. }
            | Self::Key { node_id, .. }
            | Self::MouseMove { node_id, .. }
            | Self::MouseDown { node_id, .. }
            | Self::MouseUp { node_id, .. }
            | Self::Wheel { node_id, .. } => *node_id,
        }
    }

    /// Access raw `selector` field if directly specified.
    pub fn raw_selector(&self) -> Option<&str> {
        match self {
            Self::Click { selector, .. }
            | Self::Focus { selector, .. }
            | Self::SetValue { selector, .. }
            | Self::Key { selector, .. }
            | Self::MouseMove { selector, .. }
            | Self::MouseDown { selector, .. }
            | Self::MouseUp { selector, .. }
            | Self::Wheel { selector, .. } => selector.as_deref(),
        }
    }
}

/// Modifier keys held during a key action.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyModifiers {
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub meta: bool,
}

impl KeyModifiers {
    pub const NONE: Self = Self {
        shift: false,
        ctrl: false,
        alt: false,
        meta: false,
    };
    pub const SHIFT: Self = Self {
        shift: true,
        ctrl: false,
        alt: false,
        meta: false,
    };
    pub const CTRL: Self = Self {
        shift: false,
        ctrl: true,
        alt: false,
        meta: false,
    };
    pub const ALT: Self = Self {
        shift: false,
        ctrl: false,
        alt: true,
        meta: false,
    };
    pub const META: Self = Self {
        shift: false,
        ctrl: false,
        alt: false,
        meta: true,
    };

    /// Platform-appropriate action modifier (Cmd on macOS, Ctrl on Windows/Linux).
    pub fn action_modifier() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::META
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self::CTRL
        }
    }

    pub fn is_empty(&self) -> bool {
        !self.shift && !self.ctrl && !self.alt && !self.meta
    }
}

/// Result of executing an action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResponse {
    /// Whether the action was successfully dispatched.
    pub success: bool,
    /// Target node ID that received the action.
    pub node_id: u64,
    /// Whether an active event listener intercepted and handled this action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handled: Option<bool>,
    /// Optional status or failure message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Request to wait for a number of host frames to settle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettleRequest {
    /// Optional target window handle (falls back to primary window if None).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_id: Option<u64>,
    /// Number of render/animation frames to wait.
    pub frames: u32,
}

/// Result of settling for requested frames.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettleResponse {
    /// Whether the requested frames were successfully observed.
    pub settled: bool,
    /// Number of frames actually waited.
    pub frames_waited: u32,
    /// Current frame counter upon completion.
    pub current_frame: u64,
}

/// Request to capture a visual screenshot and write the artifact to disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRequest {
    /// Optional target window handle (falls back to primary window if None).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_id: Option<u64>,
    /// Optional target node handle (if specified, crops the capture to this node's visual bounds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<u64>,
    /// Optional CSS selector to crop the capture to the matching element's visual bounds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    /// Optional polymorphic element target (node ID or CSS selector).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<ElementTarget>,
    /// Target file path where the captured image artifact must be written.
    pub output_path: String,
}

impl CaptureRequest {
    /// Resolves the intended target if specified either via `target`, `selector`, or `node_id`.
    pub fn target(&self) -> Option<ElementTarget> {
        if let Some(ref t) = self.target {
            Some(t.clone())
        } else if let Some(ref s) = self.selector {
            Some(ElementTarget::from(s.as_str()))
        } else {
            self.node_id.map(ElementTarget::Id)
        }
    }
}

/// Metadata result of capturing a visual screenshot to a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResponse {
    /// Whether capture was successful.
    pub success: bool,
    /// Absolute or relative path to the saved PNG file on disk.
    pub file_path: String,
    /// Rendered image width in physical pixels.
    pub width: u32,
    /// Rendered image height in physical pixels.
    pub height: u32,
    /// Image format (e.g. "png").
    pub format: String,
    /// Target node ID that was cropped, if requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<u64>,
    /// Number of PNG bytes written to disk.
    pub bytes: usize,
    /// Optional status or failure message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Type alias for backward compatibility.
pub type CaptureMetadataResponse = CaptureResponse;

/// Top-level control request envelope forwarded across the transport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum ControlRequest {
    /// Inspect the DOM/semantic tree.
    Inspect(InspectRequest),
    /// Execute a UI action on the host.
    Act(ActionRequest),
    /// Settle execution for a specific number of frames.
    Settle(SettleRequest),
    /// Capture a visual screenshot of the document/window.
    Capture(CaptureRequest),
}

impl ControlRequest {
    /// Resolves the intended target if specified in the payload.
    pub fn target(&self) -> Option<ElementTarget> {
        match self {
            Self::Inspect(req) => req.target(),
            Self::Act(req) => req.target(),
            Self::Capture(req) => req.target(),
            Self::Settle(_) => None,
        }
    }
}

/// Top-level control response envelope sent back across the transport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", content = "data", rename_all = "camelCase")]
pub enum ControlResponse {
    /// Successful execution returning inspect data (status: "inspectSuccess" or legacy "success").
    #[serde(rename = "inspectSuccess", alias = "success")]
    InspectSuccess(InspectResponse),
    /// Successful execution of an action.
    ActionSuccess(ActionResponse),
    /// Successful settlement of frames.
    SettleSuccess(SettleResponse),
    /// Successful visual capture.
    CaptureSuccess(CaptureResponse),
    /// Error encountered during request processing.
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_serde_roundtrip() {
        let desc = HostDescriptor {
            protocol_version: PROTOCOL_VERSION,
            pid: 12345,
            instance_id: "test-instance".into(),
            socket_path: "/tmp/blitz-host/test.sock".into(),
            renderer: "blitz".into(),
            renderer_version: "0.1.0".into(),
            primary_window_id: Some(100),
            primary_document_id: Some(1),
        };

        let json = serde_json::to_string_pretty(&desc).unwrap();
        let parsed: HostDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(desc, parsed);
    }

    #[test]
    fn test_control_envelope_serde_roundtrip() {
        let req = ControlRequest::Inspect(InspectRequest {
            window_id: None,
            root_node_id: Some(10),
            selector: None,
            target: None,
            max_depth: Some(5),
        });

        let json = serde_json::to_string(&req).unwrap();
        let parsed: ControlRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, parsed);

        let node = SemanticNode {
            id: 1,
            parent_id: None,
            tag: "div".into(),
            dom_id: Some("root-app".into()),
            role: None,
            text: None,
            bounds: Some([0.0, 0.0, 800.0, 600.0]),
            focused: Some(true),
            hovered: Some(true),
            active: Some(false),
            scroll_offset: Some([0.0, 150.0]),
            children: vec![2, 3],
        };

        let resp = ControlResponse::InspectSuccess(InspectResponse {
            document_id: 1,
            root_id: 1,
            node_count: 1,
            current_frame: Some(42),
            focused_node_id: Some(1),
            hover_node_id: Some(1),
            viewport_scroll: Some([0.0, 50.0]),
            nodes: vec![node],
            message: None,
        });

        let json = serde_json::to_string(&resp).unwrap();
        let parsed: ControlResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, parsed);

        // Test Action roundtrip: Click (node_id)
        let act_req = ControlRequest::Act(ActionRequest::Click {
            window_id: None,
            node_id: Some(42),
            selector: None,
            target: None,
        });
        let act_json = serde_json::to_string(&act_req).unwrap();
        let act_parsed: ControlRequest = serde_json::from_str(&act_json).unwrap();
        assert_eq!(act_req, act_parsed);

        // Test Action roundtrip: Click (selector)
        let sel_click_req = ControlRequest::Act(ActionRequest::Click {
            window_id: None,
            node_id: None,
            selector: Some("#test-interaction-button".into()),
            target: None,
        });
        let sel_click_json = serde_json::to_string(&sel_click_req).unwrap();
        assert!(sel_click_json.contains("\"selector\":\"#test-interaction-button\""));
        let sel_click_parsed: ControlRequest = serde_json::from_str(&sel_click_json).unwrap();
        assert_eq!(sel_click_req, sel_click_parsed);
        assert_eq!(
            sel_click_req.target(),
            Some(ElementTarget::Selector("#test-interaction-button".into()))
        );

        // Test Action roundtrip: Focus
        let focus_req = ControlRequest::Act(ActionRequest::Focus {
            window_id: Some(10),
            node_id: Some(42),
            selector: None,
            target: None,
        });
        let focus_json = serde_json::to_string(&focus_req).unwrap();
        assert!(focus_json.contains("\"action\":\"focus\""));
        let focus_parsed: ControlRequest = serde_json::from_str(&focus_json).unwrap();
        assert_eq!(focus_req, focus_parsed);

        // Test Action roundtrip: SetValue
        let set_value_req = ControlRequest::Act(ActionRequest::SetValue {
            window_id: None,
            node_id: Some(42),
            selector: None,
            target: None,
            value: "Hello World".into(),
        });
        let set_value_json = serde_json::to_string(&set_value_req).unwrap();
        assert!(set_value_json.contains("\"action\":\"setValue\""));
        assert!(set_value_json.contains("\"value\":\"Hello World\""));
        let set_value_parsed: ControlRequest = serde_json::from_str(&set_value_json).unwrap();
        assert_eq!(set_value_req, set_value_parsed);

        // Test InspectRequest with selector
        let sel_inspect_req = ControlRequest::Inspect(InspectRequest {
            window_id: None,
            root_node_id: None,
            selector: Some("#test-input".into()),
            target: None,
            max_depth: Some(3),
        });
        let sel_inspect_json = serde_json::to_string(&sel_inspect_req).unwrap();
        assert!(sel_inspect_json.contains("\"selector\":\"#test-input\""));
        let sel_inspect_parsed: ControlRequest = serde_json::from_str(&sel_inspect_json).unwrap();
        assert_eq!(sel_inspect_req, sel_inspect_parsed);
        assert_eq!(
            sel_inspect_req.target(),
            Some(ElementTarget::Selector("#test-input".into()))
        );

        let act_resp = ControlResponse::ActionSuccess(ActionResponse {
            success: true,
            node_id: 42,
            handled: Some(true),
            message: Some("clicked".into()),
        });
        let act_resp_json = serde_json::to_string(&act_resp).unwrap();
        assert!(act_resp_json.contains("\"handled\":true"));
        let act_resp_parsed: ControlResponse = serde_json::from_str(&act_resp_json).unwrap();
        assert_eq!(act_resp, act_resp_parsed);

        // Test backward compatibility without handled field
        let legacy_json = r#"{"status":"actionSuccess","data":{"success":true,"nodeId":42,"message":"legacy"}}"#;
        let legacy_resp: ControlResponse = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(
            legacy_resp,
            ControlResponse::ActionSuccess(ActionResponse {
                success: true,
                node_id: 42,
                handled: None,
                message: Some("legacy".into()),
            })
        );

        // Test Settle roundtrip
        let settle_req = ControlRequest::Settle(SettleRequest {
            window_id: None,
            frames: 2,
        });
        let settle_json = serde_json::to_string(&settle_req).unwrap();
        let settle_parsed: ControlRequest = serde_json::from_str(&settle_json).unwrap();
        assert_eq!(settle_req, settle_parsed);

        let settle_resp = ControlResponse::SettleSuccess(SettleResponse {
            settled: true,
            frames_waited: 2,
            current_frame: 44,
        });
        let settle_resp_json = serde_json::to_string(&settle_resp).unwrap();
        let settle_resp_parsed: ControlResponse = serde_json::from_str(&settle_resp_json).unwrap();
        assert_eq!(settle_resp, settle_resp_parsed);

        // Test Capture roundtrip
        let cap_req = ControlRequest::Capture(CaptureRequest {
            window_id: Some(99),
            node_id: Some(42),
            selector: None,
            target: None,
            output_path: "target/test.png".into(),
        });
        let cap_req_json = serde_json::to_string(&cap_req).unwrap();
        assert!(cap_req_json.contains("\"type\":\"capture\""));
        assert!(cap_req_json.contains("\"nodeId\":42"));
        assert!(cap_req_json.contains("\"outputPath\":\"target/test.png\""));
        let cap_req_parsed: ControlRequest = serde_json::from_str(&cap_req_json).unwrap();
        assert_eq!(cap_req, cap_req_parsed);

        // Test Action roundtrip: Key
        let key_req = ControlRequest::Act(ActionRequest::Key {
            window_id: None,
            node_id: Some(101),
            selector: None,
            target: None,
            key: "Tab".into(),
            modifiers: Some(KeyModifiers::SHIFT),
        });
        let key_json = serde_json::to_string(&key_req).unwrap();
        assert!(key_json.contains("\"action\":\"key\""));
        assert!(key_json.contains("\"key\":\"Tab\""));
        assert!(key_json.contains("\"shift\":true"));
        let key_parsed: ControlRequest = serde_json::from_str(&key_json).unwrap();
        assert_eq!(key_req, key_parsed);

        // Test Action roundtrip: Key with modifier sentinel (Cmd/Ctrl + A)
        let sentinel_req = ControlRequest::Act(ActionRequest::Key {
            window_id: None,
            node_id: None,
            selector: None,
            target: None,
            key: "a".into(),
            modifiers: Some(KeyModifiers::action_modifier()),
        });
        let sentinel_json = serde_json::to_string(&sentinel_req).unwrap();
        let sentinel_parsed: ControlRequest = serde_json::from_str(&sentinel_json).unwrap();
        assert_eq!(sentinel_req, sentinel_parsed);

        // Test Action roundtrip: MouseMove
        let move_req = ControlRequest::Act(ActionRequest::MouseMove {
            window_id: None,
            node_id: Some(102),
            selector: None,
            target: None,
            x: Some(150.0),
            y: Some(250.0),
            modifiers: None,
        });
        let move_json = serde_json::to_string(&move_req).unwrap();
        assert!(move_json.contains("\"action\":\"mouseMove\""));
        assert!(move_json.contains("\"x\":150.0"));
        let move_parsed: ControlRequest = serde_json::from_str(&move_json).unwrap();
        assert_eq!(move_req, move_parsed);

        // Test Action roundtrip: MouseDown
        let down_req = ControlRequest::Act(ActionRequest::MouseDown {
            window_id: None,
            node_id: Some(102),
            selector: None,
            target: None,
            x: None,
            y: None,
            button: Some("left".into()),
            modifiers: None,
        });
        let down_json = serde_json::to_string(&down_req).unwrap();
        assert!(down_json.contains("\"action\":\"mouseDown\""));
        assert!(down_json.contains("\"button\":\"left\""));
        let down_parsed: ControlRequest = serde_json::from_str(&down_json).unwrap();
        assert_eq!(down_req, down_parsed);

        // Test Action roundtrip: MouseUp
        let up_req = ControlRequest::Act(ActionRequest::MouseUp {
            window_id: None,
            node_id: Some(102),
            selector: None,
            target: None,
            x: None,
            y: None,
            button: Some("left".into()),
            modifiers: None,
        });
        let up_json = serde_json::to_string(&up_req).unwrap();
        assert!(up_json.contains("\"action\":\"mouseUp\""));
        let up_parsed: ControlRequest = serde_json::from_str(&up_json).unwrap();
        assert_eq!(up_req, up_parsed);

        // Test Action roundtrip: Wheel
        let wheel_req = ControlRequest::Act(ActionRequest::Wheel {
            window_id: None,
            node_id: Some(103),
            selector: None,
            target: None,
            x: None,
            y: None,
            delta_x: 0.0,
            delta_y: 50.0,
            modifiers: None,
        });
        let wheel_json = serde_json::to_string(&wheel_req).unwrap();
        assert!(wheel_json.contains("\"action\":\"wheel\""));
        assert!(wheel_json.contains("\"delta_y\":50.0"));
        let wheel_parsed: ControlRequest = serde_json::from_str(&wheel_json).unwrap();
        assert_eq!(wheel_req, wheel_parsed);

        // Test camelCase alias for deltaX/deltaY
        let camel_json = r#"{"type":"act","payload":{"action":"wheel","node_id":103,"deltaX":10.0,"deltaY":20.0}}"#;
        let camel_parsed: ControlRequest = serde_json::from_str(camel_json).unwrap();
        assert_eq!(
            camel_parsed,
            ControlRequest::Act(ActionRequest::Wheel {
                window_id: None,
                node_id: Some(103),
                selector: None,
                target: None,
                x: None,
                y: None,
                delta_x: 10.0,
                delta_y: 20.0,
                modifiers: None,
            })
        );

        let cap_resp = ControlResponse::CaptureSuccess(CaptureResponse {
            success: true,
            file_path: "target/screenshot.png".into(),
            width: 800,
            height: 600,
            format: "png".into(),
            node_id: None,
            bytes: 12345,
            message: Some("Screenshot captured".into()),
        });
        let cap_resp_json = serde_json::to_string(&cap_resp).unwrap();
        assert!(cap_resp_json.contains("\"status\":\"captureSuccess\""));
        assert!(cap_resp_json.contains("\"filePath\":\"target/screenshot.png\""));
        assert!(cap_resp_json.contains("\"bytes\":12345"));
        assert!(!cap_resp_json.contains("dataBase64"));
        let cap_resp_parsed: ControlResponse = serde_json::from_str(&cap_resp_json).unwrap();
        assert_eq!(cap_resp, cap_resp_parsed);
    }
}
