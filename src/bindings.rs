use autocxx::prelude::*;

// Autocxx entrypoint. The build script runs autocxx-build on this file.
include_cpp! {
    #include "ffi/livox_wrapper.hpp"

    // This crate is an FFI wrapper; the generated APIs are inherently unsafe.
    safety!(unsafe)

    // Generate only the thin wrapper functions to keep the surface area stable.
    generate!("carvi_livox_sdk2::sdk_init")
    generate!("carvi_livox_sdk2::sdk_uninit")
    generate!("carvi_livox_sdk2::sdk_start")
    generate!("carvi_livox_sdk2::sdk_stop")
}

// `autocxx` generates a private `ffi` module by default; re-export the symbols we need.
pub use ffi::carvi_livox_sdk2::{sdk_init, sdk_start, sdk_stop, sdk_uninit};
