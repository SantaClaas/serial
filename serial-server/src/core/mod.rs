use cfg_if::cfg_if;

/// Valid device addresses between 1..247 (inclusive) (1..248 exclusive). 0 is broadcast.
/// It is recommended to not use 1 as address as it is often used as default address for devices
/// pending configuration.
///
pub const DEVICE_ADDRESS_RANGE: core::ops::Range<u8> = 1..248;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DeviceType {
    Fan,
    TemperatureSensor,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NoStateError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CreateDeviceError {
    /// The address was out of the 1..247 range
    AddressOutOfRange,
    AddressTaken,
    NoState(NoStateError),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Device {
    pub address: u8,
    pub r#type: DeviceType,
}

cfg_if! { if #[cfg(feature = "ssr")] {
    use crate::state::OurState;
    use leptos::use_context;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn get_our_state() -> Result<Arc<Mutex<OurState>>, NoStateError> {
        use_context::<Arc<Mutex<OurState>>>().ok_or(NoStateError)
    }

    pub async fn create_device(device: Device) -> Result<Vec<Device>, CreateDeviceError> {

        if !DEVICE_ADDRESS_RANGE.contains(&device.address) {
            return Err(CreateDeviceError::AddressOutOfRange);
        }

        let state = get_our_state().map_err(CreateDeviceError::NoState)?;
        let mut lock = state.lock().await;

        let is_taken = lock.devices.iter().any(|existing_device| existing_device.address == device.address);
        if is_taken {
            return Err(CreateDeviceError::AddressTaken);
        }

        lock.devices.push(device);

        // Assume we don't have a lot of devices
        let new_devices = lock.devices.clone();
        Ok(new_devices)
    }
}}
