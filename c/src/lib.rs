#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use ::safer_ffi::prelude::*;

pub mod atom;
pub mod space;
pub mod metta;

/// Frees a C string returned from a number of HyperonC functions
/// 
/// NOTE: this function should be no different from a simple free(), unless HyperonC is using
/// a different allocator from the client code
#[ffi_export]
pub fn hyp_string_free(str: char_p::Box) {
    drop(str)
}

#[ffi_export]
pub fn init_logger() {
   hyperon::common::init_logger(false);
}

#[cfg(feature = "headers")]
#[::safer_ffi::cfg_headers]
pub fn generate_headers(file_path: &std::path::Path) -> ::std::io::Result<()> {
   ::safer_ffi::headers::builder()
      .to_file(file_path)?
      .generate()
}
