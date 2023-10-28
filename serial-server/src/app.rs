use crate::error_template::{AppError, ErrorTemplate};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use serialport::{DataBits, Parity, SerialPortInfo, StopBits};
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/serial-server.css"/>

        // sets the document title
        <Title text="Welcome to Leptos"/>

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
            serialport::SerialPortType::UsbPort(_) if port.port_name.contains("cu.usbserial-") => true,
            _ => false,
        })
        .cloned()
        .collect();

    log::info!("Loaded {} ports ", ports.len());
    Ok(ports)
}

#[server(TestPort)]
pub async fn test_port(port_name: String) -> Result<(), ServerFnError> {
    log::info!("Testing port {}", port_name);

    let baud_rate = 9600;

    let mut read_port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_secs(10))
        .data_bits(DataBits::Eight)
        .stop_bits(StopBits::One)
        .parity(Parity::None)
        .open()
        .expect("Failed to open port");


    log::info!("Opened port!");
    // Rust fmt wants to put the comments inline with the line above when they are meant for below
    // but I usually don't do code and comment in one line to avoid later issues
    #[rustfmt::skip]
    let message = [
        // Address
        0x01_u8,
        // Function code
        0x04,
        // Register address
        0x00, 0x01,
        // Number of registers
        0x00, 0x01,
        // CRC
        0x60,
        0x0a
    ];

    let bytes_written = read_port.write(&message).expect("Expected to write to port");

    log::info!("Wrote {} bytes to port!", bytes_written);

    let mut buffer: Vec<u8> = vec![0; 32];
    let bytes_read = read_port.read(buffer.as_mut_slice());

    log::info!("Read {:?} bytes: {:X?}", bytes_read, buffer);

    Ok(())
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

    view! {
        <h1 class="text-2xl">Ports</h1>

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
    }
}
