use anyrender::PaintScene;
use anyrender_vello_cpu::VelloCpuImageRenderer;
use blitz_dom::{BaseDocument, NodeId};
use blitz_host_protocol::{CaptureRequest, CaptureResponse};
use peniko::Fill;
use peniko::kurbo::Rect;

/// Capture a visual screenshot of `doc` as PNG bytes at the current viewport size.
///
/// Returns `(width, height, png_bytes)` on success.
pub fn capture_document_png(doc: &mut BaseDocument) -> Result<(u32, u32, Vec<u8>), String> {
    capture_document_node_png(doc, None).map(|(w, h, _, bytes)| (w, h, bytes))
}

/// Capture visual screenshot of `doc`, optionally cropped to a target `node_id`'s layout bounds.
///
/// Returns `(width, height, Option<target_node_id>, png_bytes)` on success.
pub fn capture_document_node_png(
    doc: &mut BaseDocument,
    target_node_id: Option<u64>,
) -> Result<(u32, u32, Option<u64>, Vec<u8>), String> {
    let (mut width, mut height) = doc.viewport().window_size;
    if width == 0 {
        width = 800;
    }
    if height == 0 {
        height = 600;
    }
    let scale = doc.viewport().scale_f64();

    // 1. If target_node_id is specified, resolve node bounds before rendering
    let crop_rect = if let Some(target_id) = target_node_id {
        let nid = NodeId::from_u64(target_id);
        let node = doc
            .get_node(nid)
            .ok_or_else(|| format!("Target node #{target_id} not found in document"))?;

        let pos = node.absolute_position(0.0, 0.0);
        let size = node.final_layout().size;

        if size.width <= 0.0 || size.height <= 0.0 {
            return Err(format!(
                "Target node #{target_id} has zero or invalid layout dimensions ({}x{})",
                size.width, size.height
            ));
        }

        // Convert CSS points/pixels to physical buffer pixels
        let crop_x = (pos.x as f64 * scale).round() as i32;
        let crop_y = (pos.y as f64 * scale).round() as i32;
        let crop_w = (size.width as f64 * scale).round() as i32;
        let crop_h = (size.height as f64 * scale).round() as i32;

        let left = crop_x.max(0).min(width as i32) as u32;
        let top = crop_y.max(0).min(height as i32) as u32;
        let right = (crop_x + crop_w).max(0).min(width as i32) as u32;
        let bottom = (crop_y + crop_h).max(0).min(height as i32) as u32;

        let final_w = right.saturating_sub(left);
        let final_h = bottom.saturating_sub(top);

        if final_w == 0 || final_h == 0 {
            return Err(format!(
                "Target node #{target_id} has no visible pixels in viewport (bounds: [{}, {}, {}, {}], viewport: {}x{})",
                pos.x, pos.y, size.width, size.height, width, height
            ));
        }

        Some((left, top, final_w, final_h))
    } else {
        None
    };

    // 2. Render document scene into an RGBA buffer via Vello CPU image renderer
    let full_buffer = anyrender::render_to_buffer::<VelloCpuImageRenderer, _>(
        |scene| {
            // Fill background with white by default
            scene.fill(
                Fill::NonZero,
                Default::default(),
                blitz_dom::util::Color::WHITE,
                Default::default(),
                &Rect::new(0.0, 0.0, width as f64, height as f64),
            );

            // Paint the live Blitz document scene
            blitz_paint::paint_scene(scene, doc, scale, width, height, 0, 0);
        },
        width,
        height,
    );

    // 3. Extract sub-region if cropped, or use full buffer
    let (final_width, final_height, image_buffer) =
        if let Some((left, top, crop_w, crop_h)) = crop_rect {
            let mut cropped = Vec::with_capacity((crop_w * crop_h * 4) as usize);
            let bottom = top + crop_h;
            for y in top..bottom {
                let row_start = ((y * width + left) * 4) as usize;
                let row_end = row_start + (crop_w * 4) as usize;
                cropped.extend_from_slice(&full_buffer[row_start..row_end]);
            }
            (crop_w, crop_h, cropped)
        } else {
            (width, height, full_buffer)
        };

    // 4. Encode RGBA buffer into PNG bytes
    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, final_width, final_height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("Failed to initialize PNG header: {e}"))?;
        writer
            .write_image_data(&image_buffer)
            .map_err(|e| format!("Failed to encode PNG image data: {e}"))?;
    }

    Ok((final_width, final_height, target_node_id, png_bytes))
}

/// Capture visual screenshot of `doc` and write directly to `request.output_path`.
pub fn capture_document(doc: &mut BaseDocument, request: CaptureRequest) -> CaptureResponse {
    let target = request.target();
    let node_id = request.node_id;
    let selector = request.selector;
    let output_path = request.output_path;

    let target_node_id = match crate::target::resolve_target_in_doc(
        doc,
        target.as_ref(),
        node_id,
        selector.as_deref(),
    ) {
        Ok(resolved) => resolved,
        Err(err) => {
            return CaptureResponse {
                success: false,
                file_path: output_path,
                width: 0,
                height: 0,
                format: "png".to_string(),
                node_id: None,
                bytes: 0,
                message: Some(err),
            };
        }
    };

    match capture_document_node_png(doc, target_node_id) {
        Ok((width, height, node_id, png_bytes)) => {
            let target_file = std::path::Path::new(&output_path);
            if let Some(parent) = target_file.parent() {
                if !parent.as_os_str().is_empty() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        return CaptureResponse {
                            success: false,
                            file_path: output_path,
                            width: 0,
                            height: 0,
                            format: "png".to_string(),
                            node_id: target_node_id,
                            bytes: 0,
                            message: Some(format!("Failed to create parent directory: {e}")),
                        };
                    }
                }
            }

            if let Err(e) = std::fs::write(target_file, &png_bytes) {
                return CaptureResponse {
                    success: false,
                    file_path: output_path,
                    width: 0,
                    height: 0,
                    format: "png".to_string(),
                    node_id: target_node_id,
                    bytes: 0,
                    message: Some(format!("Failed to write capture to file: {e}")),
                };
            }

            let message = if let Some(nid) = node_id {
                format!(
                    "Captured node #{nid} cropped to {width}x{height} PNG visual screenshot to {output_path} ({} bytes)",
                    png_bytes.len()
                )
            } else {
                format!(
                    "Captured {width}x{height} PNG visual screenshot to {output_path} ({} bytes)",
                    png_bytes.len()
                )
            };

            CaptureResponse {
                success: true,
                file_path: output_path,
                width,
                height,
                format: "png".to_string(),
                node_id,
                bytes: png_bytes.len(),
                message: Some(message),
            }
        }
        Err(err) => CaptureResponse {
            success: false,
            file_path: output_path,
            width: 0,
            height: 0,
            format: "png".to_string(),
            node_id: target_node_id,
            bytes: 0,
            message: Some(err),
        },
    }
}
