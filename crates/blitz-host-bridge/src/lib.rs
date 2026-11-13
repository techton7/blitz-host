//! # blitz-host-bridge
//!
//! Runtime host bridge connecting a real running Blitz window and `blitz_dom::BaseDocument`
//! to the blitz-host control plane.

pub mod bridge;
pub mod capture;
pub mod inspect;
pub mod target;

pub use bridge::{HostBridge, ServiceResult};
pub use capture::{capture_document, capture_document_node_png, capture_document_png};
pub use inspect::inspect_document;
pub use target::resolve_target_in_doc;

#[cfg(test)]
mod tests {
    use super::*;
    use blitz_dom::{BaseDocument, DocumentConfig};
    use blitz_host_protocol::InspectRequest;

    #[test]
    fn test_inspect_document_minimal() {
        let doc = BaseDocument::new(DocumentConfig::default());
        let response = inspect_document(&doc, InspectRequest::default());

        assert_eq!(response.document_id, doc.id());
        assert_eq!(response.root_id, doc.root_node().id.as_u64());
        assert!(response.node_count >= 1);
        assert_eq!(response.nodes[0].tag, "#document");
    }

    #[test]
    fn test_bridge_focus_and_set_value_actions() {
        use blitz_host_protocol::{ActionRequest, ControlRequest, ControlResponse};
        use blitz_host_transport::ControlBridgeRequest;
        use std::sync::mpsc::{channel, sync_channel};

        let mut doc = BaseDocument::new(DocumentConfig::default());
        let root_id = doc.root_node().id;

        let (tx, rx) = channel::<ControlBridgeRequest>();
        let mut bridge = HostBridge::new(rx);

        // 1. Send Focus request
        let (focus_resp_tx, focus_resp_rx) = sync_channel(1);
        tx.send(ControlBridgeRequest {
            request: ControlRequest::Act(ActionRequest::Focus {
                window_id: None,
                node_id: Some(root_id.as_u64()),
                selector: None,
                target: None,
                focus_visible: None,
            }),
            reply: focus_resp_tx,
        })
        .unwrap();

        let serviced = bridge.poll_and_service_with(&mut doc, 1, |act, d| match act {
            ActionRequest::Focus { node_id, .. } => {
                let nid = node_id.unwrap();
                d.set_focus_to(blitz_dom::NodeId::from_u64(nid));
                Ok(blitz_host_protocol::ActionResponse {
                    success: true,
                    node_id: nid,
                    handled: None,
                    message: Some("focused".into()),
                })
            }
            _ => Err("unsupported".into()),
        });
        assert_eq!(serviced.responded, 1);
        assert_eq!(serviced.serviced(), 1);
        assert_eq!(serviced.pending_settles, 0);
        assert!(serviced.needs_redraw);
        let resp = focus_resp_rx.recv().unwrap();
        assert!(matches!(resp, ControlResponse::ActionSuccess(_)));

        // 2. Inspect document and verify focused state
        let inspected = inspect_document(&doc, InspectRequest::default());
        assert_eq!(inspected.focused_node_id, Some(root_id.as_u64()));
        assert_eq!(inspected.nodes[0].focused, Some(true));
    }

    #[test]
    fn test_bridge_capture_document() {
        use blitz_host_protocol::{CaptureRequest, ControlRequest, ControlResponse};
        use blitz_host_transport::ControlBridgeRequest;
        use std::sync::mpsc::{channel, sync_channel};

        let mut doc = BaseDocument::new(DocumentConfig::default());

        // Test direct helper
        let (width, height, png_bytes) =
            capture_document_png(&mut doc).expect("capture_document_png must succeed");
        assert!(width > 0 && height > 0);
        assert!(png_bytes.len() > 8);
        assert_eq!(
            &png_bytes[0..4],
            &[0x89, b'P', b'N', b'G'],
            "must start with PNG magic bytes"
        );

        // Test IPC bridge routing for full window capture
        let (tx, rx) = channel::<ControlBridgeRequest>();
        let mut bridge = HostBridge::new(rx);

        let test_output_file = "target/test_bridge_full.png";
        let _ = std::fs::remove_file(test_output_file);

        let (cap_resp_tx, cap_resp_rx) = sync_channel(1);
        tx.send(ControlBridgeRequest {
            request: ControlRequest::Capture(CaptureRequest {
                window_id: None,
                node_id: None,
                selector: None,
                target: None,
                output_path: test_output_file.to_string(),
            }),
            reply: cap_resp_tx,
        })
        .unwrap();

        let serviced = bridge.poll_and_service_with(&mut doc, 1, |_, _| Err("no actions".into()));
        assert_eq!(serviced.responded, 1);
        assert_eq!(serviced.serviced(), 1);
        assert_eq!(serviced.pending_settles, 0);
        assert!(!serviced.needs_redraw);

        let resp = cap_resp_rx.recv().unwrap();
        match resp {
            ControlResponse::CaptureSuccess(cap) => {
                assert!(cap.success);
                assert_eq!(cap.format, "png");
                assert_eq!(cap.file_path, test_output_file);
                assert_eq!(cap.width, width);
                assert_eq!(cap.height, height);
                assert_eq!(cap.node_id, None);
                assert_eq!(cap.bytes, png_bytes.len());
                let disk_bytes = std::fs::read(test_output_file).expect("file must exist on disk");
                assert_eq!(disk_bytes, png_bytes);
            }
            other => panic!("expected CaptureSuccess, got {other:?}"),
        }

        // Test IPC bridge routing for node-level capture (non-existent node should fail gracefully)
        let (node_resp_tx, node_resp_rx) = sync_channel(1);
        tx.send(ControlBridgeRequest {
            request: ControlRequest::Capture(CaptureRequest {
                window_id: None,
                node_id: Some(999999),
                selector: None,
                target: None,
                output_path: "target/test_bridge_nonexistent.png".to_string(),
            }),
            reply: node_resp_tx,
        })
        .unwrap();

        let serviced2 = bridge.poll_and_service_with(&mut doc, 1, |_, _| Err("no actions".into()));
        assert_eq!(serviced2.responded, 1);
        assert_eq!(serviced2.serviced(), 1);
        assert_eq!(serviced2.pending_settles, 0);
        assert!(!serviced2.needs_redraw);

        let resp2 = node_resp_rx.recv().unwrap();
        match resp2 {
            ControlResponse::CaptureSuccess(cap) => {
                assert!(
                    !cap.success,
                    "capture of non-existent node must report failure"
                );
                assert!(cap.message.unwrap().contains("not found in document"));
            }
            other => panic!("expected CaptureSuccess with failure status, got {other:?}"),
        }
    }

    #[test]
    fn test_settle_multi_frame_redraw_chaining() {
        use blitz_host_protocol::{ControlRequest, ControlResponse, SettleRequest};
        use blitz_host_transport::ControlBridgeRequest;
        use std::sync::mpsc::{channel, sync_channel};

        let mut doc = BaseDocument::new(DocumentConfig::default());
        let (tx, rx) = channel::<ControlBridgeRequest>();
        let mut bridge = HostBridge::new(rx);

        let (settle_resp_tx, settle_resp_rx) = sync_channel(1);
        tx.send(ControlBridgeRequest {
            request: ControlRequest::Settle(SettleRequest {
                window_id: None,
                frames: 3,
            }),
            reply: settle_resp_tx,
        })
        .unwrap();

        // Frame 0: Registers the 3-frame settle request (target_frame = 0 + 3 = 3)
        let res0 = bridge.poll_and_service_with(&mut doc, 0, |_, _| Err("no actions".into()));
        assert_eq!(res0.responded, 0);
        assert_eq!(res0.pending_settles, 1);
        assert!(res0.needs_redraw);

        // Frame 1: Returns needs_redraw = true while still awaiting future frames
        let res1 = bridge.poll_and_service_with(&mut doc, 1, |_, _| Err("no actions".into()));
        assert_eq!(res1.responded, 0);
        assert_eq!(res1.pending_settles, 1);
        assert!(res1.needs_redraw);

        // Frame 2: Still returns needs_redraw = true
        let res2 = bridge.poll_and_service_with(&mut doc, 2, |_, _| Err("no actions".into()));
        assert_eq!(res2.responded, 0);
        assert_eq!(res2.pending_settles, 1);
        assert!(res2.needs_redraw);

        // Frame 3: Resolves on frame 3 with responded = 1, pending_settles = 0, needs_redraw = false
        let res3 = bridge.poll_and_service_with(&mut doc, 3, |_, _| Err("no actions".into()));
        assert_eq!(res3.responded, 1);
        assert_eq!(res3.pending_settles, 0);
        assert!(!res3.needs_redraw);

        let resp = settle_resp_rx.recv().unwrap();
        match resp {
            ControlResponse::SettleSuccess(settle) => {
                assert!(settle.settled);
                assert_eq!(settle.frames_waited, 3);
                assert_eq!(settle.current_frame, 3);
            }
            other => panic!("expected SettleSuccess, got {other:?}"),
        }
    }

    #[test]
    fn test_act_triggers_needs_redraw() {
        use blitz_host_protocol::{ActionRequest, ActionResponse, ControlRequest, ControlResponse};
        use blitz_host_transport::ControlBridgeRequest;
        use std::sync::mpsc::{channel, sync_channel};

        let mut doc = BaseDocument::new(DocumentConfig::default());
        let root_id = doc.root_node().id;

        let (tx, rx) = channel::<ControlBridgeRequest>();
        let mut bridge = HostBridge::new(rx);

        let (act_resp_tx, act_resp_rx) = sync_channel(1);
        tx.send(ControlBridgeRequest {
            request: ControlRequest::Act(ActionRequest::Focus {
                window_id: None,
                node_id: Some(root_id.as_u64()),
                selector: None,
                target: None,
                focus_visible: None,
            }),
            reply: act_resp_tx,
        })
        .unwrap();

        let serviced = bridge.poll_and_service_with(&mut doc, 1, |act, d| match act {
            ActionRequest::Focus { node_id, .. } => {
                let nid = node_id.unwrap();
                d.set_focus_to(blitz_dom::NodeId::from_u64(nid));
                Ok(ActionResponse {
                    success: true,
                    node_id: nid,
                    handled: None,
                    message: Some("focused".into()),
                })
            }
            _ => Err("unsupported".into()),
        });

        assert_eq!(serviced.responded, 1);
        assert_eq!(serviced.serviced(), 1);
        assert_eq!(serviced.pending_settles, 0);
        assert!(serviced.needs_redraw, "Act request must set needs_redraw = true");

        let resp = act_resp_rx.recv().unwrap();
        assert!(matches!(resp, ControlResponse::ActionSuccess(_)));
    }

    #[test]
    fn test_inspect_does_not_need_redraw() {
        use blitz_host_protocol::{ControlRequest, ControlResponse};
        use blitz_host_transport::ControlBridgeRequest;
        use std::sync::mpsc::{channel, sync_channel};

        let mut doc = BaseDocument::new(DocumentConfig::default());
        let (tx, rx) = channel::<ControlBridgeRequest>();
        let mut bridge = HostBridge::new(rx);

        let (inspect_resp_tx, inspect_resp_rx) = sync_channel(1);
        tx.send(ControlBridgeRequest {
            request: ControlRequest::Inspect(InspectRequest::default()),
            reply: inspect_resp_tx,
        })
        .unwrap();

        let serviced = bridge.poll_and_service_with(&mut doc, 1, |_, _| Err("no actions".into()));
        assert_eq!(serviced.responded, 1);
        assert_eq!(serviced.serviced(), 1);
        assert_eq!(serviced.pending_settles, 0);
        assert!(!serviced.needs_redraw, "Inspect request must not require redraw");

        let resp = inspect_resp_rx.recv().unwrap();
        assert!(matches!(resp, ControlResponse::InspectSuccess(_)));
    }

    #[test]
    fn test_inspect_document_reports_focus_visible_modality() {
        use blitz_dom::{Attribute, FocusOptions, FocusSource, QualName, local_name, ns};

        let mut doc = BaseDocument::new(DocumentConfig::default());
        let root_id = doc.root_node().id;
        let btn_id = {
            let mut mutator = doc.mutate();
            let btn = mutator.create_element(
                QualName::new(None, ns!(html), local_name!("button")),
                vec![Attribute {
                    name: QualName::new(None, ns!(), local_name!("id")),
                    value: "trigger-btn".to_string(),
                }],
            );
            mutator.append_children(root_id, &[btn]);
            btn
        };
        doc.resolve(0.0);

        // 1. Keyboard focus on <button>: focused = Some(true), focus_visible = Some(true)
        assert!(doc.set_focus_to_with_source(btn_id, FocusSource::Keyboard));
        let snap_kb = inspect_document(&doc, InspectRequest::default());
        assert_eq!(snap_kb.focused_node_id, Some(btn_id.as_u64()));
        assert_eq!(snap_kb.focused_visible, Some(true));
        let btn_kb = snap_kb
            .nodes
            .iter()
            .find(|n| n.id == btn_id.as_u64())
            .expect("trigger-btn must exist");
        assert_eq!(btn_kb.focused, Some(true));
        assert_eq!(btn_kb.focus_visible, Some(true));

        // 2. Pointer focus on <button>: focused = Some(true), focus_visible = Some(false)
        doc.clear_focus();
        assert!(doc.set_focus_to_with_source(btn_id, FocusSource::Pointer));
        let snap_ptr = inspect_document(&doc, InspectRequest::default());
        assert_eq!(snap_ptr.focused_node_id, Some(btn_id.as_u64()));
        assert_eq!(snap_ptr.focused_visible, Some(false));
        let btn_ptr = snap_ptr
            .nodes
            .iter()
            .find(|n| n.id == btn_id.as_u64())
            .expect("trigger-btn must exist");
        assert_eq!(btn_ptr.focused, Some(true));
        assert_eq!(btn_ptr.focus_visible, Some(false));

        // 3. Explicit FocusOptions { focus_visible: Some(false) } overrides keyboard modality
        doc.clear_focus();
        assert!(doc.set_focus_to_with_source(btn_id, FocusSource::Keyboard));
        doc.clear_focus();
        assert!(doc.set_focus_to_with_options(
            btn_id,
            FocusSource::Script,
            FocusOptions {
                prevent_scroll: false,
                focus_visible: Some(false),
            },
        ));
        let snap_suppressed = inspect_document(&doc, InspectRequest::default());
        assert_eq!(snap_suppressed.focused_visible, Some(false));
        let btn_suppressed = snap_suppressed
            .nodes
            .iter()
            .find(|n| n.id == btn_id.as_u64())
            .expect("trigger-btn must exist");
        assert_eq!(btn_suppressed.focus_visible, Some(false));
    }
}
