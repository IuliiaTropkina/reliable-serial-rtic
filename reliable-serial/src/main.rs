//! ## Configurations
//!
//! Pins:
//!
//! | ESP32-C3  | Host  |
//! | :-:       | :-:   |
//! | IO21/TX   | RX    |
//! | IO20/RX   | TX    |
//!
//! Baud: 115_200 BPS
#![no_std]
#![no_main]

use the_protocol_serde::Request;  
use the_protocol_serde::Codec;  

mod serial;

// Bring in a panic handler
use panic_rtt_target as _;

use the_protocol::{Command, Funct, Response, RejectReason, Payload};
use the_protocol_serde::Codec;

#[rtic::app(device = esp32c3, dispatchers=[FROM_CPU_INTR0, FROM_CPU_INTR1, FROM_CPU_INTR2])]
mod app {
    use super::*;
    use crate::serial;

    use esp_hal::{
        rmt::{ConstChannelAccess, Rmt},
        time,
        uart::{self, Uart, UartRx, UartTx},
        gpio::{Output, OutputConfig},
        Blocking,
    };
    use esp_hal_smartled::{smart_led_buffer, SmartLedsAdapter};
    use rtic_monotonics::esp32c3::prelude::*;
    use rtt_target::{rprintln, rtt_init_print};

    // Register SysTimer as the monotonic timer for this platform
    esp32c3_systimer_monotonic!(Mono);

    #[local]
    struct Local {
        /// UART RX receives bytes which are framed into COBS packets
        uart_rx: UartRx<'static, Blocking>,
        /// [the_protocol_serde::Response]'s are sent back over UART TX
        _uart_tx: UartTx<'static, Blocking>,
        /// RGB led for showing the time of day
        _rgb_led: SmartLedsAdapter<ConstChannelAccess<esp_hal::rmt::Tx, 0>, 25>,
        
        // TODO: add missing local resources here as needed TODO: aggregation
        // buffer for commands which are received byte by byte
        cmd_buf: [u8; Command::MAX_SERIALIZED_LEN],
        /// Current length of cmd_buf.
        cmd_len: usize,
    }

    #[shared]
    struct Shared {
        // TODO: add missing shared resources here as needed
        /// Blink period in milliseconds. 0 = disabled.
        led_interval_ms: u64,
        /// Global counter for C2 / C9.
        counter: u64,
        /// LED output pin.
        led_pin: Output<'static>,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        rtt_init_print!();

        // Retrieve the program's name from the host's environment at time of
        // compilation
        rprintln!(env!("CARGO_CRATE_NAME"));

        rprintln!("`init`: enter");

        // Set up a monotonic timer. This may be helpful for making things
        // happen on a fixed timeline
        let systimer = cx.device.SYSTIMER;
        Mono::start(systimer);

        let peripherals = esp_hal::init(esp_hal::Config::default());

        let rmt = Rmt::new(peripherals.RMT, time::Rate::from_mhz(80u32)).unwrap();
        // We use one of the RMT channels to instantiate a `SmartLedsAdapter` which can be used
        // directly with all `smart_led` implementations
        let rmt_buffer = smart_led_buffer!(1);
        let rgb_led = SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO2, rmt_buffer);

        let (tx, rx) = (peripherals.GPIO21, peripherals.GPIO20);
        let mut serial = Uart::<'static>::new(
            peripherals.UART0,
            uart::Config::default().with_rx(uart::RxConfig::default().with_fifo_full_threshold(1)),
        )
        .unwrap()
        .with_rx(rx)
        .with_tx(tx);
        serial.listen(uart::UartInterrupt::RxFifoFull);

        let (uart_rx, uart_tx) = serial.split();

        // LED on GPIO7
        let cfg = OutputConfig::default();
        let led_pin = Output::new(peripherals.GPIO7, esp_hal::gpio::Level::Low, cfg);

        // Start the async blink loop task
        blink_led::spawn().ok();

        rprintln!("`init`: exit");

        (
            Shared {
                led_interval_ms: 0,
                counter: 0,
                led_pin,
            },
            Local {
                _uart_tx: uart_tx,
                uart_rx,
                _rgb_led: rgb_led,
                cmd_buf: [0; Command::MAX_SERIALIZED_LEN],
                cmd_len: 0,
            },
        )
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        rprintln!("`idle`: enter");
        #[allow(clippy::empty_loop)]
        loop {
            // TODO: idle task (priority = 0) may handle least urgent actions

            // Go to sleep
            // N.b., going to sleep causes a delay with RTT prints, causing output to not appear on
            // the RTT terminal. Enable with care.
            //unsafe { core::arch::asm!("wfi") };
        }
    }

    /// On UART0, aggregate incoming byte(s) to a buffer
    #[task(binds = UART0, priority = 3, local = [ uart_rx, cmd_buf, cmd_len ], shared = [])]
    fn receive_byte(cx: receive_byte::Context) {
        rprintln!("`receive_byte`: enter");

        // Unpend the interrupt. This is necessary to prevent the interrupt from
        // re-firing after this task completes.
        serial::unpend_rxfifo_full_int();

        let rx = cx.local.uart_rx;
        let unit_buf = &mut [0; 1];
        while let Result::Ok(1) = rx.read_buffered(unit_buf) {
            let byte = unit_buf[0];

            // TODO: aggregate bytes, construct a command

            if *cx.local.cmd_len < cx.local.cmd_buf.len() {
                cx.local.cmd_buf[*cx.local.cmd_len] = byte;
                *cx.local.cmd_len += 1;
            } else {
                // Buffer overflow -> corrupted frame
                send_response::spawn(Response::Rejected(RejectReason::CorruptedFrame)).ok();
                *cx.local.cmd_len = 0;
                return;
            }

            if serial::is_termination_byte(byte) {
                // Copy frame into an independent buffer for in-place deserialization
                let mut frame = [0u8; Command::MAX_SERIALIZED_LEN];
                let len = *cx.local.cmd_len;
                frame[..len].copy_from_slice(&cx.local.cmd_buf[..len]);

                // Reset original buffer
                *cx.local.cmd_len = 0;
                cx.local.cmd_buf.fill(0);

                // Decode from independent frame
                match Command::deserialize_in_place(&mut frame[..len]) {
                    Ok(cmd) => {
                        process_command::spawn(cmd).ok();
                    }
                    Err(_) => {
                        send_response::spawn(Response::Rejected(RejectReason::CorruptedFrame)).ok();
                    }
                }
                rprintln!("received termination byte ({})", byte);
            } else {
                rprintln!("received byte: {}", byte);
            }
        }

        rprintln!("receive_byte: exit");
    }

    // ======================= SEND RESPONSE ============================
    #[task(local = [_uart_tx], priority = 2)]
    async fn send_response(cx: send_response::Context, resp: Response) {
        // Using the serial helper which writes the serialized response
        // to UART.
        serial::send_response(resp, cx.local._uart_tx);
    }


    // ======================= PROCESS COMMAND ==========================
    #[task(priority = 2)]
    async fn process_command(_: process_command::Context, cmd: Command) {
        match cmd {
            Command::Reset => {
                rprintln!("Command recieved - Reset");
                reset::spawn().ok();
            }
            Command::Counter => {
                get_counter::spawn().ok();
            }
            Command::Immediate(f) => {
                process_funct::spawn(f).ok();
            }
            _ => {
                send_response::spawn(Response::Rejected(RejectReason::NotImplemented)).ok();
            }
        }
    }


        // ======================= FUNCT HANDLER ============================
    #[task(priority = 2, shared = [led_interval_ms, counter])]
    async fn process_funct(mut _cx: process_funct::Context, f: Funct) {
        match f {
            Funct::Increment => {
                increment_counter::spawn().ok();
            }
            Funct::EnableBlink { period_ms } => {
                set_led_interval::spawn(period_ms).ok();
            }
            Funct::DisableBlink => {
                set_led_interval::spawn(0).ok();
            }
            _ => {
                send_response::spawn(Response::Rejected(RejectReason::NotImplemented)).ok();
            }
        }
    }

    // =========================== C2: Increment =======================
    #[task(priority = 2, shared = [counter])]
    async fn increment_counter(mut cx: increment_counter::Context) {
        cx.shared.counter.lock(|c| *c += 1);
        send_response::spawn(Response::Ok(None)).ok();
    }

    // =========================== C9: Counter =========================
    #[task(priority = 2, shared = [counter])]
    async fn get_counter(mut cx: get_counter::Context) {
        let v = cx.shared.counter.lock(|c| *c);
        send_response::spawn(Response::Ok(Some(Payload::Counter(v)))).ok();
    }

    // =========================== C3/C4: Blink =======================
    #[task(priority = 2, shared = [led_interval_ms])]
    async fn set_led_interval(mut cx: set_led_interval::Context, ms: u64) {
        cx.shared.led_interval_ms.lock(|v| *v = ms);
        send_response::spawn(Response::Ok(None)).ok();
    }

    // =========================== C1: Reset ===========================
    #[task(priority = 2, shared = [led_interval_ms, counter, led_pin])]
    async fn reset(mut cx: reset::Context) {
        cx.shared.led_interval_ms.lock(|v| *v = 0);
        cx.shared.counter.lock(|c| *c = 0);
        cx.shared.led_pin.lock(|p| p.set_low());

        send_response::spawn(Response::Ok(None)).ok();
    }

    // ====================== BLINK LED LOOP ===========================
   
    #[task(shared = [led_interval_ms, led_pin], priority = 1)]
    async fn blink_led(mut cx: blink_led::Context) {
        loop {
            let ms = cx.shared.led_interval_ms.lock(|v| *v);

            if ms == 0 {
                cx.shared.led_pin.lock(|p| p.set_low());
                Mono::delay(200.millis()).await;
            } else {
                cx.shared.led_pin.lock(|p| p.toggle());
                Mono::delay(ms.millis()).await; // Toggle every specified period
            }
        }
    }
}
