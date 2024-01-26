#![no_std]
#![no_main]

use pros::prelude::*;
use pros::time::Instant;

use core::hint::black_box;

pub fn benchmark() {
    let mat = black_box(MAT);
    let vec = black_box(VEC);

    for _ in 0..10_000_000 {
        black_box([
            mat[0] * vec[0] + mat[1] * vec[1] + mat[2] * vec[2],
            mat[3] * vec[0] + mat[4] * vec[1] + mat[5] * vec[2],
            mat[6] * vec[0] + mat[7] * vec[1] + mat[8] * vec[2],
        ]);
    }
}

const MAT: [f64; 9] = [
    1.0, 3.0, -2.0,
    2.0, 100.5, -6.7,
    -40.0, 32.0, -1.0,
];

const VEC: [f64; 3] = [
    3.0, -12.0, 400.0
];

#[derive(Default)]
pub struct Robot;

impl SyncRobot for Robot {
    fn opcontrol(&mut self) -> pros::Result {
        let now = Instant::now();
        benchmark();
        let elapsed = now.elapsed();
        println!("Finished benchmark in {:?}.", elapsed);

        Ok(())
    }
}
sync_robot!(Robot);
