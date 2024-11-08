// Copyright 2020 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use com::registration::{dll_register_server, dll_unregister_server, RegistryKeyInfo};
use thumbnail_provider::{ThumbnailProvider, CLSID_THUMBNAIL_PROVIDER_CLASS};

mod error;
mod interfaces;
mod thumbnail_provider;
mod utils;

// we replace the com::registration::inproc_dll_module macro in order to be able
// to modify DllRegisterServer and DllUnregisterServer functions
macro_rules! inproc_dll_module {
    (($class_id_one:ident, $class_type_one:ty), $(($class_id:ident, $class_type:ty)),*) => {
        #[no_mangle]
        extern "stdcall" fn DllGetClassObject(class_id: *const com::sys::CLSID, iid: *const com::sys::IID, result: *mut *mut std::ffi::c_void) -> com::sys::HRESULT {
            use com::registration::initialize_class_object;
            assert!(!class_id.is_null(), "class id passed to DllGetClassObject should never be null");

            let class_id = unsafe { &*class_id };
            if class_id == &$class_id_one {
                let instance = <$class_type_one>::get_class_object();
                initialize_class_object(instance, iid, result)
            } $(else if class_id == &$class_id {
                let instance = <$class_type>::get_class_object();
                initialize_class_object(instance, iid, result)
            })* else {
                com::sys::CLASS_E_CLASSNOTAVAILABLE
            }
        }

        fn get_relevant_registry_keys() -> Vec<com::registration::RegistryKeyInfo> {
            use com::registration::RegistryKeyInfo;
            let file_path = com::registration::get_dll_file_path();
            vec![
                RegistryKeyInfo::new(
                    &com::registration::class_key_path($class_id_one),
                    "",
                    stringify!($class_type_one),
                ),
                RegistryKeyInfo::new(
                    &com::registration::class_inproc_key_path($class_id_one),
                    "",
                    &file_path,
                ),
                $(RegistryKeyInfo::new(
                    &com::registration::class_key_path($class_id),
                    "",
                    stringify!($class_type),
                ),
                RegistryKeyInfo::new(
                    &com::registration::class_inproc_key_path($class_id),
                    "",
                    &file_path,
                )),*
            ]
        }
    };
}

static WINLOG_SOURCE: &'static str = "reSVG Thumbnailer";

inproc_dll_module![(CLSID_THUMBNAIL_PROVIDER_CLASS, ThumbnailProvider),];

fn get_all_relevant_registry_keys() -> Vec<RegistryKeyInfo> {
    let mut res = get_relevant_registry_keys();
    res.extend(vec![RegistryKeyInfo::new(
        ".SVG\\shellex\\{E357FCCD-A995-4576-B01F-234630154E96}",
        "",
        "{4432C229-DFD0-4B18-8C4D-F58932AF6105}",
    )]);

    res
}

#[no_mangle]
extern "stdcall" fn DllRegisterServer() -> com::sys::HRESULT {
    if winlog::try_register(WINLOG_SOURCE).is_err() {
        return com::sys::SELFREG_E_CLASS;
    }
    dll_register_server(&mut get_all_relevant_registry_keys())
}

#[no_mangle]
extern "stdcall" fn DllUnregisterServer() -> com::sys::HRESULT {
    if winlog::try_deregister(WINLOG_SOURCE).is_err() {
        return com::sys::SELFREG_E_CLASS;
    }
    dll_unregister_server(&mut get_all_relevant_registry_keys())
}
