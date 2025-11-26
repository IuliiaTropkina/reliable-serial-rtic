#![no_std]
#![no_main]

use core::iter;
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    esp_riscv_rt::entry,
    rmt::{ConstChannelAccess, Rmt},
    time::Rate,
};
use esp_hal_smartled::{smart_led_buffer, SmartLedsAdapter};
use rtt_target::{rprintln, rtt_init_print};
use smart_leds::{brightness, SmartLedsWrite, RGB8};

// Type ascription provided for reference
type RgbAdapter = SmartLedsAdapter<ConstChannelAccess<esp_hal::rmt::Tx, 0>, 25>;

pub fn set_rgb(color: RGB8, rgb_led: &mut RgbAdapter) {
    // Convert from the HSV color space (where we can easily transition from one
    // color to the other) to the RGB color space that we can then send to the LED
    //
    // When sending to the LED, we do a gamma correction first (see smart_leds
    // documentation for details) and then limit the brightness to 10 out of 255 so
    // that the output is not too bright.
    rgb_led.write(brightness(iter::once(color), 20)).unwrap();
}

#[entry]
fn main() -> ! {
    // This call is required to configure RTT-based serial output
    rtt_init_print!();

    // Retrieve the program's name from the host's environment at time of compilation
    rprintln!(env!("CARGO_CRATE_NAME"));

    // Obtain an instance of all device peripherals
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80u32)).unwrap();
    // We use one of the RMT channels to instantiate a `SmartLedsAdapter` which can be used
    // directly with all `smart_led` implementations
    let rmt_buffer = smart_led_buffer!(1);

    // Type ascription provided for reference
    let mut rgb_led: RgbAdapter =
        SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO2, rmt_buffer);

    let delay = Delay::new();
    let mut state = false;

    rprintln!("start blinking RGB");
    loop {
        if state {
            set_rgb(RGB8::new(0, 80, 40), &mut rgb_led);
        } else {
            set_rgb(RGB8::new(0, 0, 0), &mut rgb_led);
        }
        state = !state;

        delay.delay_millis(500);
    }
}
