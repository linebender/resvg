// Copyright 2020 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::interfaces::{IInitializeWithStream, IThumbnailProvider};
use crate::utils::{img_to_hbitmap, render_thumbnail, tree_from_istream};
use crate::WINLOG_SOURCE;
use com::co_class;
use com::sys::{HRESULT, IID, S_OK};
use log::error;
use resvg::usvg;
use std::cell::RefCell;
use winapi::shared::minwindef::{DWORD, UINT};
use winapi::shared::windef::HBITMAP;
use winapi::um::objidlbase::LPSTREAM;

// {4432C229-DFD0-4B18-8C4D-F58932AF6105}
pub const CLSID_THUMBNAIL_PROVIDER_CLASS: IID = IID {
    data1: 0x4432c229,
    data2: 0xdfd0,
    data3: 0x4b18,
    data4: [0x8c, 0x4d, 0xf5, 0x89, 0x32, 0xaf, 0x61, 0x5],
};

#[co_class(implements(IThumbnailProvider, IInitializeWithStream))]
pub struct ThumbnailProvider {
    tree: RefCell<Option<usvg::Tree>>,
}

impl IInitializeWithStream for ThumbnailProvider {
    unsafe fn read(&self, pstream: LPSTREAM, _grf_mode: DWORD) -> HRESULT {
        tree_from_istream(pstream).map_or_else(
            |err| {
                error!("{}", err);
                err.into()
            },
            |tree| {
                self.tree.replace(Some(tree));
                S_OK
            },
        )
    }
}

impl IThumbnailProvider for ThumbnailProvider {
    unsafe fn get_thumbnail(&self, cx: UINT, phbmp: *mut HBITMAP, pdw_alpha: *mut UINT) -> HRESULT {
        render_thumbnail(&*self.tree.borrow(), cx)
            .and_then(|img| img_to_hbitmap(&img))
            .map_or_else(
                |err| {
                    error!("{}", err);
                    err.into()
                },
                |hbmp| {
                    *phbmp = hbmp;
                    *pdw_alpha = 2;
                    S_OK
                },
            )
    }
}

impl ThumbnailProvider {
    pub(crate) fn new() -> Box<ThumbnailProvider> {
        // winlog::init fails sometimes but logging still works
        #[allow(unused_must_use)]
        {
            winlog::init(WINLOG_SOURCE);
        }
        ThumbnailProvider::allocate(RefCell::new(None))
    }
}
