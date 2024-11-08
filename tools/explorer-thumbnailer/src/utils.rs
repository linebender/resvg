// Copyright 2020 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::error::Error;
use com::sys::S_OK;
use resvg::{tiny_skia, usvg};
use std::mem;
use std::ptr;
use usvg::fontdb;
use winapi::ctypes::c_void;
use winapi::shared::minwindef::ULONG;
use winapi::shared::windef::{HBITMAP, HDC};
use winapi::um::objidlbase::{LPSTREAM, STATSTG};
use winapi::um::wingdi::{CreateDIBSection, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS};

pub unsafe fn tree_from_istream(pstream: LPSTREAM) -> Result<usvg::Tree, Error> {
    let mut stat: STATSTG = Default::default();
    let stat_res = (*pstream).Stat(&mut stat, 0);
    if stat_res != S_OK {
        return Err(Error::IStreamStat(stat_res));
    }

    let size = *stat.cbSize.QuadPart();
    let mut svg_data = Vec::with_capacity(size as usize);
    let mut len: ULONG = 0;
    let read_res = (*pstream).Read(svg_data.as_mut_ptr() as *mut c_void, size as u32, &mut len);
    if read_res != S_OK {
        return Err(Error::IStreamRead(read_res));
    }
    svg_data.set_len(len as usize);

    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();

    let tree = usvg::Tree::from_data(&svg_data, &opt).map_err(|e| Error::TreeError(e))?;
    Ok(tree)
}

pub fn render_thumbnail(tree: &Option<usvg::Tree>, cx: u32) -> Result<tiny_skia::Pixmap, Error> {
    if cx == 0 {
        return Err(Error::RenderError);
    }

    let tree = tree.as_ref().ok_or(Error::TreeEmpty)?;

    let size = if tree.size().width() > tree.size().height() {
        tree.size().to_int_size().scale_to_width(cx)
    } else {
        tree.size().to_int_size().scale_to_height(cx)
    }
    .ok_or(Error::RenderError)?;

    let transform = tiny_skia::Transform::from_scale(
        size.width() as f32 / tree.size().width() as f32,
        size.height() as f32 / tree.size().height() as f32,
    );

    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Ok(pixmap)
}

pub unsafe fn img_to_hbitmap(img: &tiny_skia::Pixmap) -> Result<HBITMAP, Error> {
    let hdc: HDC = ptr::null_mut();
    let mut bmi: BITMAPINFO = Default::default();
    bmi.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;
    bmi.bmiHeader.biWidth = img.width() as i32;
    bmi.bmiHeader.biHeight = -(img.height() as i32);
    let mut ppv_bits = ptr::null_mut();

    let hbitmap = CreateDIBSection(hdc, &bmi, DIB_RGB_COLORS, &mut ppv_bits, ptr::null_mut(), 0);
    if hbitmap as *const c_void == ptr::null() {
        return Err(Error::CreateDIBSectionError);
    }

    let mut i = 0;
    let ppv_bits = ppv_bits as *mut u8;
    for px in img.pixels() {
        let px = px.demultiply();
        ptr::write(ppv_bits.offset(i + 0), px.blue());
        ptr::write(ppv_bits.offset(i + 1), px.green());
        ptr::write(ppv_bits.offset(i + 2), px.red());
        ptr::write(ppv_bits.offset(i + 3), px.alpha());
        i += 4;
    }
    Ok(hbitmap)
}
