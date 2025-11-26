//! Test full-duplex serial with RTIC
//!
//! Listens to UART, waiting for control character ('#'). Upon receiving control
//! character, returns "ok" over UART.
//!
//! The following wiring is assumed:
//! - TX => GPIO21
//! - RX => GPIO20

#![no_std]
#![no_main]

use esp_backtrace as _;

#[rtic::app(device = esp32c3, dispatchers = [FROM_CPU_INTR0])]
mod app {
    use core::fmt::Write;

    use esp_hal::{
        uart::{self, Uart, UartRx, UartTx},
        Blocking,
    };
    use rtic_sync::channel::{Receiver, Sender};
    use rtt_target::{rprintln, rtt_init_print};

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        uart_rx: UartRx<'static, Blocking>,
        uart_tx: UartTx<'static, Blocking>,

        response_sink: Sender<'static, (), 8>,
    }

    #[init]
    fn init(_cx: init::Context) -> (Shared, Local) {
        rtt_init_print!();

        // Retrieve the program's name from the host's environment at time of compilation
        rprintln!(env!("CARGO_CRATE_NAME"));

        let peripherals = esp_hal::init(esp_hal::Config::default());

        // Configure UART
        let (tx, rx) = (peripherals.GPIO21, peripherals.GPIO20);
        let mut serial = Uart::<'static>::new(peripherals.UART0, uart::Config::default())
            .unwrap()
            .with_rx(rx)
            .with_tx(tx);

        // Fire interrupt on receiving specific character
        serial.set_at_cmd(uart::AtCmdConfig::default().with_cmd_char(b'#'));
        serial.listen(uart::UartInterrupt::AtCmd);

        let (uart_rx, uart_tx) = serial.split();

        let (response_sink, response_src) = rtic_sync::make_channel!((), 8);

        responder::spawn(response_src).unwrap();

        (
            Shared {},
            Local {
                uart_rx,
                uart_tx,
                response_sink,
            },
        )
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        rprintln!("idle enter");

        // Sleep while not doing anything else
        loop {
            unsafe { core::arch::asm!("wfi") };
        }
    }

    #[task(binds = UART0, priority = 2, local = [uart_rx, response_sink])]
    fn receive_cmd(cx: receive_cmd::Context) {
        rprintln!("receive_cmd enter");

        // This is necessary to prevent the interrupt from re-firing
        // Replaces: `serial.reset_at_cmd_interrupt();`
        let uart0 = unsafe { esp32c3::UART0::steal() };
        uart0.int_clr().write(|w| w.at_cmd_char_det().bit(true));

        let mut buf = [0; 64];
        cx.local.uart_rx.read_buffered(&mut buf).unwrap();
        rprintln!("received bytes: {}", core::str::from_utf8(&buf).unwrap());

        // Enqueue a response
        cx.local.response_sink.try_send(()).unwrap();

        rprintln!("receive_cmd exit");
    }

    #[task(priority = 1, local = [ uart_tx ])]
    async fn responder(cx: responder::Context, mut response_src: Receiver<'static, (), 8>) {
        rprintln!("responder enter");

        while let Ok(()) = response_src.recv().await {
            rprintln!("sending response");
            cx.local.uart_tx.write_str("ok\r\n").ok();
        }

        rprintln!("responder exit");
    }
}
