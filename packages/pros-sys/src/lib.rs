#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

pub mod adi;
#[cfg(feature = "xapi")]
pub mod apix;
pub mod colors;
pub mod distance;
pub mod error;
pub mod ext_adi;
pub mod gps;
pub mod imu;
pub mod link;
pub mod llemu;
pub mod misc;
pub mod motor;
pub mod optical;
pub mod rotation;
pub mod rtos;
pub mod vision;

use core::ffi::{c_char, c_int, c_void};

pub use adi::*;
pub use colors::*;
pub use distance::*;
pub use error::*;
pub use ext_adi::*;
pub use gps::*;
pub use imu::*;
pub use link::*;
pub use llemu::*;
pub use misc::*;
pub use motor::*;
pub use optical::*;
pub use rotation::*;
pub use rtos::*;
#[cfg(feaute = "apix")]
pub use serial::*;
pub use vision::*;
#[cfg(feaute = "apix")]
pub mod serial;

pub const CLOCKS_PER_SEC: u32 = 1000;

extern "C" {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn memalign(alignment: usize, size: usize) -> *mut c_void;
    #[cfg(not(target_arch = "wasm32"))]
    pub fn free(ptr: *mut c_void);
    pub fn __errno() -> *mut i32;
    pub fn clock() -> i32;
    pub fn puts(s: *const c_char) -> i32;
    pub fn exit(code: i32) -> !;
    pub fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
}

#[no_mangle]
extern "C" fn cpp_opcontrol() {}

#[no_mangle]
extern "C" fn cpp_autonomous() {}

#[no_mangle]
extern "C" fn cpp_disabled() {}

#[no_mangle]
extern "C" fn cpp_competition_initialize() {}

#[no_mangle]
extern "C" fn cpp_initialize() {}

#[no_mangle]
extern "C" fn task_fn_wrapper() {}
