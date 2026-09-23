use anyrender::PaintScene;
use anyrender_vello_cpu::VelloCpuImageRenderer;
use blitz_dom::BaseDocument;
use blitz_host_protocol::{CaptureRequest, CaptureResponse};
use peniko::Fill;
use peniko::kurbo::Rect;

/// Capture a visual screenshot of `doc` as PNG bytes at the current viewport size.
///
/// Returns `(width, height, png_bytes)` on success.
pub fn capture_document_png(doc: &mut BaseDocument) -> Result<(u32, u32, Vec<u8>), String> {
    let (mut width, mut height) = doc.viewport().window_size;
    if width == 0 {
        width = 800;
    }
    if height == 0 {
        height = 600;
    }
    let scale = doc.viewport().scale_f64();

    // Render document scene into an RGBA buffer via Vello CPU image renderer
    let buffer = anyrender::render_to_buffer::<VelloCpuImageRenderer, _>(
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

    // Encode RGBA buffer into PNG bytes
    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("Failed to initialize PNG header: {e}"))?;
        writer
            .write_image_data(&buffer)
            .map_err(|e| format!("Failed to encode PNG image data: {e}"))?;
    }

    Ok((width, height, png_bytes))
}

/// Capture visual screenshot of `doc` and format as a typed [`CaptureResponse`].
pub fn capture_document(doc: &mut BaseDocument, _request: CaptureRequest) -> CaptureResponse {
    match capture_document_png(doc) {
        Ok((width, height, png_bytes)) => {
            use base64::prelude::*;
            let data_base64 = BASE64_STANDARD.encode(&png_bytes);
            CaptureResponse {
                success: true,
                width,
                height,
                format: "png".to_string(),
                data_base64,
                message: Some(format!(
                    "Captured {width}x{height} PNG visual screenshot ({} bytes)",
                    png_bytes.len()
                )),
            }
        }
        Err(err) => CaptureResponse {
            success: false,
            width: 0,
            height: 0,
            format: "png".to_string(),
            data_base64: String::new(),
            message: Some(err),
        },
    }
}
