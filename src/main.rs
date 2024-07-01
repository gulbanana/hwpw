#![no_std]
#![no_main]

mod display;

use embassy_futures::select::select;
use embassy_rp::gpio::{Input, Level, Output};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) {
    let io = embassy_rp::init(Default::default());

    let mut led_r = Output::new(io.PIN_6, Level::High);
    let mut led_g = Output::new(io.PIN_7, Level::High);
    let mut led_b = Output::new(io.PIN_8, Level::High);

    let mut sw_a = Input::new(io.PIN_12, embassy_rp::gpio::Pull::Up);
    let mut sw_b = Input::new(io.PIN_13, embassy_rp::gpio::Pull::Up);
    let mut sw_x = Input::new(io.PIN_14, embassy_rp::gpio::Pull::Up);
    let mut sw_y = Input::new(io.PIN_15, embassy_rp::gpio::Pull::Up);

    let lcd = display::LCDPeripherals {
        spi: io.SPI0,
        dc: io.PIN_16,
        cs: io.PIN_17,
        sclk: io.PIN_18,
        mosi: io.PIN_19,
        bl_en: io.PIN_20,
    };

    spawner.spawn(display::task(lcd)).unwrap();

    loop {
        select(sw_a.wait_for_any_edge(), sw_b.wait_for_any_edge()).await;

        if sw_a.is_high() {
            led_b.set_high(); // could this use PWM?
        } else {
            led_b.set_low();
        }

        if sw_b.is_high() {
            led_r.set_high();
        } else {
            led_r.set_low();
        }
    }
}
