#![cfg(all(feature = "firmware", target_arch = "riscv32"))]
#![no_std]
#![no_main]

use panic_rtt_target as _;

#[rtic::app(device = esp32c3, dispatchers = [FROM_CPU_INTR0, FROM_CPU_INTR1])]
mod app {
    use rtt_target::{rprintln, rtt_init_print};
    use esp_hal::{
        gpio::{Event, Input, InputConfig, Pull, Output, OutputConfig, Level},
        time::Duration,
        delay::Delay,
    };

    #[derive(Clone, Copy)]
    pub enum Command {
        C1Reset,
        C2Increment,
        C3EnableBlink,
        C4DisableBlink,
        C5EnableRgb,
        C6DisableRgb,
        C7ScheduleIncrement,
        C8SetDateTime,
        C9Counter,
    }

    pub struct AppState {
        counter: u32,
        blinking: bool,
    }

    impl AppState {
        fn new() -> Self {
            Self {
                counter: 0,
                blinking: false,
            }
        }
    }

    #[shared]
    struct Shared {
        state: AppState,
    }

    #[local]
    struct Local {
        button: Input<'static>,
        led_pin: Output<'static>,
        next_cmd: u8,
        avg_interval: Duration,
        avg_press_duration: Duration,
    }

    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        rtt_init_print!();
        rprintln!("Testing commands");

        let peripherals = esp_hal::init(esp_hal::Config::default());

        let config = InputConfig::default().with_pull(Pull::Up);
        let mut button = Input::new(peripherals.GPIO9, config);
        button.listen(Event::AnyEdge);

        let led_pin = Output::new(peripherals.GPIO7, Level::Low, OutputConfig::default());

        let avg_press_duration = Duration::from_millis(1000);
        let avg_interval = Duration::from_millis(1000);

        (
            Shared { state: AppState::new() },
            Local {
                button,
                next_cmd: 0,
                avg_press_duration,
                avg_interval,
                led_pin,
            },
        )
    }

    #[task(shared = [state], local = [led_pin, avg_press_duration, avg_interval])]
    async fn blink(mut cx: blink::Context) {
        let delay = Delay::new();

        loop {
            let enable_blinking = cx.shared.state.lock(|s| s.blinking);

            if enable_blinking {
                let on_time_ms = cx.local.avg_press_duration.as_millis() as u32;
                cx.local.led_pin.set_high();
                delay.delay_millis(on_time_ms);

                cx.local.led_pin.set_low();
                let interval_ms = cx.local.avg_interval.as_millis() as u32;
                delay.delay_millis(interval_ms);
            }
        }
    }

    #[task(shared = [state], local = [next_cmd])]
    async fn exec_command(mut cx: exec_command::Context) {
        let cmd = match *cx.local.next_cmd {
            0 => Command::C1Reset,
            1 => Command::C2Increment,
            2 => Command::C3EnableBlink,
            3 => Command::C4DisableBlink,
            4 => Command::C9Counter,
            _ => Command::C1Reset,
        };

        *cx.local.next_cmd = (*cx.local.next_cmd + 1) % 4;

        cx.shared.state.lock(|s| {
            match cmd {
                Command::C1Reset => {
                    rprintln!("C1 Reset → state cleared");
                }
                Command::C2Increment => {
                    s.counter += 1;
                    rprintln!("C2 Increment → counter={}", s.counter);
                }
                Command::C3EnableBlink => {
                    s.blinking = true;
                    rprintln!("C3 EnableBlink");
                    blink::spawn().ok();
                }
                Command::C4DisableBlink => {
                    s.blinking = false;
                    rprintln!("C4 DisableBlink");
                }
                Command::C9Counter => {
                    rprintln!("C9 Counter → {}", s.counter);
                }
                _ => {}
            }
        });
    }

    #[task(binds = GPIO, local = [button])]
    fn button(cx: button::Context) {
        cx.local.button.clear_interrupt();
        if cx.local.button.is_low() {
            rprintln!("BUTTON → executing next command");
            exec_command::spawn().ok();
        }
    }
}
