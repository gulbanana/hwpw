#![no_std]
#![no_main]

use embassy_rp::gpio::{Input, Level, Output};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {
    let io = embassy_rp::init(Default::default());    
    let mut btn_a = Input::new(io.PIN_12, embassy_rp::gpio::Pull::Up);
    let mut led_r = Output::new(io.PIN_6, Level::High);
    let mut led_g = Output::new(io.PIN_7, Level::High);
    let mut led_b = Output::new(io.PIN_8, Level::High);

    loop {
        btn_a.wait_for_any_edge().await;
        if btn_a.is_high() {
            led_r.set_high();
            led_g.set_high();
            led_b.set_high();
        } else {
            led_r.set_low();
            led_g.set_low();
            led_b.set_low();
        }        
    }
}
