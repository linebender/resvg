// Copyright 2026 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! WebP is not supported, like in Chrome and Firefox, because its EXIF chunk comes after
//! the image data: <https://www.w3.org/TR/css-images-3/#image-metadata>

use std::ops::Range;

use tiny_skia_path::Transform;

use crate::{ImageKind, Size};

const ORIENTATION_TAG: u16 = 0x0112;
const TYPE_SHORT: u16 = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum Orientation {
    #[default]
    Normal,
    FlipHorizontal,
    Rotate180,
    FlipVertical,
    Transpose,
    Rotate90,
    Transverse,
    Rotate270,
}

impl Orientation {
    fn from_exif(value: u16) -> Option<Self> {
        match value {
            1 => Some(Self::Normal),
            2 => Some(Self::FlipHorizontal),
            3 => Some(Self::Rotate180),
            4 => Some(Self::FlipVertical),
            5 => Some(Self::Transpose),
            6 => Some(Self::Rotate90),
            7 => Some(Self::Transverse),
            8 => Some(Self::Rotate270),
            _ => None,
        }
    }

    pub(crate) fn apply_to_size(self, size: Size) -> Size {
        match self {
            Self::Normal | Self::FlipHorizontal | Self::Rotate180 | Self::FlipVertical => size,
            Self::Transpose | Self::Rotate90 | Self::Transverse | Self::Rotate270 => {
                Size::from_wh(size.height(), size.width()).unwrap()
            }
        }
    }

    /// Maps the stored pixels onto the oriented image.
    pub(crate) fn transform(self, size: Size) -> Transform {
        let (w, h) = (size.width(), size.height());
        match self {
            Self::Normal => Transform::identity(),
            Self::FlipHorizontal => Transform::from_row(-1.0, 0.0, 0.0, 1.0, w, 0.0),
            Self::Rotate180 => Transform::from_row(-1.0, 0.0, 0.0, -1.0, w, h),
            Self::FlipVertical => Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, h),
            Self::Transpose => Transform::from_row(0.0, 1.0, 1.0, 0.0, 0.0, 0.0),
            Self::Rotate90 => Transform::from_row(0.0, 1.0, -1.0, 0.0, h, 0.0),
            Self::Transverse => Transform::from_row(0.0, -1.0, -1.0, 0.0, h, w),
            Self::Rotate270 => Transform::from_row(0.0, -1.0, 1.0, 0.0, 0.0, w),
        }
    }
}

pub(crate) fn orientation(kind: &ImageKind) -> Orientation {
    find_orientation(kind)
        .map(|tag| tag.orientation)
        .unwrap_or_default()
}

#[cfg(feature = "writer")]
pub(crate) fn without_orientation(kind: &ImageKind) -> Option<Vec<u8>> {
    let tag = find_orientation(kind)?;
    if tag.orientation == Orientation::Normal {
        return None;
    }

    let mut data = match kind {
        ImageKind::JPEG(data) | ImageKind::PNG(data) => data.as_slice().to_vec(),
        _ => return None,
    };

    let value: [u8; 2] = if tag.big_endian { [0, 1] } else { [1, 0] };
    data.get_mut(tag.value_offset..tag.value_offset + 2)?
        .copy_from_slice(&value);

    if let Some(chunk) = tag.png_chunk {
        let crc = crc32(data.get(chunk.clone())?);
        data.get_mut(chunk.end..chunk.end + 4)?
            .copy_from_slice(&crc.to_be_bytes());
    }

    Some(data)
}

struct OrientationTag {
    orientation: Orientation,
    #[cfg_attr(not(feature = "writer"), allow(dead_code))]
    value_offset: usize,
    #[cfg_attr(not(feature = "writer"), allow(dead_code))]
    big_endian: bool,
    #[cfg_attr(not(feature = "writer"), allow(dead_code))]
    png_chunk: Option<Range<usize>>,
}

fn find_orientation(kind: &ImageKind) -> Option<OrientationTag> {
    match kind {
        ImageKind::JPEG(data) => {
            let exif = find_jpeg_exif(data)?;
            find_tiff_orientation(data, exif, None)
        }
        ImageKind::PNG(data) => {
            let (exif, chunk) = find_png_exif(data)?;
            find_tiff_orientation(data, exif, Some(chunk))
        }
        _ => None,
    }
}

fn find_jpeg_exif(data: &[u8]) -> Option<Range<usize>> {
    const EXIF_HEADER: &[u8] = b"Exif\0\0";

    if !data.starts_with(&[0xFF, 0xD8]) {
        return None;
    }

    let mut pos = 2;
    loop {
        if *data.get(pos)? != 0xFF {
            return None;
        }

        while *data.get(pos)? == 0xFF {
            pos += 1;
        }

        let marker = data[pos];
        pos += 1;

        match marker {
            // SOS, EOI
            0xDA | 0xD9 => return None,
            // No length
            0x01 | 0xD0..=0xD8 => continue,
            _ => {}
        }

        let len = usize::from(read_u16(data, pos, true)?);
        let segment = pos.checked_add(2)?..pos.checked_add(len)?;
        if len < 2 || segment.end > data.len() {
            return None;
        }

        // APP1
        if marker == 0xE1 && data[segment.clone()].starts_with(EXIF_HEADER) {
            return Some(segment.start + EXIF_HEADER.len()..segment.end);
        }

        pos = segment.end;
    }
}

/// Returns the TIFF data and the chunk type and data, which the CRC covers.
fn find_png_exif(data: &[u8]) -> Option<(Range<usize>, Range<usize>)> {
    const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

    if !data.starts_with(SIGNATURE) {
        return None;
    }

    let mut pos = SIGNATURE.len();
    loop {
        let len = usize::try_from(read_u32(data, pos, true)?).ok()?;
        let chunk_type = data.get(pos + 4..pos + 8)?;
        let chunk_data = pos.checked_add(8)?..pos.checked_add(8)?.checked_add(len)?;
        if chunk_data.end > data.len() {
            return None;
        }

        match chunk_type {
            b"IDAT" | b"IEND" => return None,
            b"eXIf" => return Some((chunk_data.clone(), pos + 4..chunk_data.end)),
            _ => {}
        }

        // Skip the CRC.
        pos = chunk_data.end.checked_add(4)?;
    }
}

fn find_tiff_orientation(
    data: &[u8],
    tiff: Range<usize>,
    png_chunk: Option<Range<usize>>,
) -> Option<OrientationTag> {
    let tiff_data = data.get(tiff.clone())?;
    let big_endian = match tiff_data.get(0..4)? {
        b"II\x2A\x00" => false,
        b"MM\x00\x2A" => true,
        _ => return None,
    };

    let ifd = usize::try_from(read_u32(tiff_data, 4, big_endian)?).ok()?;
    let count = read_u16(tiff_data, ifd, big_endian)?;
    for i in 0..usize::from(count) {
        // 12 byte entries: tag, type, count, value
        let entry = ifd.checked_add(2 + i * 12)?;
        if read_u16(tiff_data, entry, big_endian)? != ORIENTATION_TAG {
            continue;
        }

        if read_u16(tiff_data, entry + 2, big_endian)? != TYPE_SHORT
            || read_u32(tiff_data, entry + 4, big_endian)? != 1
        {
            return None;
        }

        let value_offset = entry + 8;
        let orientation = Orientation::from_exif(read_u16(tiff_data, value_offset, big_endian)?)?;
        return Some(OrientationTag {
            orientation,
            value_offset: tiff.start + value_offset,
            big_endian,
            png_chunk,
        });
    }

    None
}

fn read_u16(data: &[u8], pos: usize, big_endian: bool) -> Option<u16> {
    let bytes = data.get(pos..pos.checked_add(2)?)?.try_into().ok()?;
    Some(if big_endian {
        u16::from_be_bytes(bytes)
    } else {
        u16::from_le_bytes(bytes)
    })
}

fn read_u32(data: &[u8], pos: usize, big_endian: bool) -> Option<u32> {
    let bytes = data.get(pos..pos.checked_add(4)?)?.try_into().ok()?;
    Some(if big_endian {
        u32::from_be_bytes(bytes)
    } else {
        u32::from_le_bytes(bytes)
    })
}

#[cfg(any(feature = "writer", test))]
fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn tiff(big_endian: bool, entries: &[(u16, u16, u32, u16)]) -> Vec<u8> {
        let u16_bytes = |v: u16| {
            if big_endian {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        };
        let u32_bytes = |v: u32| {
            if big_endian {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        };

        let mut data = Vec::new();
        data.extend_from_slice(if big_endian {
            b"MM\x00\x2A"
        } else {
            b"II\x2A\x00"
        });
        data.extend_from_slice(&u32_bytes(8));
        data.extend_from_slice(&u16_bytes(entries.len() as u16));
        for &(tag, kind, count, value) in entries {
            data.extend_from_slice(&u16_bytes(tag));
            data.extend_from_slice(&u16_bytes(kind));
            data.extend_from_slice(&u32_bytes(count));
            data.extend_from_slice(&u16_bytes(value));
            data.extend_from_slice(&[0, 0]);
        }
        data.extend_from_slice(&u32_bytes(0));
        data
    }

    fn jpeg(tiff: &[u8]) -> ImageKind {
        let mut data = vec![0xFF, 0xD8];
        data.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00]);
        data.extend_from_slice(&[0xFF, 0xE1]);
        data.extend_from_slice(&((tiff.len() + 8) as u16).to_be_bytes());
        data.extend_from_slice(b"Exif\0\0");
        data.extend_from_slice(tiff);
        data.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x02, 0xFF, 0xD9]);
        ImageKind::JPEG(Arc::new(data))
    }

    fn png_chunk(data: &mut Vec<u8>, chunk_type: &[u8], chunk_data: &[u8]) {
        data.extend_from_slice(&(chunk_data.len() as u32).to_be_bytes());
        let start = data.len();
        data.extend_from_slice(chunk_type);
        data.extend_from_slice(chunk_data);
        let crc = crc32(&data[start..]);
        data.extend_from_slice(&crc.to_be_bytes());
    }

    fn png(tiff: &[u8], before_idat: bool) -> ImageKind {
        let mut data = b"\x89PNG\r\n\x1a\n".to_vec();
        png_chunk(&mut data, b"IHDR", &[0; 13]);
        if before_idat {
            png_chunk(&mut data, b"eXIf", tiff);
        }
        png_chunk(&mut data, b"IDAT", &[0; 4]);
        if !before_idat {
            png_chunk(&mut data, b"eXIf", tiff);
        }
        png_chunk(&mut data, b"IEND", &[]);
        ImageKind::PNG(Arc::new(data))
    }

    fn orientation_tag(value: u16) -> (u16, u16, u32, u16) {
        (ORIENTATION_TAG, TYPE_SHORT, 1, value)
    }

    #[test]
    fn jpeg_little_endian() {
        let tiff = tiff(false, &[(0x010F, 2, 4, 0), orientation_tag(6)]);
        assert_eq!(orientation(&jpeg(&tiff)), Orientation::Rotate90);
    }

    #[test]
    fn jpeg_big_endian() {
        let tiff = tiff(true, &[orientation_tag(3), (0x0110, 2, 4, 0)]);
        assert_eq!(orientation(&jpeg(&tiff)), Orientation::Rotate180);
    }

    #[test]
    fn all_values() {
        let expected = [
            Orientation::Normal,
            Orientation::FlipHorizontal,
            Orientation::Rotate180,
            Orientation::FlipVertical,
            Orientation::Transpose,
            Orientation::Rotate90,
            Orientation::Transverse,
            Orientation::Rotate270,
        ];
        for (value, expected) in (1..=8).zip(expected) {
            let tiff = tiff(false, &[orientation_tag(value)]);
            assert_eq!(orientation(&jpeg(&tiff)), expected);
        }
    }

    #[test]
    fn invalid_values() {
        for value in [0, 9, 0xFFFF] {
            let tiff = tiff(false, &[orientation_tag(value)]);
            assert_eq!(orientation(&jpeg(&tiff)), Orientation::Normal);
        }
    }

    #[test]
    fn wrong_type_or_count() {
        let tiff1 = tiff(false, &[(ORIENTATION_TAG, 4, 1, 6)]);
        assert_eq!(orientation(&jpeg(&tiff1)), Orientation::Normal);

        let tiff2 = tiff(false, &[(ORIENTATION_TAG, TYPE_SHORT, 2, 6)]);
        assert_eq!(orientation(&jpeg(&tiff2)), Orientation::Normal);
    }

    #[test]
    fn no_orientation_tag() {
        let tiff = tiff(false, &[(0x010F, 2, 4, 0)]);
        assert_eq!(orientation(&jpeg(&tiff)), Orientation::Normal);
    }

    #[test]
    fn malformed_jpeg() {
        let mut tiff = tiff(false, &[orientation_tag(6)]);

        let ImageKind::JPEG(data) = jpeg(&tiff) else {
            unreachable!()
        };
        for len in 0..data.len() {
            let _ = orientation(&ImageKind::JPEG(Arc::new(data[..len].to_vec())));
        }

        // An IFD offset outside of the data.
        tiff[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(orientation(&jpeg(&tiff)), Orientation::Normal);

        tiff[0..2].copy_from_slice(b"XX");
        assert_eq!(orientation(&jpeg(&tiff)), Orientation::Normal);

        let data = Arc::new(b"not a jpeg".to_vec());
        assert_eq!(orientation(&ImageKind::JPEG(data)), Orientation::Normal);
    }

    #[test]
    fn jpeg_exif_after_scan_is_ignored() {
        let tiff = tiff(false, &[orientation_tag(6)]);
        let mut data = vec![0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02];
        data.extend_from_slice(&[0xFF, 0xE1]);
        data.extend_from_slice(&((tiff.len() + 8) as u16).to_be_bytes());
        data.extend_from_slice(b"Exif\0\0");
        data.extend_from_slice(&tiff);
        assert_eq!(
            orientation(&ImageKind::JPEG(Arc::new(data))),
            Orientation::Normal
        );
    }

    #[test]
    fn png_exif_before_idat() {
        let tiff = tiff(true, &[orientation_tag(8)]);
        assert_eq!(orientation(&png(&tiff, true)), Orientation::Rotate270);
    }

    #[test]
    fn png_exif_after_idat_is_ignored() {
        let tiff = tiff(true, &[orientation_tag(8)]);
        assert_eq!(orientation(&png(&tiff, false)), Orientation::Normal);
    }

    #[test]
    fn other_formats_are_ignored() {
        let data = Arc::new(b"RIFF\0\0\0\0WEBP".to_vec());
        assert_eq!(orientation(&ImageKind::WEBP(data)), Orientation::Normal);
    }

    #[test]
    fn transforms_map_the_image_onto_the_oriented_size() {
        let size = Size::from_wh(4.0, 2.0).unwrap();
        for value in 1..=8 {
            let orientation = Orientation::from_exif(value).unwrap();
            let oriented = orientation.apply_to_size(size);
            let rect = size
                .to_rect(0.0, 0.0)
                .unwrap()
                .transform(orientation.transform(size))
                .unwrap();
            assert_eq!(rect, oriented.to_rect(0.0, 0.0).unwrap(), "{orientation:?}");
        }
    }

    #[cfg(feature = "writer")]
    #[test]
    fn reset_jpeg_orientation() {
        for big_endian in [false, true] {
            let image = jpeg(&tiff(big_endian, &[orientation_tag(6)]));
            let data = without_orientation(&image).unwrap();
            let image = ImageKind::JPEG(Arc::new(data));
            assert_eq!(
                find_orientation(&image).unwrap().orientation,
                Orientation::Normal
            );
        }
    }

    #[cfg(feature = "writer")]
    #[test]
    fn reset_png_orientation_updates_the_checksum() {
        let image = png(&tiff(false, &[orientation_tag(6)]), true);
        let data = without_orientation(&image).unwrap();

        let tag = find_orientation(&ImageKind::PNG(Arc::new(data.clone()))).unwrap();
        assert_eq!(tag.orientation, Orientation::Normal);

        let chunk = tag.png_chunk.unwrap();
        let crc = u32::from_be_bytes(data[chunk.end..chunk.end + 4].try_into().unwrap());
        assert_eq!(crc, crc32(&data[chunk]));
    }

    #[cfg(feature = "writer")]
    #[test]
    fn nothing_to_reset() {
        let image = jpeg(&tiff(false, &[orientation_tag(1)]));
        assert!(without_orientation(&image).is_none());

        let image = jpeg(&tiff(false, &[]));
        assert!(without_orientation(&image).is_none());
    }
}
