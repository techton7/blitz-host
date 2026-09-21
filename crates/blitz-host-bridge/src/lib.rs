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
}
