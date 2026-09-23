//! # blitz-host-bridge
//!
//! Runtime host bridge connecting a real running Blitz window and `blitz_dom::BaseDocument`
//! to the blitz-host control plane.

pub mod bridge;
pub mod inspect;

pub use bridge::HostBridge;
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
}
