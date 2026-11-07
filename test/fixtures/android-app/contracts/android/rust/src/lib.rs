use std::ffi::c_void;

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_RustPackagingProbe_value(
    _env: *mut c_void,
    _class: *mut c_void,
) -> i32 {
    31_415
}
