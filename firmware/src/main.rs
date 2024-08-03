#![no_std]
#![no_main]

mod debounce;
mod lcd;
mod secrets;
mod usb;

use core::future::Future;
use debounce::{Debounced, Debouncy};
use embassy_futures::select::{select4, Either4};
use embassy_rp::gpio::{Input, Level, Output, Pin};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use endec::{heapless::Vec, Endec};
use panic_probe as _;
use secrets::{CODE_LENGTH, PASS_COUNT};

static LCD: Channel<CriticalSectionRawMutex, lcd::Message, 2> = Channel::new();
static USB: Channel<CriticalSectionRawMutex, usb::Message, 2> = Channel::new();

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) {
    let io = embassy_rp::init(Default::default());

    let _led_r = Output::new(io.PIN_6, Level::High);
    let _led_g = Output::new(io.PIN_7, Level::High);
    let _led_b = Output::new(io.PIN_8, Level::High);

    let mut sw_a = Debounced::new(Input::new(io.PIN_12, embassy_rp::gpio::Pull::Up), 400);
    let mut sw_b = Debounced::new(Input::new(io.PIN_13, embassy_rp::gpio::Pull::Up), 400);
    let mut sw_x = Debounced::new(Input::new(io.PIN_14, embassy_rp::gpio::Pull::Up), 400);
    let mut sw_y = Debounced::new(Input::new(io.PIN_15, embassy_rp::gpio::Pull::Up), 400);

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

    // passwords, decrypted once the unlock code is available and then stored until reset
    let mut endec = Endec::new(0);
    let passwords: [Vec<u8, 64>; PASS_COUNT];

    // initial lock screen state: sliding window with index
    let mut code_window = [0u8; CODE_LENGTH];
    let mut code_ix = 0;

    LCD.send(lcd::Message::SetName(&secrets::PASS_NAMES[0]))
        .await;

    loop {
        let input = select4(
            sw_a.debounce(),
            sw_b.debounce(),
            sw_x.debounce(),
            sw_y.debounce(),
        )
        .await;

        let key = slide_window(input, &mut code_window, &mut code_ix);

        match endec.dec(&key, &secrets::CODE_BUTTONS) {
            Ok(buffer) if buffer.as_slice() == b"CODE_BUTTONS" => {
                let mut i: u8 = 1;
                passwords = secrets::PASS_WORDS.map(|secret| {
                    let mut endec = Endec::new(i);
                    i = i + 1;
                    endec.dec(&key, &secret).unwrap()
                });

                code_window = [0; CODE_LENGTH];
                LCD.send(lcd::Message::Unlock).await;
                break;
            }
            _ => (),
        }
    }

    // extra app state: lock state, current selected password
    let mut unlocked = true;
    let mut cred_ix = 0;

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
                    cred_ix = (cred_ix + 1) % secrets::PASS_COUNT;
                    LCD.send(lcd::Message::SetName(&secrets::PASS_NAMES[cred_ix]))
                        .await;
                }
                Either4::Third(_) => {
                    let username = secrets::PASS_USERS[cred_ix];
                    let password = passwords[cred_ix].clone();

                    USB.send(usb::Message::Credentials { username, password })
                        .await;
                }
                Either4::Fourth(_) => {
                    let password = passwords[cred_ix].clone();
                    USB.send(usb::Message::Password { password }).await;
                }
            }

        // foo
        } else {
            let key = slide_window(input, &mut code_window, &mut code_ix);

            match endec.dec(&key, &secrets::CODE_BUTTONS) {
                Ok(buffer)
                    if buffer.as_slice() == b"The quick brown fox jumps over the lazy dog." =>
                {
                    code_window = [0; CODE_LENGTH];
                    unlocked = true;
                    LCD.send(lcd::Message::Unlock).await;
                }
                _ => {
                    LCD.send(lcd::Message::Wake).await;
                }
            }
        }
    }
}

fn slide_window(
    input: Either4<(), (), (), ()>,
    code_window: &mut [u8; CODE_LENGTH],
    code_ix: &mut usize,
) -> [u8; 32] {
    let code_element = match input {
        Either4::First(_) => b'a',
        Either4::Second(_) => b'b',
        Either4::Third(_) => b'x',
        Either4::Fourth(_) => b'y',
    };

    code_window[*code_ix] = code_element;
    *code_ix = (*code_ix + 1) % CODE_LENGTH;

    let sliding_window = code_window[*code_ix..=(CODE_LENGTH - 1)]
        .iter()
        .chain(code_window[0..*code_ix].iter());

    let mut sliding_bytes = [0u8; CODE_LENGTH];

    let mut i = 0;
    for b in sliding_window {
        sliding_bytes[i] = *b;
        i += 1;
    }

    Endec::make_key(&sliding_bytes)
}

impl<'a, T: Pin> Debouncy for Input<'a, T> {
    type Output = ();

    fn read(&mut self) -> impl Future<Output = ()> {
        self.wait_for_falling_edge()
    }
}
