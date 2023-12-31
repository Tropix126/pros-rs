use core::ffi::c_double;

use pros_sys::PROS_ERR;

use crate::{error::{bail_on, PortError}, smart_device::SmartDevice};

pub struct DistanceSensor {
    port: u8,
}

impl DistanceSensor {
    pub fn new(port: u8) -> Result<Self, PortError> {
        let sensor = Self { port };
        sensor.distance()?;
        Ok(sensor)
    }

    /// Returns the distance to the object the sensor detects in millimeters.
    pub fn distance(&self) -> Result<u32, PortError> {
        Ok(unsafe { bail_on!(PROS_ERR, pros_sys::distance_get(self.port)) as u32 })
    }

    /// returns the velocity of the object the sensor detects in m/s
    pub fn object_velocity(&self) -> Result<f64, PortError> {
        // all VEX Distance Sensor functions return PROS_ERR on failure even though
        // some return floating point values (not PROS_ERR_F)
        Ok(unsafe {
            bail_on!(
                PROS_ERR as c_double,
                pros_sys::distance_get_object_velocity(self.port)
            )
        })
    }

    /// Returns the confidence in the distance measurement from 0% to 100%.
    pub fn distance_confidence(&self) -> Result<f32, PortError> {
        // 0 -> 63
        let confidence =
            unsafe { bail_on!(PROS_ERR, pros_sys::distance_get_confidence(self.port)) } as f32;
        Ok(confidence * 100.0 / 63.0)
    }
}

impl SmartDevice for DistanceSensor {
    fn port(&self) -> u8 {
        self.port
    }

    fn installed(&self) -> bool {
        self.distance().is_ok()
    }
}
