// Copyright 2020 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use com::sys::{E_POINTER, HRESULT, S_FALSE};
use resvg::usvg;
use Error::*;

#[derive(Debug)]
pub enum Error {
    IStreamStat(HRESULT),
    IStreamRead(HRESULT),
    TreeError(usvg::Error),
    TreeEmpty,
    CreateDIBSectionError,
    RenderError,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            IStreamStat(code) => write!(f, "IStream::stat failed with error {}", code),
            IStreamRead(code) => write!(f, "IStream::read failed with error {}", code),
            TreeError(err) => write!(f, "Tree::from_data failed with error \"{}\"", err),
            TreeEmpty => write!(f, "SVG tree was not initialized"),
            CreateDIBSectionError => write!(f, "CreateDIBSection failed"),
            RenderError => write!(f, "resvg::render returned None"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &*self {
            IStreamStat(_) | IStreamRead(_) | TreeEmpty | CreateDIBSectionError | RenderError => {
                None
            }
            TreeError(source) => Some(source),
        }
    }
}

impl From<Error> for HRESULT {
    fn from(err: Error) -> Self {
        match err {
            IStreamStat(code) | IStreamRead(code) => code,
            TreeError(_) | TreeEmpty | RenderError => S_FALSE,
            CreateDIBSectionError => E_POINTER,
        }
    }
}
