//! # blitz-host-bridge
//!
//! Runtime host bridge connecting a real running Blitz window and `blitz_dom::BaseDocument`
//! to the blitz-host control plane.

pub mod bridge;
pub mod capture;
pub mod inspect;
pub mod target;

pub use bridge::HostBridge;
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
        assert_eq!(serviced, 1);
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
        assert_eq!(serviced, 1);

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
        assert_eq!(serviced2, 1);

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
}
