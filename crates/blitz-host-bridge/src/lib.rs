//! # blitz-host-bridge
//!
//! Runtime host bridge connecting a real running Blitz window and `blitz_dom::BaseDocument`
//! to the blitz-host control plane.

pub mod bridge;
pub mod capture;
pub mod inspect;

pub use bridge::HostBridge;
pub use capture::{capture_document, capture_document_png};
pub use inspect::inspect_document;

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
        use std::sync::mpsc::{channel, sync_channel};
        use blitz_host_protocol::{ActionRequest, ControlRequest, ControlResponse};
        use blitz_host_transport::ControlBridgeRequest;

        let mut doc = BaseDocument::new(DocumentConfig::default());
        let root_id = doc.root_node().id;

        let (tx, rx) = channel::<ControlBridgeRequest>();
        let mut bridge = HostBridge::new(rx);

        // 1. Send Focus request
        let (focus_resp_tx, focus_resp_rx) = sync_channel(1);
        tx.send(ControlBridgeRequest {
            request: ControlRequest::Act(ActionRequest::Focus {
                window_id: None,
                node_id: root_id.as_u64(),
            }),
            reply: focus_resp_tx,
        }).unwrap();

        let serviced = bridge.poll_and_service_with(&mut doc, 1, |act, d| {
            match act {
                ActionRequest::Focus { node_id, .. } => {
                    d.set_focus_to(blitz_dom::NodeId::from_u64(*node_id));
                    Ok(blitz_host_protocol::ActionResponse {
                        success: true,
                        node_id: *node_id,
                        message: Some("focused".into()),
                    })
                }
                _ => Err("unsupported".into()),
            }
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
        use std::sync::mpsc::{channel, sync_channel};
        use blitz_host_protocol::{CaptureRequest, ControlRequest, ControlResponse};
        use blitz_host_transport::ControlBridgeRequest;

        let mut doc = BaseDocument::new(DocumentConfig::default());

        // Test direct helper
        let (width, height, png_bytes) = capture_document_png(&mut doc).expect("capture_document_png must succeed");
        assert!(width > 0 && height > 0);
        assert!(png_bytes.len() > 8);
        assert_eq!(&png_bytes[0..4], &[0x89, b'P', b'N', b'G'], "must start with PNG magic bytes");

        // Test IPC bridge routing
        let (tx, rx) = channel::<ControlBridgeRequest>();
        let mut bridge = HostBridge::new(rx);

        let (cap_resp_tx, cap_resp_rx) = sync_channel(1);
        tx.send(ControlBridgeRequest {
            request: ControlRequest::Capture(CaptureRequest::default()),
            reply: cap_resp_tx,
        }).unwrap();

        let serviced = bridge.poll_and_service_with(&mut doc, 1, |_, _| {
            Err("no actions".into())
        });
        assert_eq!(serviced, 1);

        let resp = cap_resp_rx.recv().unwrap();
        match resp {
            ControlResponse::CaptureSuccess(cap) => {
                assert!(cap.success);
                assert_eq!(cap.format, "png");
                assert_eq!(cap.width, width);
                assert_eq!(cap.height, height);
                use base64::prelude::*;
                let decoded = BASE64_STANDARD.decode(&cap.data_base64).unwrap();
                assert_eq!(decoded, png_bytes);
            }
            other => panic!("expected CaptureSuccess, got {other:?}"),
        }
    }
}
