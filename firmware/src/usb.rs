use crate::{
    debounce::{Debounced, Debouncy},
    secrets::PASS_WORDS,
};
use core::{
    future::Future,
    sync::atomic::{AtomicBool, Ordering},
};
use embassy_futures::join::join;
use embassy_rp::{
    bind_interrupts,
    peripherals::USB,
    usb::{Driver, InterruptHandler},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_usb::{
    class::hid::{self, HidWriter},
    Builder, Config, Handler,
};
use endec::Endec;
use usbd_hid::descriptor::{KeyboardReport, KeyboardUsage::*, SerializedDescriptor};

pub enum Message {
    Credentials {
        username: &'static [u8],
        password_ix: usize,
        password_key: [u8; 32],
    },
    Password {
        password_ix: usize,
        password_key: [u8; 32],
    },
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
    config.product = Some("Password Manager");
    config.serial_number = Some("00000001");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // HID config
    let mut hid_state = hid::State::new(); // must be dropped before builder
    let hid_config = hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: None,
        poll_ms: 8,
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
        let mut keyboard = Keyboard::new(&mut hid);
        let mut msg = Debounced::new(msg, 1200);
        loop {
            match msg.debounce().await {
                Message::Credentials {
                    username,
                    password_ix,
                    password_key,
                } => {
                    let mut endec: Endec = Endec::new(password_ix as u8 + 1);
                    let password = endec.dec(&password_key, &PASS_WORDS[password_ix]).unwrap();

                    keyboard.send_str(username).await;
                    keyboard.send_key(KeyboardTab as u8, false).await;
                    keyboard.send_str(password).await;
                    keyboard.send_key(KeyboardEnter as u8, false).await;
                }
                Message::Password {
                    password_ix,
                    password_key,
                } => {
                    let mut endec: Endec = Endec::new(password_ix as u8 + 1);
                    let password = endec.dec(&password_key, &PASS_WORDS[password_ix]).unwrap();

                    keyboard.send_str(password).await;
                    keyboard.send_key(KeyboardEnter as u8, false).await;
                }
            }
        }
    };

    join(usb_future, msg_future).await;
}

impl<T> Debouncy for &Channel<CriticalSectionRawMutex, T, 2> {
    type Output = T;

    fn read(&mut self) -> impl Future<Output = T> {
        self.receive()
    }
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

struct Keyboard<'a> {
    hid: &'a mut HidWriter<'a, Driver<'a, USB>, 8>,
}

impl<'a> Keyboard<'a> {
    fn new(hid: &'a mut HidWriter<'a, Driver<'a, USB>, 8>) -> Self {
        Keyboard { hid }
    }

    async fn send_key(&mut self, keycode: u8, shift: bool) {
        let report = KeyboardReport {
            keycodes: [keycode, 0, 0, 0, 0, 0],
            leds: 0,
            modifier: if shift { 0x02 } else { 0 },
            reserved: 0,
        };
        self.hid.write_serialize(&report).await.unwrap();

        let report = KeyboardReport {
            keycodes: [0, 0, 0, 0, 0, 0],
            leds: 0,
            modifier: 0,
            reserved: 0,
        };
        self.hid.write_serialize(&report).await.unwrap();
    }

    async fn send_str(&mut self, value: &[u8]) {
        for char in value.iter() {
            let (keycode, shift) = match *char {
                b' ' => (KeyboardSpacebar, false),
                b'!' => (Keyboard1Exclamation, true),
                b'"' => (KeyboardSingleDoubleQuote, true),
                b'#' => (Keyboard3Hash, true),
                b'$' => (Keyboard4Dollar, true),
                b'%' => (Keyboard5Percent, true),
                b'&' => (Keyboard7Ampersand, true),
                b'\'' => (Keyboard3Hash, true),
                b'(' => (Keyboard9OpenParens, true),
                b')' => (Keyboard0CloseParens, true),
                b'*' => (Keyboard8Asterisk, true),
                b'+' => (KeyboardEqualPlus, true),
                b',' => (KeyboardCommaLess, false),
                b'-' => (KeyboardDashUnderscore, false),
                b'.' => (KeyboardPeriodGreater, false),
                b'/' => (KeyboardSlashQuestion, false),

                b'0' => (Keyboard0CloseParens, false),
                numeric if numeric >= b'1' && numeric < b'9' => {
                    ((Keyboard1Exclamation as u8 + numeric - b'1').into(), false)
                }

                b':' => (KeyboardSemiColon, true),
                b';' => (KeyboardSemiColon, false),
                b'<' => (KeyboardCommaLess, true),
                b'=' => (KeyboardEqualPlus, false),
                b'>' => (KeyboardPeriodGreater, true),
                b'?' => (KeyboardSlashQuestion, true),
                b'@' => (Keyboard2At, true),

                upper if upper >= b'A' && upper <= b'Z' => {
                    ((KeyboardAa as u8 + upper - b'A').into(), true)
                }

                b'[' => (KeyboardOpenBracketBrace, false),
                b'\\' => (KeyboardBackslashBar, false),
                b']' => (KeyboardCloseBracketBrace, false),
                b'^' => (Keyboard6Caret, true),
                b'_' => (KeyboardDashUnderscore, true),
                b'`' => (KeyboardBacktickTilde, false),

                lower if lower >= b'a' && lower <= b'z' => {
                    ((KeyboardAa as u8 + lower - b'a').into(), false)
                }

                b'{' => (KeyboardOpenBracketBrace, true),
                b'|' => (KeyboardBackslashBar, true),
                b'}' => (KeyboardCloseBracketBrace, true),
                b'~' => (KeyboardBacktickTilde, true),

                _ => (KeyboardSlashQuestion, true),
            };

            self.send_key(keycode as u8, shift).await
        }
    }
}
