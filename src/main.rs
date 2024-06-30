#![no_std]
#![no_main]

use embassy_futures::select::select;
use embassy_rp::gpio::{Input, Level, Output};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {
    let io = embassy_rp::init(Default::default());    
    let mut btn_a = Input::new(io.PIN_12, embassy_rp::gpio::Pull::Up);
    let mut btn_b = Input::new(io.PIN_13, embassy_rp::gpio::Pull::Up);
    let mut btn_x = Input::new(io.PIN_15, embassy_rp::gpio::Pull::Up);
    let mut btn_y = Input::new(io.PIN_16, embassy_rp::gpio::Pull::Up);
    let mut led_r = Output::new(io.PIN_6, Level::High);
    let mut led_g = Output::new(io.PIN_7, Level::High);
    let mut led_b = Output::new(io.PIN_8, Level::High);

    loop {
        select(btn_a.wait_for_any_edge(), btn_b.wait_for_any_edge()).await;
        
        if btn_a.is_high() {
            led_b.set_high();
        } else {
            led_b.set_low();
        }

        if btn_b.is_high() {
            led_r.set_high();
        } else {
            led_r.set_low();
        }
    }
}
