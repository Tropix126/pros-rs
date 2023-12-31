#![feature(error_in_core, stdsimd, negative_impls)]
#![no_std]

extern crate alloc;

pub mod async_runtime;
pub mod controller;
pub mod error;
pub mod motor;
pub mod pid;
pub mod position;
pub mod sensors;
pub mod sync;
#[macro_use]
pub mod task;
#[doc(hidden)]
pub use pros_sys as __pros_sys;
#[cfg(target_os = "vexos")]
mod vexos_env;
#[cfg(target_arch = "wasm32")]
mod wasm_env;
#[macro_use]
pub mod lcd;
pub mod adi;
pub mod battery;
pub mod competition;
pub mod link;
pub mod lvgl;
pub mod usd;
pub mod smart_device;

pub use async_trait::async_trait;

pub type Result<T = ()> = core::result::Result<T, alloc::boxed::Box<dyn core::error::Error>>;

use alloc::boxed::Box;
#[async_trait::async_trait]
pub trait AsyncRobot {
    async fn opcontrol(&mut self) -> Result {
        Ok(())
    }
    async fn auto(&mut self) -> Result {
        Ok(())
    }
    async fn disabled(&mut self) -> Result {
        Ok(())
    }
    async fn comp_init(&mut self) -> Result {
        Ok(())
    }
}

pub trait SyncRobot {
    fn opcontrol(&mut self) -> Result {
        Ok(())
    }
    fn auto(&mut self) -> Result {
        Ok(())
    }
    fn disabled(&mut self) -> Result {
        Ok(())
    }
    fn comp_init(&mut self) -> Result {
        Ok(())
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __gen_sync_exports {
    ($rbt:ty) => {
        pub static mut ROBOT: Option<$rbt> = None;

        #[doc(hidden)]
        #[no_mangle]
        extern "C" fn opcontrol() {
            <$rbt as $crate::SyncRobot>::opcontrol(unsafe {
                ROBOT
                    .as_mut()
                    .expect("Expected initialize to run before opcontrol")
            })
            .unwrap();
        }

        #[doc(hidden)]
        #[no_mangle]
        extern "C" fn autonomous() {
            <$rbt as $crate::SyncRobot>::auto(unsafe {
                ROBOT
                    .as_mut()
                    .expect("Expected initialize to run before opcontrol")
            })
            .unwrap();
        }

        #[doc(hidden)]
        #[no_mangle]
        extern "C" fn disabled() {
            <$rbt as $crate::SyncRobot>::disabled(unsafe {
                ROBOT
                    .as_mut()
                    .expect("Expected initialize to run before opcontrol")
            })
            .unwrap();
        }

        #[doc(hidden)]
        #[no_mangle]
        extern "C" fn competition_initialize() {
            <$rbt as $crate::SyncRobot>::comp_init(unsafe {
                ROBOT
                    .as_mut()
                    .expect("Expected initialize to run before opcontrol")
            })
            .unwrap();
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __gen_async_exports {
    ($rbt:ty) => {
        pub static mut ROBOT: Option<$rbt> = None;

        #[doc(hidden)]
        #[no_mangle]
        extern "C" fn opcontrol() {
            $crate::async_runtime::block_on(<$rbt as $crate::AsyncRobot>::opcontrol(unsafe {
                ROBOT
                    .as_mut()
                    .expect("Expected initialize to run before opcontrol")
            }))
            .unwrap();
            $crate::async_runtime::complete_all();
        }

        #[doc(hidden)]
        #[no_mangle]
        extern "C" fn autonomous() {
            $crate::async_runtime::block_on(<$rbt as $crate::AsyncRobot>::opcontrol(unsafe {
                ROBOT
                    .as_mut()
                    .expect("Expected initialize to run before auto")
            }))
            .unwrap();
            $crate::async_runtime::complete_all();
        }

        #[doc(hidden)]
        #[no_mangle]
        extern "C" fn disabled() {
            $crate::async_runtime::block_on(<$rbt as $crate::AsyncRobot>::opcontrol(unsafe {
                ROBOT
                    .as_mut()
                    .expect("Expected initialize to run before disabled")
            }))
            .unwrap();
            $crate::async_runtime::complete_all();
        }

        #[doc(hidden)]
        #[no_mangle]
        extern "C" fn competition_initialize() {
            $crate::async_runtime::block_on(<$rbt as $crate::AsyncRobot>::opcontrol(unsafe {
                ROBOT
                    .as_mut()
                    .expect("Expected initialize to run before comp_init")
            }))
            .unwrap();
            $crate::async_runtime::complete_all();
        }
    };
}

/// Allows your async robot code to be executed by the pros kernel.
/// If your robot struct implements Default then you can just supply this macro with its type.
/// If not, you can supply an expression that returns your robot type to initialize your robot struct.
///
/// Example of using the macro with a struct that implements Default:
/// ```rust
/// use pros::prelude::*;
/// #[derive(Default)]
/// struct ExampleRobot;
/// #[async_trait]
/// impl AsyncRobot for ExampleRobot {
///    asnyc fn opcontrol(&mut self) -> pros::Result {
///       println!("Hello, world!");
///      Ok(())
///   }
/// }
/// async_robot!(ExampleRobot);
/// ```
///
/// Example of using the macro with a struct that does not implement Default:
/// ```rust
/// use pros::prelude::*;
/// struct ExampleRobot {
///    x: i32,
/// }
/// #[async_trait]
/// impl AsyncRobot for ExampleRobot {
///     async fn opcontrol(&mut self) -> pros::Result {
///         println!("Hello, world! {}", self.x);
///         Ok(())
///     }
/// }
/// impl ExampleRobot {
///     pub fn new() -> Self {
///        Self { x: 5 }
///    }
/// }
/// async_robot!(ExampleRobot, ExampleRobot::new());
#[macro_export]
macro_rules! async_robot {
    ($rbt:ty) => {
        $crate::__gen_async_exports!($rbt);

        #[no_mangle]
        extern "C" fn initialize() {
            ::pros::task::__init_main();
            unsafe {
                ROBOT = Some(Default::default());
            }
        }
    };
    ($rbt:ty, $init:expr) => {
        $crate::__gen_async_exports!($rbt);

        #[no_mangle]
        extern "C" fn initialize() {
            ::pros::task::__init_main();
            unsafe {
                ROBOT = Some($init);
            }
        }
    };
}

/// Allows your sync robot code to be executed by the pros kernel.
/// If your robot struct implements Default then you can just supply this macro with its type.
/// If not, you can supply an expression that returns your robot type to initialize your robot struct.
///
/// Example of using the macro with a struct that implements Default:
/// ```rust
/// use pros::prelude::*;
/// #[derive(Default)]
/// struct ExampleRobot;
/// impl SyncRobot for ExampleRobot {
///    asnyc fn opcontrol(&mut self) -> pros::Result {
///       println!("Hello, world!");
///      Ok(())
///   }
/// }
/// sync_robot!(ExampleRobot);
/// ```
///
/// Example of using the macro with a struct that does not implement Default:
/// ```rust
/// use pros::prelude::*;
/// struct ExampleRobot {
///    x: i32,
/// }
/// impl SyncRobot for ExampleRobot {
///     async fn opcontrol(&mut self) -> pros::Result {
///         println!("Hello, world! {}", self.x);
///         Ok(())
///     }
/// }
/// impl ExampleRobot {
///     pub fn new() -> Self {
///        Self { x: 5 }
///    }
/// }
/// sync_robot!(ExampleRobot, ExampleRobot::new());
#[macro_export]
macro_rules! sync_robot {
    ($rbt:ty) => {
        $crate::__gen_sync_exports!($rbt);

        #[no_mangle]
        extern "C" fn initialize() {
            ::pros::task::__init_main();
            unsafe {
                ROBOT = Some(Default::default());
            }
        }
    };
    ($rbt:ty, $init:expr) => {
        $crate::__gen_sync_exports!($rbt);

        #[no_mangle]
        extern "C" fn initialize() {
            ::pros::task::__init_main();
            unsafe {
                ROBOT = Some($init);
            }
        }
    };
}

pub mod prelude {
    pub use crate::{async_robot, sync_robot};
    pub use crate::{AsyncRobot, SyncRobot};

    // Import Box from alloc so that it can be used in async_trait!
    pub use crate::{async_trait, os_task_local, print, println};
    pub use alloc::boxed::Box;

    pub use crate::async_runtime::*;
    pub use crate::controller::*;
    pub use crate::error::PortError;
    pub use crate::lcd::{buttons::Button, LcdError};
    pub use crate::link::*;
    pub use crate::motor::*;
    pub use crate::pid::*;
    pub use crate::position::*;
    pub use crate::smart_device::*;
    pub use crate::sensors::distance::*;
    pub use crate::sensors::gps::*;
    pub use crate::sensors::rotation::*;
    pub use crate::sensors::vision::*;
    pub use crate::task::{sleep, spawn};
}
