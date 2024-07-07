#![no_std]
#![no_main]

mod lcd;
mod secrets;
mod usb;

use embassy_futures::select::{select4, Either4};
use embassy_rp::gpio::{Input, Level, Output};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::Timer;
use panic_probe as _;

static LCD: Channel<CriticalSectionRawMutex, lcd::Message, 2> = Channel::new();
static USB: Channel<CriticalSectionRawMutex, usb::Message, 2> = Channel::new();

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) {
    let io = embassy_rp::init(Default::default());

    let _led_r = Output::new(io.PIN_6, Level::High);
    let _led_g = Output::new(io.PIN_7, Level::High);
    let _led_b = Output::new(io.PIN_8, Level::High);

    let mut sw_a = Input::new(io.PIN_12, embassy_rp::gpio::Pull::Up);
    let mut sw_b = Input::new(io.PIN_13, embassy_rp::gpio::Pull::Up);
    let mut sw_x = Input::new(io.PIN_14, embassy_rp::gpio::Pull::Up);
    let mut sw_y = Input::new(io.PIN_15, embassy_rp::gpio::Pull::Up);

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

    let mut ix = 0;
    LCD.send(lcd::Message::SetName(&secrets::DISPLAY_NAMES[ix]))
        .await;

    loop {
        match select4(
            sw_a.wait_for_falling_edge(),
            sw_b.wait_for_falling_edge(),
            sw_x.wait_for_falling_edge(),
            sw_y.wait_for_falling_edge(),
        )
        .await
        {
            Either4::First(_) => {
                LCD.send(lcd::Message::Lock).await;
            }
            Either4::Second(_) => {
                ix = (ix + 1) % secrets::COUNT;
                LCD.send(lcd::Message::SetName(&secrets::DISPLAY_NAMES[ix]))
                    .await;
            }
            Either4::Third(_) => {
                let username = secrets::USERNAMES[ix];
                let password = secrets::PASSWORDS[ix];
                USB.send(usb::Message::Credentials { username, password })
                    .await;
            }
            Either4::Fourth(_) => {
                let password = secrets::PASSWORDS[ix];
                USB.send(usb::Message::Password { password }).await;
            }
        }

        Timer::after_millis(400).await; // world's dumbest debounce
    }
}
