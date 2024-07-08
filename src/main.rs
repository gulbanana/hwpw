#![no_std]
#![no_main]

mod lcd;
mod secrets;
mod usb;

use embassy_futures::select::{select4, Either4};
use embassy_rp::gpio::{Input, Level, Output, Pin};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Instant};
use panic_probe as _;

static LCD: Channel<CriticalSectionRawMutex, lcd::Message, 2> = Channel::new();
static USB: Channel<CriticalSectionRawMutex, usb::Message, 2> = Channel::new();

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) {
    let io = embassy_rp::init(Default::default());

    let _led_r = Output::new(io.PIN_6, Level::High);
    let _led_g = Output::new(io.PIN_7, Level::High);
    let _led_b = Output::new(io.PIN_8, Level::High);

    let mut sw_a = Debounced::new(Input::new(io.PIN_12, embassy_rp::gpio::Pull::Up));
    let mut sw_b = Debounced::new(Input::new(io.PIN_13, embassy_rp::gpio::Pull::Up));
    let mut sw_x = Debounced::new(Input::new(io.PIN_14, embassy_rp::gpio::Pull::Up));
    let mut sw_y = Debounced::new(Input::new(io.PIN_15, embassy_rp::gpio::Pull::Up));

    let lcd = lcd::LCDPeripherals {
        spi: io.SPI0,
        dc: io.PIN_16,
        cs: io.PIN_17,
        sclk: io.PIN_18,
        mosi: io.PIN_19,
        bl_en: io.PIN_20,
    };

    spawner.spawn(lcd::task(lcd, &LCD)).unwrap();
    spawner.spawn(usb::task(io.USB, &USB)).unwrap();

    // app state: current selected password, sliding window for unlock code
    let mut unlocked = false;
    let mut code_window = [0u8, 0, 0, 0];
    let mut cred_ix = 0;
    let mut code_ix = 0;

    LCD.send(lcd::Message::SetName(&secrets::DISPLAY_NAMES[cred_ix]))
        .await;

    loop {
        let input = select4(
            sw_a.debounce(),
            sw_b.debounce(),
            sw_x.debounce(),
            sw_y.debounce(),
        )
        .await;

        // activate commands
        if unlocked {
            match input {
                Either4::First(_) => {
                    unlocked = false;
                    LCD.send(lcd::Message::Lock).await;
                }
                Either4::Second(_) => {
                    cred_ix = (cred_ix + 1) % secrets::COUNT;
                    LCD.send(lcd::Message::SetName(&secrets::DISPLAY_NAMES[cred_ix]))
                        .await;
                }
                Either4::Third(_) => {
                    let username = secrets::USERNAMES[cred_ix];
                    let password = secrets::PASSWORDS[cred_ix];
                    USB.send(usb::Message::Credentials { username, password })
                        .await;
                }
                Either4::Fourth(_) => {
                    let password = secrets::PASSWORDS[cred_ix];
                    USB.send(usb::Message::Password { password }).await;
                }
            }

        // foo
        } else {
            let code_element = match input {
                Either4::First(_) => b'a',
                Either4::Second(_) => b'b',
                Either4::Third(_) => b'x',
                Either4::Fourth(_) => b'y',
            };

            code_window[code_ix] = code_element;
            code_ix = (code_ix + 1) % code_window.len();

            let sliding_window = code_window[code_ix..=3]
                .iter()
                .chain(code_window[0..code_ix].iter());

            if secrets::PASSCODE.iter().eq(sliding_window) {
                code_window = [0, 0, 0, 0];
                unlocked = true;
                LCD.send(lcd::Message::Unlock).await;
            } else {
                LCD.send(lcd::Message::Wake).await;
            }
        }
    }
}

struct Debounced<'a, T: Pin> {
    input: Input<'a, T>,
    deadline: Instant,
}

impl<'a, T: Pin> Debounced<'a, T> {
    fn new(input: Input<'a, T>) -> Self {
        Debounced {
            input,
            deadline: Instant::now(),
        }
    }

    async fn debounce(&mut self) {
        loop {
            self.input.wait_for_falling_edge().await;
            if Instant::now() >= self.deadline {
                self.deadline = Instant::now() + Duration::from_millis(400);
                break;
            }
        }
    }
}
