use blitz_dom::{BaseDocument, NodeData, NodeId};
use blitz_host_protocol::{InspectRequest, InspectResponse, SemanticNode};

/// Inspect a real running Blitz `BaseDocument` and produce a typed `InspectResponse`.
pub fn inspect_document(doc: &BaseDocument, request: InspectRequest) -> InspectResponse {
    let document_id = doc.id();
    let target = request.target();
    let root_id = match crate::target::resolve_target_in_doc(
        doc,
        target.as_ref(),
        request.root_node_id,
        request.selector.as_deref(),
    ) {
        Ok(Some(id)) => NodeId::from_u64(id),
        Ok(None) => doc.root_node().id,
        Err(err) => {
            return InspectResponse {
                document_id,
                root_id: 0,
                node_count: 0,
                current_frame: None,
                focused_node_id: doc.get_focussed_node_id().map(|id| id.as_u64()),
                hover_node_id: doc.get_hover_node_id().map(|id| id.as_u64()),
                viewport_scroll: None,
                nodes: Vec::new(),
                message: Some(err),
            };
        }
    };

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
                let text = elem
                    .text_input_data()
                    .map(|input| input.editor.raw_text().to_string())
                    .or_else(|| {
                        elem.attrs
                            .iter()
                            .find(|a| a.name.local.as_ref() == "value")
                            .map(|a| a.value.to_string())
                    });
                (tag, dom_id, role, text)
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
                let pos = node.absolute_position(0.0, 0.0);
                Some([pos.x, pos.y, layout.size.width, layout.size.height])
            }
            _ => None,
        };

        let is_focused = doc.get_focussed_node_id() == Some(node_id);
        let focused = if is_focused { Some(true) } else { None };
        let hovered = if node.is_hovered() { Some(true) } else { None };
        let active = if node.is_active() { Some(true) } else { None };
        let scroll_offset = match &node.data {
            NodeData::Element(_) | NodeData::AnonymousBlock(_) | NodeData::Document(_) => {
                let offset = *node.scroll_offset();
                if offset.x != 0.0 || offset.y != 0.0 {
                    Some([offset.x, offset.y])
                } else {
                    None
                }
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
            focused,
            hovered,
            active,
            scroll_offset,
            children: children_ids,
        });
    }

    let vp_scroll = doc.viewport_scroll();
    let viewport_scroll = if vp_scroll.x != 0.0 || vp_scroll.y != 0.0 {
        Some([vp_scroll.x, vp_scroll.y])
    } else {
        None
    };

    InspectResponse {
        document_id,
        root_id: root_id.as_u64(),
        node_count: nodes.len(),
        current_frame: None,
        focused_node_id: doc.get_focussed_node_id().map(|id| id.as_u64()),
        hover_node_id: doc.get_hover_node_id().map(|id| id.as_u64()),
        viewport_scroll,
        nodes,
        message: None,
    }
}
