// Copyright 2018 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use vello_cpu::kurbo::Affine;
use vello_cpu::RenderContext;

pub fn render(
    image: &usvg::Image,
    transform: Affine,
    rctx: &mut RenderContext,
) {
    if !image.is_visible() {
        return;
    }

    render_inner(image.kind(), transform, image.rendering_mode(), rctx);
}

pub fn render_inner(
    image_kind: &usvg::ImageKind,
    transform: Affine,
    #[allow(unused_variables)] rendering_mode: usvg::ImageRendering,
    pixmap: &mut RenderContext,
) {
    match image_kind {
        usvg::ImageKind::SVG(ref tree) => {
            render_vector(tree, transform, pixmap);
        }
        #[cfg(feature = "raster-images")]
        _ => {
            raster_images::render_raster(image_kind, transform, rendering_mode, pixmap);
        }
        #[cfg(not(feature = "raster-images"))]
        _ => {
            log::warn!("Images decoding was disabled by a build feature.");
        }
    }
}

fn render_vector(
    tree: &usvg::Tree,
    transform: Affine,
    rctx: &mut RenderContext,
) -> Option<()> {
    rctx.push_layer(None, None, None, None);
    crate::render(tree, transform, rctx);
    rctx.pop_layer();

    Some(())
}

#[cfg(feature = "raster-images")]
mod raster_images {
    use std::sync::Arc;
    use vello_cpu::kurbo::{Affine, Rect};
    use vello_cpu::{peniko, Image, ImageSource, PaintType, Pixmap, RenderContext};
    use vello_cpu::peniko::ImageQuality;
    use crate::OptionLog;
    use usvg::{tiny_skia_path, ImageRendering};

    fn decode_raster(image: &usvg::ImageKind) -> Option<Pixmap> {
        match image {
            usvg::ImageKind::SVG(_) => None,
            usvg::ImageKind::JPEG(ref data) => {
                decode_jpeg(data).log_none(|| log::warn!("Failed to decode a JPEG image."))
            }
            usvg::ImageKind::PNG(ref data) => {
                decode_png(data).log_none(|| log::warn!("Failed to decode a PNG image."))
            }
            usvg::ImageKind::GIF(ref data) => {
                decode_gif(data).log_none(|| log::warn!("Failed to decode a GIF image."))
            }
            usvg::ImageKind::WEBP(ref data) => {
                decode_webp(data).log_none(|| log::warn!("Failed to decode a WebP image."))
            }
        }
    }

    fn decode_png(data: &[u8]) -> Option<Pixmap> {
        Pixmap::from_png(data).ok()
    }

    fn decode_jpeg(data: &[u8]) -> Option<Pixmap> {
        use zune_jpeg::zune_core::colorspace::ColorSpace;
        use zune_jpeg::zune_core::options::DecoderOptions;

        let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
        let mut decoder = zune_jpeg::JpegDecoder::new_with_options(data, options);
        decoder.decode_headers().ok()?;
        let output_cs = decoder.get_output_colorspace()?;

        let mut img_data = {
            let data = decoder.decode().ok()?;
            match output_cs {
                ColorSpace::RGBA => data,
                // `set_output_color_space` is not guaranteed to actually always set the output space
                // to RGBA (its docs say "we do not guarantee the decoder can convert to all colorspaces").
                // In particular, it seems like it doesn't work for luma JPEGs,
                // so we convert them manually.
                ColorSpace::Luma => data
                    .into_iter()
                    .flat_map(|p| [p, p, p, 255])
                    .collect::<Vec<_>>(),
                _ => return None,
            }
        };

        let info = decoder.info()?;
        
        premultiply(&mut img_data);

        let size = tiny_skia_path::IntSize::from_wh(info.width as u32, info.height as u32)?;
        Some(Pixmap::from_parts(bytemuck::cast_vec(img_data), size.width() as u16, size.height() as u16))
    }
    
    // TODO: Don't clone buffer for gif/webp (see resvg impl)

    fn decode_gif(data: &[u8]) -> Option<Pixmap> {
        let mut decoder = gif::DecodeOptions::new();
        decoder.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = decoder.read_info(data).ok()?;
        let first_frame = decoder.read_next_frame().ok()??;
        
        let mut data = first_frame.buffer.to_vec();
        premultiply(&mut data);

        Some(Pixmap::from_parts(bytemuck::cast_vec(data), first_frame.width, first_frame.height))
    }

    fn decode_webp(data: &[u8]) -> Option<Pixmap> {
        let mut decoder = image_webp::WebPDecoder::new(std::io::Cursor::new(data)).ok()?;
        let mut first_frame = vec![0; decoder.output_buffer_size()?];
        decoder.read_image(&mut first_frame).ok()?;

        let (w, h) = decoder.dimensions();

        let mut data = if decoder.has_alpha() {
            first_frame
        } else {
            first_frame.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect()
        };

        premultiply(&mut data);

        Some(Pixmap::from_parts(bytemuck::cast_vec(data), w as u16, h as u16))
    }
    
    fn premultiply(data: &mut [u8]) {
        for p in data.chunks_mut(4) {
            let a = p[3] as u16;
            
            p[0] = ((p[0] as u16 * a) / 255) as u8;
            p[1] = ((p[1] as u16 * a) / 255) as u8;
            p[2] = ((p[2] as u16 * a) / 255) as u8;
        }
    }

    pub(crate) fn render_raster(
        image: &usvg::ImageKind,
        transform: Affine,
        rendering_mode: ImageRendering,
        rctx: &mut RenderContext,
    ) -> Option<()> {
        let pixmap = decode_raster(image)?;

        let rect = Rect::new(0.0, 0.0, pixmap.width() as f64, pixmap.height() as f64);

        let quality = match rendering_mode {
            ImageRendering::OptimizeQuality => ImageQuality::High,
            ImageRendering::OptimizeSpeed => ImageQuality::Low,
            ImageRendering::Smooth => ImageQuality::Medium,
            ImageRendering::HighQuality => ImageQuality::High,
            ImageRendering::CrispEdges => ImageQuality::Low,
            ImageRendering::Pixelated => ImageQuality::Medium,
        };

        let image = Image {
            source: ImageSource::Pixmap(Arc::new(pixmap)),
            x_extend: peniko::Extend::Pad,
            y_extend: peniko::Extend::Pad,
            quality,
        };
        
        rctx.set_paint(image);
        rctx.set_transform(transform);
        rctx.set_paint_transform(Affine::IDENTITY);
        rctx.fill_rect(&rect);

        Some(())
    }
}
