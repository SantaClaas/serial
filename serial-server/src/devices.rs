use crate::core::{CreateDeviceError, Device, DEVICE_ADDRESS_RANGE};
use leptos::*;
use leptos_router::*;
use std::fmt::Display;

#[server(CreateDevice)]
pub async fn create_device(device: Device) -> Result<Result<Vec<Device>, CreateDeviceError>, ServerFnError> {
    dbg!(&device);

    Ok(crate::core::create_device(device).await)

    // Ok(Ok(()))
}

#[component]
pub fn DeviceList(devices: ReadSignal<Vec<Device>>, set_devices: WriteSignal<Vec<Device>>) -> impl IntoView {
    // let (devices, set_devices) = create_signal::<Vec<Device>>(Vec::default());

    // A note on state syncing:
    // As this project is limited in scope, we keep the state management simple and just push the
    // the entire device list down after an update.
    // We also don't attempt to keep them in sync for every client. So client A will need to manually
    // refresh when client B makes a change to the device list on the server.
    // We assume that only one client makes changes to the device list at a time and rely on user
    // communication of the change if there are multiple simultaneous users.
    // However we should handle errors due to out of sync clients gracefully.
    let create_device = Action::<CreateDevice, _>::server();
    create_effect(move |_| {
        let value = create_device.value().get();

        if let Some(Ok(Ok(devices))) = value {
            set_devices.set(devices);
        }
    });

    view! {
        <hgroup>
            <h2>Devices</h2>
            <p>Add devices to assign them to their serial port</p>
        </hgroup>

        <ActionForm action=create_device>
            <fieldset>
                <legend>New Device</legend>
                <label for="address">Address</label>
                <input
                  id="address"
                  type="number"
                  name="device[address]"
                  inputmode="numeric"
                  min=DEVICE_ADDRESS_RANGE.start
                  // HTML max is inclusive the range is exclusive
                  max=DEVICE_ADDRESS_RANGE.end - 1
                  step="1"
                />

                <label for="type">Type</label>
                <select id="type" name="device[type]">
                    <option value="Fan">Fan</option>
                    <option value="TemperatureSensor">Temperature Sensor</option>
                </select>
            </fieldset>

            <button type="submit">Add</button>
        </ActionForm>
        // Iterate over the rows and display each value
        <For
            each=devices
            key=|device| device.address.clone()
            let:device
        >
            <p>{device.address}</p>
        </For>
    }
}

