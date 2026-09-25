use blitz_dom::{BaseDocument, NodeId};
use blitz_host_protocol::ElementTarget;

/// Resolves an element target against the live `BaseDocument`.
///
/// Supports either:
/// 1. Direct numeric node ID: verifies that the node exists in the document.
/// 2. CSS selector string: queries the live document using `doc.query_selector` backed by Stylo.
///    If an unadorned ID string was passed (e.g. without `#`), falls back to `doc.get_element_by_id`.
///
/// Returns:
/// - `Ok(Some(u64))` on successful resolution.
/// - `Ok(None)` if no target was specified.
/// - `Err(String)` if a target was specified but not found or the selector was invalid.
pub fn resolve_target_in_doc(
    doc: &BaseDocument,
    target: Option<&ElementTarget>,
    node_id: Option<u64>,
    selector: Option<&str>,
) -> Result<Option<u64>, String> {
    // 1. Resolve explicit ElementTarget if provided
    if let Some(target) = target {
        return match target {
            ElementTarget::Id(id) => {
                let nid = NodeId::from_u64(*id);
                if doc.get_node(nid).is_some() {
                    Ok(Some(*id))
                } else {
                    Err(format!("Node #{id} not found in document"))
                }
            }
            ElementTarget::Selector(sel) => resolve_selector_in_doc(doc, sel),
        };
    }

    // 2. Fall back to selector field if provided
    if let Some(sel) = selector {
        return resolve_selector_in_doc(doc, sel);
    }

    // 3. Fall back to direct numeric node_id field if provided
    if let Some(id) = node_id {
        let nid = NodeId::from_u64(id);
        if doc.get_node(nid).is_some() {
            Ok(Some(id))
        } else {
            Err(format!("Node #{id} not found in document"))
        }
    } else {
        Ok(None)
    }
}

fn resolve_selector_in_doc(doc: &BaseDocument, raw_sel: &str) -> Result<Option<u64>, String> {
    let trimmed = raw_sel.trim();
    if trimmed.is_empty() {
        return Err("Empty selector provided".to_string());
    }

    // 1. Execute live CSS selector query via Blitz Stylo engine
    match doc.query_selector(trimmed) {
        Ok(Some(matched_node)) => Ok(Some(matched_node.as_u64())),
        Ok(None) => {
            // Check fallback for bare ID without leading '#'
            let fallback_id = trimmed.strip_prefix('#').unwrap_or(trimmed);
            if let Some(matched_node) = doc.get_element_by_id(fallback_id) {
                return Ok(Some(matched_node.as_u64()));
            }
            Err(format!(
                "Element matching selector '{trimmed}' not found in document"
            ))
        }
        Err(err) => {
            // If selector parsing failed (e.g. invalid CSS syntax), try bare element id fallback
            let fallback_id = trimmed.strip_prefix('#').unwrap_or(trimmed);
            if let Some(matched_node) = doc.get_element_by_id(fallback_id) {
                return Ok(Some(matched_node.as_u64()));
            }
            Err(format!("Invalid CSS selector '{trimmed}': {err:?}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blitz_dom::DocumentConfig;

    #[test]
    fn test_resolve_target_by_node_id() {
        let doc = BaseDocument::new(DocumentConfig::default());
        let root_id = doc.root_node().id.as_u64();

        // Valid node ID
        let res = resolve_target_in_doc(&doc, Some(&ElementTarget::Id(root_id)), None, None);
        assert_eq!(res, Ok(Some(root_id)));

        // Non-existent node ID
        let res = resolve_target_in_doc(&doc, Some(&ElementTarget::Id(9999999)), None, None);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("not found in document"));
    }

    #[test]
    fn test_resolve_target_none() {
        let doc = BaseDocument::new(DocumentConfig::default());
        let res = resolve_target_in_doc(&doc, None, None, None);
        assert_eq!(res, Ok(None));
    }
}
