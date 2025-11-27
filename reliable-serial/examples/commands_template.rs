
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
        hour: u8,
    }

    impl AppState {
        fn new() -> Self {
            Self {
                counter: 0,
                blinking: false,
                rgb_on: false,
                hour: 0
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
        rgb::spawn().ok();

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





        let off_color = RGB8 { r: 0, g: 0, b: 0 };

        loop {
            let enable_rgb = cx.shared.state.lock(|s| s.rgb_on);

            if enable_rgb {
                let hour: u8 = cx.shared.state.lock(|s| s.hour); 

                let color = match hour {
                    3..=9   => RGB8 { r: 0xF8, g: 0xF3, b: 0x2B }, // Dawn, Aureolin
                    9..=15  => RGB8 { r: 0x9C, g: 0xFF, b: 0xFA }, // Noon, Ice blue
                    15..=21 => RGB8 { r: 0x05, g: 0x3C, b: 0x5E }, // Evening, Indigo dye
                    21..=24 => RGB8 { r: 0x31, g: 0x08, b: 0x1F }, // Night, Dark purple
                    0..=3 => RGB8 { r: 0x31, g: 0x08, b: 0x1F }, // Night, Dark purple
                    _       => RGB8 { r: 0, g: 0, b: 0 },
                };
                cx.local.rgb_led.write(brightness(core::iter::once(color), 10)).unwrap();
                Mono::delay(200.millis()).await; 

            } else {
                cx.local.rgb_led.write(brightness(core::iter::once(off_color), 10)).unwrap();
                Mono::delay(200.millis()).await; 

            }
        }

        

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
            5 => Command::C8SetDateTime,
            6 => Command::C6DisableRgb,
            7 => Command::C9Counter,
            _ => Command::C1Reset,
        };

        *cx.local.next_cmd = (*cx.local.next_cmd + 1) % 8;
        

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

                }
                Command::C6DisableRgb => {
                    s.rgb_on = false;
                    rprintln!("C6 DisableRgb");
                }
                Command::C8SetDateTime => {
                    s.hour = 15;
                    rprintln!("C6 SetDateTime");
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
