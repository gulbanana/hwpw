use core::sync::atomic::{AtomicBool, Ordering};

use embassy_futures::join::join;
use embassy_rp::{bind_interrupts, peripherals::USB, usb::{Driver, InterruptHandler}};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_usb::{class::hid::{self, HidWriter}, Builder, Config, Handler};
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};

pub enum Message
{
    X,
    Y
}

#[embassy_executor::task]
pub async fn task(io: USB, msg: &'static Channel<CriticalSectionRawMutex, Message, 2>) {
    // descriptor buffers
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut msos_descriptor = [0; 256];
    let mut control_buf = [0; 64];

    // callbacks
    let mut control_handler = ControlHandler(AtomicBool::new(false));

    // stack config
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Bananasoft");
    config.product = Some("Password manager");
    config.serial_number = Some("00000001");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // HID config
    let mut hid_state = hid::State::new(); // must be dropped before builder
    let hid_config = hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: None,
        poll_ms: 255,
        max_packet_size: 64,
    };

    // build device
    let mut builder = Builder::new(
        Driver::new(io, Irqs),
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut msos_descriptor,
        &mut control_buf,
    );    
    builder.handler(&mut control_handler);
    let mut hid = HidWriter::<_, 8>::new(&mut builder, &mut hid_state, hid_config);
    let mut device = builder.build();

    // concurrently, run the USB stack and a message-handling loop
    let usb_future = device.run();
    let msg_future = async {
        loop {
            match msg.receive().await {
                Message::X => send_key(&mut hid, 0x1b).await,
                Message::Y => send_key(&mut hid, 0x1c).await
            }    
        }
    };

    join(usb_future, msg_future).await;
}

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

struct ControlHandler(AtomicBool);

impl Handler for ControlHandler {
    fn reset(&mut self) {
        self.0.store(false, Ordering::Relaxed);
    }
    
    fn enabled(&mut self, _enabled: bool) {
        self.0.store(false, Ordering::Relaxed);
    }

    fn addressed(&mut self, _addr: u8) {
        self.0.store(false, Ordering::Relaxed);
    }

    fn configured(&mut self, configured: bool) {
        self.0.store(configured, Ordering::Relaxed);
    }
}

async fn send_key<'a>(hid: &mut HidWriter<'a, Driver<'a, USB>, 8>, code: u8) {
    let report = KeyboardReport {
        keycodes: [code, 0, 0, 0, 0, 0],
        leds: 0,
        modifier: 0,
        reserved: 0,
    };
    hid.write_serialize(&report).await.unwrap();

    let report = KeyboardReport {
        keycodes: [0, 0, 0, 0, 0, 0],
        leds: 0,
        modifier: 0,
        reserved: 0,
    };
    hid.write_serialize(&report).await.unwrap();            
}