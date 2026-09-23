//! # blitz-host-transport
//!
//! Local Unix domain socket transport, owner-only discovery descriptors, and UI-thread handoff for blitz-host.

pub mod client;
pub mod discovery;
pub mod server;
pub mod waker;

pub use client::DebugClient;
pub use discovery::{
    descriptor_dir, discover, discover_target, is_reachable, list_hosts, read_descriptor,
    write_descriptor, TargetSelector,
};
pub use server::{ControlBridgeRequest, DebugServer};
pub use waker::ServiceWaker;

#[cfg(test)]
mod tests {
    use super::*;
    use blitz_host_protocol::{ControlRequest, ControlResponse, InspectRequest, InspectResponse, SemanticNode};
    use std::thread;

    #[test]
    fn test_transport_roundtrip_server_client() {
        let (server, rx) = DebugServer::start("test-blitz", "0.1.0").expect("server must start");
        let descriptor = server.descriptor().clone();

        let server_thread = thread::spawn(move || {
            let req = rx.recv().expect("must receive request");
            match &req.request {
                ControlRequest::Inspect(inspect_req) => {
                    assert_eq!(inspect_req.root_node_id, Some(42));
                    req.respond(ControlResponse::InspectSuccess(InspectResponse {
                        document_id: 1,
                        root_id: 42,
                        node_count: 1,
                        current_frame: Some(10),
                        nodes: vec![SemanticNode {
                            id: 42,
                            parent_id: None,
                            tag: "button".into(),
                            dom_id: Some("test-btn".into()),
                            role: Some("button".into()),
                            text: Some("Click Me".into()),
                            bounds: Some([10.0, 20.0, 100.0, 40.0]),
                            children: vec![],
                        }],
                    }));
                }
                other => panic!("Unexpected request: {other:?}"),
            }
        });

        let mut client = DebugClient::connect(descriptor).expect("client must connect");
        let resp = client
            .inspect(InspectRequest {
                window_id: None,
                root_node_id: Some(42),
                max_depth: None,
            })
            .expect("inspect must succeed");

        assert_eq!(resp.document_id, 1);
        assert_eq!(resp.nodes.len(), 1);
        assert_eq!(resp.nodes[0].tag, "button");
        assert_eq!(resp.nodes[0].dom_id.as_deref(), Some("test-btn"));

        server_thread.join().unwrap();
        server.shutdown();
    }
}
