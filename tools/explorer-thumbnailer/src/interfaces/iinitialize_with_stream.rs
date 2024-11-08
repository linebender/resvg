// Copyright 2020 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use com::{com_interface, interfaces::iunknown::IUnknown, sys::HRESULT};
use winapi::shared::minwindef::DWORD;
use winapi::um::objidlbase::LPSTREAM;

#[com_interface("B824B49D-22AC-4161-AC8A-9916E8FA3F7F")]
pub trait IInitializeWithStream: IUnknown {
    unsafe fn read(&self, pstream: LPSTREAM, grf_mode: DWORD) -> HRESULT;
}
