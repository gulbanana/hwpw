#![no_std]
#![no_main]

mod lcd;
mod usb;

use embassy_futures::select::{select4, Either4};
use embassy_rp::gpio::{Input, Level, Output};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
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
                LCD.send(lcd::Message::Left).await;
            }
            Either4::Second(_) => {
                LCD.send(lcd::Message::Right).await;
            }
            Either4::Third(_) => {
                USB.send(usb::Message::X).await;
            }
            Either4::Fourth(_) => {
                USB.send(usb::Message::Y).await;
            }
        }
    }
}
