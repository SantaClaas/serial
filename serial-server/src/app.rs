use crate::core::{CreateDeviceError, Device, DEVICE_ADDRESS_RANGE};
use crate::error_template::{AppError, ErrorTemplate};
use crc::{Crc, CRC_16_MODBUS};
use leptos::leptos_dom::logging::console_log;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use serde::de::StdError;
use serde::{Deserialize, Serialize};
use serialport::SerialPortInfo;
use std::fmt::Display;
use std::io::{Read, Write};

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/serial-server.css"/>

        // Sets the document title
        <Title text="Fan Control"/>

        // Content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { <ErrorTemplate outside_errors/> }.into_view()
        }>
            <main>
                <Routes>
                    <Route path="" view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

#[server(GetPorts)]
pub async fn get_ports() -> Result<Vec<SerialPortInfo>, ServerFnError> {
    log::info!("Getting serial ports");
    // Returning serial port directly is not advised as it is a struct not under our control
    // meaning we might leak data if the serial port struct updates and gets fields through a crate
    // update with information we might not want to be send to the client
    let ports: Vec<SerialPortInfo> = serialport::available_ports()
        .expect("Expected to get serial ports")
        .iter()
        .filter(|port| match port.port_type {
            // Only USB devices as our adapter is a USB device
            // Browser Serial API uses "cu.usbserial-<number>" so we assume that is best
            serialport::SerialPortType::UsbPort(_) if port.port_name.contains("cu.usbserial-") => {
                true
            }
            _ => false,
        })
        .cloned()
        .collect();

    log::info!("Loaded {} ports ", ports.len());
    Ok(ports)
}

const CRC: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);

mod modbus {
    use std::io::Write;

    //IDK there is probably a serialize to binary crate with trait
    trait Serialize<const N: usize> {
        fn serialize(&self) -> [u8; N];
    }

    #[derive(Clone, Copy)]
    pub(crate) struct Register {
        pub(crate) address: u16,
        /// Length in bytes of the register
        pub(crate) length: u16,
    }
    pub(crate) enum FunctionCode {
        ReadHoldingRegister,
        ReadInputRegister,
        WriteHoldingRegister,
    }

    enum Command {
        ReadHoldingRegister,
        ReadInputRegister,
        WriteSingleRegister,
        Diagnostics,
        WriteMultipleRegisters,
    }

    impl Command {
        fn code(&self) -> CommandCode {
            match self {
                Command::ReadHoldingRegister => 0x03,
                Command::ReadInputRegister => 0x04,
                Command::WriteSingleRegister => 0x06,
                Command::Diagnostics => 0x08,
                Command::WriteMultipleRegisters => 0x10,
            }
        }
    }

    // Let's over-engineer this! 🤡

    /// Valid addresses are 1..247 (1 to 247).
    /// 0 is the broadcast address.
    /// Consider that 1 is often used as default address for a device
    /// if no other address has been configured.
    type DeviceAddress = u8;
    type CommandCode = u8;

    type Checksum = u16;

    type Data = Vec<u8>;
    /// A modbus frame that gets send over the wire
    struct Frame(DeviceAddress, CommandCode, Data, Checksum);

    impl Frame {
        fn serialize(&self) -> Vec<u8> {
            // Size is data size N
            // + 1 byte device address
            // + 1 byte command code
            // + 2 byte checksum

            let data_length = self.2.len();
            let mut frame: Vec<u8> = Vec::with_capacity(data_length + 4);
            frame[0] = self.0;
            frame[1] = self.1;

            for index in 0..data_length {
                frame[1 + index] = self.2[index];
            }

            let checksum = self.3.to_be_bytes();
            frame[data_length + 1] = checksum[0];
            frame[data_length + 2] = checksum[1];
            frame
        }
    }

    /// Implementers are able to send frames of size N
    trait SendFrame<E> {
        fn send(&mut self, frame: &Frame) -> Result<(), E>;
    }

    // For example serialport can send modbus frames of size N

    impl SendFrame<std::io::Error> for dyn Write {
        fn send(&mut self, frame: &Frame) -> Result<(), std::io::Error> {
            let frame = frame.serialize();
            self.write_all(&frame)
        }
    }

    // pub(crate) enum Command<const N: usize> {
    //     ReadHoldingRegister(Register),
    //     ReadInputRegister(Register),
    //     WriteHoldingRegister(Register, u16),
    // }
    //
    // pub(crate) struct ReadHoldingRegister(Register);
    //
    // impl Serialize<8> for ReadHoldingRegister {
    //     fn serialize(&self) -> [u8; 8] {
    //         let address : [u8; 2] = self.0.address.to_le_bytes();
    //         [address[0], address[1], 0, 0, 0, 0, 0, 0]
    //     }
    // }

    pub(super) enum FunctionType {
        ReadHoldingRegister,
        ReadInputRegister,
        WriteHoldingRegister,
    }

    pub(super) struct Function {
        pub(super) register: Register,
        pub(super) r#type: FunctionType,
    }

    impl Function {
        pub(super) fn code(&self) -> u8 {
            match self.r#type {
                FunctionType::ReadHoldingRegister => 0x03,
                FunctionType::ReadInputRegister => 0x04,
                FunctionType::WriteHoldingRegister => 0x06,
            }
        }
    }
}


#[server(TestPort)]
pub async fn test_port(port_name: String) -> Result<(), ServerFnError> {
    use modbus::FunctionType;
    use modbus::Function;
    use modbus::Register;

    use serialport::Parity;
    use serialport::StopBits;
    use serialport::DataBits;
    use core::time::Duration;

    log::info!("Testing port {}", port_name);

    let baud_rate = 9600;

    let mut read_port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_secs(10))
        .data_bits(DataBits::Eight)
        .stop_bits(StopBits::One)
        .parity(Parity::None)
        .open()
        .expect("Failed to open port");

    let temperature_register = Register {
        address: 0x0001,
        length: 2,
    };

    let function = Function {
        r#type: FunctionType::ReadInputRegister,
        register: temperature_register,
    };

    log::info!("Opened port!");
    let mut message = [0x00u8; 8];

    // Device address
    message[0] = 0x01;
    // Function Code
    message[1] = function.code();

    // Address
    let address_bytes: [u8; 2] = function.register.address.to_be_bytes();
    message[2] = address_bytes[0];
    message[3] = address_bytes[1];

    // Number of registers
    // Always write only one register currently
    // No-op
    // message[4] = 0x00;
    message[5] = 0x01;

    // Checksum
    let checksum: [u8; 2] = CRC.checksum(&message[..6]).to_be_bytes();
    // They come out reversed...
    message[6] = checksum[1];
    message[7] = checksum[0];

    let expected = [0x60_u8, 0x0a];
    let actual = &message[6..];
    assert_eq!(
        actual,
        &expected[..],
        "Expected: {:X?} Actual: {:X?}",
        &expected[..],
        actual
    );

    let bytes_written = read_port
        .write(&message)
        .expect("Expected to write to port");

    log::info!("Wrote {} bytes to port!", bytes_written);

    let mut buffer: Vec<u8> = vec![0; 32];
    let bytes_read = read_port.read(buffer.as_mut_slice());

    log::info!("Read {:?} bytes: {:X?}", bytes_read, buffer);

    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SetFanSpeedError {
    /// The value was either larger than 1 or smaller than 0
    SpeedOutOfRange,
}
#[server(SetFanSpeed)]
pub async fn set_fan_speed(speed: f32) -> Result<Result<(), SetFanSpeedError>, ServerFnError> {
    if 0. > speed || speed > 1. {
        // This feels weird
        return Ok(Err(SetFanSpeedError::SpeedOutOfRange));
    }

    dbg!(speed);

    Ok(Ok(()))
}
#[server(CreateDevice)]
pub async fn create_device(device: Device) -> Result<Result<Vec<Device>, CreateDeviceError>, ServerFnError> {
    dbg!(&device);

    Ok(crate::core::create_device(device))

    // Ok(Ok(()))
}

#[component]
fn DeviceList(devices: ReadSignal<Vec<Device>>, set_devices: WriteSignal<Vec<Device>>) -> impl IntoView {
    // let (devices, set_devices) = create_signal::<Vec<Device>>(Vec::default());

    // A note on state syncing:
    // As this project is limited in scope, we keep the state management simple and don't try to
    // keep the device list on each client in sync with the server.
    // We assume that only one client makes changes to the device list at a time and rely on user
    // communication of the change.
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
        // iterate over the rows and display each value
        <For
            each=devices
            key=|device| device.address.clone()
            let:device
        >
            <p>{device.address}</p>
        </For>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    // Runs once as source signal is empty
    let ports = create_resource(|| (), |_| get_ports());

    let call_test = |name| {
        spawn_local(async {
            let _ = test_port(name).await;
        })
    };

    let refresh = move |_| {
        ports.refetch();
    };

    let fan_speed_change = |event| {
        let value = event_target_value(&event);
        // console_log(&*format!("{}", value));

        let value = value.parse::<f32>();

        console_log(&*format!("{:?}", value));

        if let Ok(value) = value {
            spawn_local(async move {
                let _ = set_fan_speed(value).await;
            })
        }
    };

    let (devices, set_devices) = create_signal::<Vec<Device>>(Vec::default());

    view! {
        <h1 class="text-2xl">Ports</h1>
        <button on:click=refresh>Refresh</button>

        <Suspense fallback=move || {
            view! { <p>Loading ports...</p> }
        }>

            {move || match ports.get() {
                None => view! { <p>Loading ports...</p> }.into_view(),
                Some(result) => {
                    match result {
                        Err(_) => view! { <p>Sorry, could not load ports</p> }.into_view(),
                        Ok(ports) => {
                            if ports.is_empty() {
                                view! { <p>No ports found</p> }.into_view()
                            } else {
                                view! {
                                    <ol>
                                        {ports
                                            .into_iter()
                                            .map(move |port| {
                                                let name = port.port_name.clone();
                                                view! {
                                                    // TODO how to less clone
                                                    <li class="flex gap-4">
                                                        <span>{port.port_name.clone()}</span>
                                                        <button
                                                            class="rounded-full bg-cyan-600 px-2.5 py-1 text-xs font-semibold text-white shadow-sm hover:bg-cyan-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-cyan-600"
                                                            on:click=move |_| call_test(name.clone())
                                                        >
                                                            Test
                                                        </button>
                                                    </li>
                                                }
                                                    .into_view()
                                            })
                                            .collect_view()}
                                    </ol>
                                }
                                    .into_view()
                            }
                        }
                    }
                }
            }}

        </Suspense>

        <input id="fan_speed" type="range" min="0" max="1" step="0.1" on:input=fan_speed_change/>
        <label for="fan_speed">Fan</label>


        <DeviceList devices set_devices />
    }
}
