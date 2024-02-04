#![no_std]
#![no_main]

use pros::prelude::*;

struct MyStruct {}

impl Drop for MyStruct {
	fn drop(&mut self) {
		println!("If you're reading this, unwinding is working!");
	}
}

#[derive(Default)]
pub struct Robot;

impl AsyncRobot for Robot {
    async fn opcontrol(&mut self) -> pros::Result {
        println!("Yo");
        let s = MyStruct {};
        panic!("{:?}", "Panicked.");
        Ok(())
    }
}
async_robot!(Robot);
