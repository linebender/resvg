// Copyright 2026 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn pixmap() -> tiny_skia::Pixmap {
    let mut pixmap = tiny_skia::Pixmap::new(20, 10).unwrap();
    pixmap.fill(tiny_skia::Color::from_rgba8(0, 128, 128, 128));
    pixmap
}

/// Reads back what a PNG declares, in pixels per meter.
fn pixel_dims(data: &[u8]) -> Option<png::PixelDimensions> {
    let decoder = png::Decoder::new(std::io::Cursor::new(data));
    let reader = decoder.read_info().unwrap();
    reader.info().pixel_dims
}

#[test]
fn writes_the_resolution() {
    let data = resvg::encode_png_with_dpi(pixmap().as_ref(), 300.0).unwrap();
    let dims = pixel_dims(&data).expect("no pHYs chunk");

    // 300 dpi is 300 / 0.0254 pixels per meter, rounded to a whole pixel.
    assert_eq!(dims.xppu, 11811);
    assert_eq!(dims.yppu, 11811);
    assert_eq!(dims.unit, png::Unit::Meter);
}

#[test]
fn plain_encoding_declares_nothing() {
    let data = pixmap().encode_png().unwrap();
    assert!(pixel_dims(&data).is_none());
}

/// The added chunk must not change the image itself.
#[test]
fn keeps_the_pixels() {
    let pixmap = pixmap();
    let with_dpi = resvg::encode_png_with_dpi(pixmap.as_ref(), 300.0).unwrap();
    let plain = pixmap.encode_png().unwrap();

    let decode = |data: &[u8]| {
        let mut reader = png::Decoder::new(std::io::Cursor::new(data.to_vec()))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        buf.truncate(info.buffer_size());
        buf
    };

    assert_eq!(decode(&with_dpi), decode(&plain));
}

#[test]
fn saves_to_a_file() {
    let dir = std::env::temp_dir().join("resvg-png-dpi-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("dpi.png");

    resvg::save_png_with_dpi(pixmap().as_ref(), &path, 96.0).unwrap();

    let data = std::fs::read(&path).unwrap();
    assert_eq!(pixel_dims(&data).unwrap().xppu, 3780);
}
