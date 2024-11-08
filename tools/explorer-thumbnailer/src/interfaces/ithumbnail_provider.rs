// Copyright 2020 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use com::com_interface;
use com::interfaces::iunknown::IUnknown;
use com::sys::HRESULT;
use winapi::shared::minwindef::UINT;
use winapi::shared::windef::HBITMAP;

#[com_interface("E357FCCD-A995-4576-B01F-234630154E96")]
pub trait IThumbnailProvider: IUnknown {
    unsafe fn get_thumbnail(&self, cx: UINT, phbmp: *mut HBITMAP, pdw_alpha: *mut UINT) -> HRESULT;
}
