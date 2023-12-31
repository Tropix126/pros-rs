use pros_sys::{PROS_ERR, PROS_ERR_F};
use snafu::Snafu;

use crate::error::{bail_on, map_errno, PortError};

pub struct GpsStatus {
    pub x: f64,
    pub y: f64,
    pub pitch: f64,
    pub roll: f64,
    pub yaw: f64,
    pub heading: f64,

    pub accel_x: f64,
    pub accel_y: f64,
    pub accel_z: f64,
}

pub struct GpsSensor {
    port: u8,
}

impl GpsSensor {
    pub fn new(port: u8, x: f64, y: f64, heading: f64, x_offset: f64, y_offset: f64) -> Result<Self, GpsError> {
        unsafe {
            bail_on!(
                PROS_ERR,
                pros_sys::gps_initialize_full(port, x, y, heading, x_offset, y_offset)
            );
        }

        Ok(Self { port })
    }

    pub fn set_offset(&self, x: f64, y: f64) -> Result<(), GpsError> {
        unsafe {
            bail_on!(PROS_ERR, pros_sys::gps_set_offset(self.port, x, y));
        }
        Ok(())
    }

    pub fn offset(&self) -> Result<(f64, f64), GpsError> {
        let mut output: (&mut f64, &mut f64);
        
        unsafe { bail_on!(PROS_ERR, pros_sys::gps_get_offset(self.port, output.0, output.1)) }

        Ok((*output.0, *output.1))
    }

    pub fn set_position(&self, x: f64, y: f64, heading: f64) -> Result<(), GpsError> {
        unsafe {
            bail_on!(PROS_ERR, pros_sys::gps_set_position(self.port, x, y, heading));
        }
        Ok(())
    }

    pub fn set_imu_data_rate(&self, rate: core::time::Duration) -> Result<(), GpsError> {
        unsafe {
            bail_on!(PROS_ERR, pros_sys::gps_set_data_rate(self.port, rate.as_millis() as u32));
        }
        Ok(())
    }

    pub fn rms_error(&self) -> Result<f64, GpsError> {
        Ok(unsafe { bail_on!(PROS_ERR_F, pros_sys::gps_get_error(self.port)) })
    }

    pub fn status(&self) -> Result<GpsStatus, GpsError> {
        unsafe {
            let status = pros_sys::gps_get_status(self.port);
            bail_on!(PROS_ERR_F, status.x);
            let accel = pros_sys::gps_get_accel(self.port);
            bail_on!(PROS_ERR_F, accel.x);
            let heading = bail_on!(PROS_ERR_F, pros_sys::gps_get_heading(self.port));

            Ok(GpsStatus {
                x: status.x,
                y: status.y,
                pitch: status.pitch,
                roll: status.roll,
                yaw: status.yaw,
                heading,

                accel_x: accel.x,
                accel_y: accel.y,
                accel_z: accel.z,
            })
        }
    }

    pub fn zero_rotation(&self) -> Result<(), GpsError> {
        unsafe {
            bail_on!(PROS_ERR, pros_sys::gps_tare_rotation(self.port));
        }
        Ok(())
    }
}

#[derive(Debug, Snafu)]
pub enum GpsError {
    #[snafu(display("GPS sensor is still calibrating."))]
    StillCalibrating,
    #[snafu(display("{source}"), context(false))]
    Port { source: PortError },
}

map_errno! {
    GpsError {
        EAGAIN => Self::StillCalibrating,
    }
    inherit PortError;
}
