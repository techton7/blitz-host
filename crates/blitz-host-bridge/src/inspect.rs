use blitz_dom::{BaseDocument, NodeData, NodeId};
use blitz_host_protocol::{InspectRequest, InspectResponse, SemanticNode};

/// Inspect a real running Blitz `BaseDocument` and produce a typed `InspectResponse`.
pub fn inspect_document(doc: &BaseDocument, request: InspectRequest) -> InspectResponse {
    let document_id = doc.id();
    let root_id = request
        .root_node_id
        .map(NodeId::from_u64)
        .unwrap_or_else(|| doc.root_node().id);

    let max_depth = request.max_depth.unwrap_or(u32::MAX);

    let mut nodes = Vec::new();
    let mut queue = std::collections::VecDeque::new();
    queue.push_back((root_id, 0u32));

    while let Some((node_id, depth)) = queue.pop_front() {
        if depth > max_depth {
            continue;
        }

        let Some(node) = doc.get_node(node_id) else {
            continue;
        };

        let (tag, dom_id, role, text) = match &node.data {
            NodeData::Document(_) => ("#document".to_string(), None, None, None),
            NodeData::Element(elem) => {
                let tag = elem.name.local.to_string();
                let dom_id = elem.id.as_ref().map(|id| id.to_string());
                let role = elem
                    .attrs
                    .iter()
                    .find(|a| a.name.local.as_ref() == "role")
                    .map(|a| a.value.to_string());
                (tag, dom_id, role, None)
            }
            NodeData::AnonymousBlock(_) => ("anonymous-block".to_string(), None, None, None),
            NodeData::Text(t) => ("#text".to_string(), None, None, Some(t.content.clone())),
            NodeData::Comment { contents } => {
                ("#comment".to_string(), None, None, Some(contents.clone()))
            }
        };

        let bounds = match &node.data {
            NodeData::Element(_) | NodeData::AnonymousBlock(_) | NodeData::Document(_) => {
                let layout = node.final_layout();
                Some([
                    layout.location.x,
                    layout.location.y,
                    layout.size.width,
                    layout.size.height,
                ])
            }
            _ => None,
        };

        let children_ids: Vec<u64> = node.children.iter().map(|c| c.as_u64()).collect();

        // Enqueue children
        for &child in &node.children {
            queue.push_back((child, depth + 1));
        }

        nodes.push(SemanticNode {
            id: node_id.as_u64(),
            parent_id: node.parent.map(|p| p.as_u64()),
            tag,
            dom_id,
            role,
            text,
            bounds,
            children: children_ids,
        });
    }

    InspectResponse {
        document_id,
        root_id: root_id.as_u64(),
        node_count: nodes.len(),
        current_frame: None,
        nodes,
    }
}
