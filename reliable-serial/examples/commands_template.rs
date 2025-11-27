
#![no_std]
#![no_main]

use panic_rtt_target as _;

#[rtic::app(device = esp32c3, dispatchers = [FROM_CPU_INTR0, FROM_CPU_INTR1])]
mod app {
    use rtt_target::{rprintln, rtt_init_print};
    use esp_hal::{
        rmt::{ConstChannelAccess, Rmt},
        time,
        gpio::{Event, Input, InputConfig, Pull, Output, OutputConfig, Level},
        time::Duration,
        delay::Delay,
    };

    use core::iter;
    use rtic_monotonics::esp32c3::prelude::*;
    use esp_hal_smartled::{smart_led_buffer, SmartLedsAdapter, buffer_size};
    use smart_leds::{SmartLedsWrite, brightness, RGB8};
    use smart_leds::colors::*; 

    use esp_hal::time::Rate;

    esp32c3_systimer_monotonic!(Mono);

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
        rgb_on: bool,
    }

    impl AppState {
        fn new() -> Self {
            Self {
                counter: 0,
                blinking: false,
                rgb_on: false,
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
        rgb_led: SmartLedsAdapter<ConstChannelAccess<esp_hal::rmt::Tx, 0>, 25>,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        let systimer = cx.device.SYSTIMER;
        Mono::start(systimer);

        rtt_init_print!();
        rprintln!("Testing commands");
        
        let rmt_buffer = smart_led_buffer!(1);
        
        let peripherals = esp_hal::init(esp_hal::Config::default());

        let config = InputConfig::default().with_pull(Pull::Up);
        let mut button = Input::new(peripherals.GPIO9, config);
        button.listen(Event::AnyEdge);

        let led_pin = Output::new(peripherals.GPIO7, Level::Low, OutputConfig::default());

        let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();


        let avg_press_duration = Duration::from_millis(1000);
        let avg_interval = Duration::from_millis(1000);

        let rgb_led = SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO2, rmt_buffer);

        (
            Shared { state: AppState::new() },
            Local {
                button,
                next_cmd: 0,
                avg_press_duration,
                avg_interval,
                led_pin,
                rgb_led
            },
        )
    }
    
    #[task(shared = [state], local = [rgb_led])]
    async fn rgb(mut cx: rgb::Context) {

        let light_blue = RGB8 { r: 80, g: 180, b: 255 };

        cx.local.rgb_led.write(brightness(core::iter::once(light_blue), 10)).unwrap();

    }
    
    #[task(shared = [state], local = [led_pin, avg_press_duration, avg_interval])]
    async fn blink(mut cx: blink::Context) {
        let delay = Delay::new();

        loop {
            let enable_blinking = cx.shared.state.lock(|s| s.blinking);

            if enable_blinking {
                let on_time_ms = cx.local.avg_press_duration.as_millis() as u32;
                cx.local.led_pin.toggle();
                Mono::delay(200.millis()).await; // Toggle every specified period
            } else {
                cx.local.led_pin.set_low();
                Mono::delay(200.millis()).await; 

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
            4 => Command::C5EnableRgb,
            5 => Command::C6DisableRgb,
            6 => Command::C9Counter,
            _ => Command::C1Reset,
        };

        *cx.local.next_cmd = (*cx.local.next_cmd + 1) % 7;
        rprintln!("ncrement → counter={}", *cx.local.next_cmd);

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
                Command::C5EnableRgb => {
                    s.rgb_on = true;
                    rprintln!("C5 EnableRgb");
                    rgb::spawn().ok();
                }
                Command::C6DisableRgb => {
                    s.rgb_on = false;
                    rprintln!("C6 DisableRgb");
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
