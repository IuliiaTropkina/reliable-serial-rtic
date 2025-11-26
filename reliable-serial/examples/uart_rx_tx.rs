//! This minimal, bare-metal program send and receives over UART, showing how to configure it.
//!
//! Based on upstream example at
//! <https://github.com/esp-rs/esp-hal/blob/main/examples/src/bin/advanced_serial.rs>
//!
//! You can short the TX and RX pin and see it reads what was written ("0x42" or "B").
//!
//! This example could be used with a logic analyzer to see how changes of configuration affect the
//! output signal. This is not relevant for the current implementation of the course.
//!
//! The following wiring is assumed:
//! - TX => GPIO21
//! - RX => GPIO20

#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    esp_riscv_rt::entry,
    uart::{self, Uart},
};
use rtt_target::{rprintln, rtt_init_print};

#[entry]
fn main() -> ! {
    // This call is required to configure RTT-based serial output
    rtt_init_print!();

    // Retrieve the program's name from the host's environment at time of compilation
    rprintln!(env!("CARGO_CRATE_NAME"));

    // Obtain an instance of all device peripherals
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let (rx, tx) = (peripherals.GPIO20, peripherals.GPIO21);
    let mut serial = Uart::new(peripherals.UART0, uart::Config::default())
        .unwrap()
        .with_rx(rx)
        .with_tx(tx);

    let delay = Delay::new();

    rprintln!("Start");
    loop {
        serial.write(&[0x42]).unwrap();

        let unit_buf = &mut [0; 1];
        let read_count = serial.read(unit_buf);

        match read_count {
            Ok(_read_count) => rprintln!("Read 0x{:02x}", unit_buf[0]),
            Err(err) => rprintln!("Error {:?}", err),
        }

        delay.delay_millis(250);
    }
}
