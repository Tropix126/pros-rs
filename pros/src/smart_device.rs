pub struct SmartPort(u8);

impl SmartPort {
}

pub trait SmartDevice {
	/// Gets the current smart port index that this device is plugged into.
	fn port(&self) -> u8;

	/// Gets the device's connection status in the port.
	fn installed(&self) -> bool;
}