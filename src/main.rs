#![no_std]
#![no_main]

use embassy_rp::gpio::{Input, Level, Output};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {
    let io = embassy_rp::init(Default::default());    
    let mut button = Input::new(io.PIN_16, embassy_rp::gpio::Pull::None);
    let mut led = Output::new(io.PIN_15, Level::Low);

    loop {
        button.wait_for_any_edge().await;
        if button.is_high() {
            led.set_high();
        } else {
            led.set_low();
        }        
    }
}
