//! Driver for the Pimoroni Pico Display Pack, an ST7789-based SPI LCD

use core::{cell::RefCell, str::from_utf8};
use display_interface::{DataFormat, DisplayError, WriteOnlyDataCommand};
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_rp::{
    gpio::{Level, Output, Pin},
    peripherals,
    spi::{Config, Phase, Polarity, Spi},
};
use embassy_sync::{
    blocking_mutex::{
        raw::{CriticalSectionRawMutex, NoopRawMutex},
        Mutex,
    },
    channel::Channel,
};
use embassy_time::Delay;
use embedded_graphics::{
    image::{Image, ImageRawLE},
    mono_font::MonoTextStyle,
    pixelcolor::Rgb565,
    prelude::*,
    text::Text,
};
use embedded_hal::spi::SpiDevice; // alternative: SpiDeviceWithConfig<raw::NoopRawMutex, Spi<p::SPI0, Blocking>, Output<p::PIN_17>>
use mipidsi::{
    models::ST7789,
    options::{ColorInversion, Orientation, Rotation, TearingEffect},
    Builder,
};
use profont;

pub struct LCDPeripherals {
    pub spi: peripherals::SPI0,
    // wrteonly MIPIDSI
    pub cs: peripherals::PIN_17,
    pub sclk: peripherals::PIN_18,
    pub mosi: peripherals::PIN_19,
    /// TX mode: 0 = command, 1 = data
    pub dc: peripherals::PIN_16,
    /// backlight
    pub bl_en: peripherals::PIN_20,
}

pub enum Message {
    Lock,
    SetName(&'static [u8; 4]),
}

#[embassy_executor::task]
pub async fn task(io: LCDPeripherals, msg: &'static Channel<CriticalSectionRawMutex, Message, 2>) {
    // backlight toggle
    let mut bl_en = Output::new(io.bl_en, Level::High);

    // write mode toggle
    let dc = Output::new(io.dc, Level::Low);

    // SPI interface
    let mut display_config = Config::default();
    display_config.frequency = 62_500_000u32;
    display_config.phase = Phase::CaptureOnSecondTransition;
    display_config.polarity = Polarity::IdleHigh;

    let spi = Spi::new_blocking_txonly(io.spi, io.sclk, io.mosi, display_config.clone());
    let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi));
    let configured_spi =
        SpiDeviceWithConfig::new(&spi_bus, Output::new(io.cs, Level::High), display_config);

    // MIPIDSI interface
    let interface = DisplayInterface {
        spi: configured_spi,
        dcx: dc,
    };
    let mut driver = Builder::new(ST7789, interface)
        .display_size(135, 240)
        .display_offset(52, 40)
        .invert_colors(ColorInversion::Inverted)
        .init(&mut Delay)
        .unwrap();

    driver
        .set_tearing_effect(TearingEffect::HorizontalAndVertical)
        .unwrap();

    // ui assets
    let text_style = MonoTextStyle::new(&profont::PROFONT_24_POINT, Rgb565::WHITE);

    let lock_data = ImageRawLE::new(include_bytes!("../images/lock-48x48.raw"), 48);
    let rotate_data = ImageRawLE::new(include_bytes!("../images/rotate-48x48.raw"), 48);
    let credentials_data = ImageRawLE::new(include_bytes!("../images/credentials-48x48.raw"), 48);
    let password_data = ImageRawLE::new(include_bytes!("../images/password-48x48.raw"), 48);

    let lock_image = Image::new(&lock_data, Point::new(0, 12));
    let rotate_image = Image::new(&rotate_data, Point::new(0, 100));
    let credentials_image = Image::new(&credentials_data, Point::new(86, 191));
    let password_image = Image::new(&password_data, Point::new(0, 191));

    // ui state
    let mut name: &'static [u8; 4] = b"INIT";

    // this weird rotation dance is to work around bugs in mipidsi - if reoriented,
    // it can't fill or draw all the way to the right, and offsets are wrong
    loop {
        match msg.receive().await {
            Message::Lock => bl_en.toggle(),
            Message::SetName(n) => name = n,
        }

        driver.set_orientation(Orientation::default()).unwrap();

        driver.clear(Rgb565::BLACK).unwrap();

        credentials_image.draw(&mut driver).unwrap();
        password_image.draw(&mut driver).unwrap();

        driver
            .set_orientation(Orientation::default().rotate(Rotation::Deg90))
            .unwrap();

        // switch icons
        lock_image.draw(&mut driver).unwrap();
        rotate_image.draw(&mut driver).unwrap();

        // selected password name
        Text::new(from_utf8(name).unwrap(), Point::new(78, 87), text_style)
            .draw(&mut driver)
            .unwrap();
    }
}

struct DisplayInterface<'a, SPI: SpiDevice, DCX: Pin> {
    spi: SPI,
    dcx: Output<'a, DCX>,
}

impl<'a, SPI: SpiDevice, DC: Pin> WriteOnlyDataCommand for DisplayInterface<'a, SPI, DC> {
    fn send_commands(
        &mut self,
        cmd: display_interface::DataFormat<'_>,
    ) -> Result<(), display_interface::DisplayError> {
        self.dcx.set_low();

        send_bytes(&mut self.spi, cmd).map_err(|_| DisplayError::BusWriteError)?;
        Ok(())
    }

    fn send_data(
        &mut self,
        buf: display_interface::DataFormat<'_>,
    ) -> Result<(), display_interface::DisplayError> {
        self.dcx.set_high();

        send_bytes(&mut self.spi, buf).map_err(|_| DisplayError::BusWriteError)?;
        Ok(())
    }
}

fn send_bytes<T: SpiDevice>(spi: &mut T, words: DataFormat<'_>) -> Result<(), T::Error> {
    match words {
        DataFormat::U8(slice) => spi.write(slice),
        DataFormat::U16(slice) => {
            use byte_slice_cast::*;
            spi.write(slice.as_byte_slice())
        }
        DataFormat::U16LE(slice) => {
            use byte_slice_cast::*;
            for v in slice.as_mut() {
                *v = v.to_le();
            }
            spi.write(slice.as_byte_slice())
        }
        DataFormat::U16BE(slice) => {
            use byte_slice_cast::*;
            for v in slice.as_mut() {
                *v = v.to_be();
            }
            spi.write(slice.as_byte_slice())
        }
        DataFormat::U8Iter(iter) => {
            let mut buf = [0; 32];
            let mut i = 0;

            for v in iter.into_iter() {
                buf[i] = v;
                i += 1;

                if i == buf.len() {
                    spi.write(&buf)?;
                    i = 0;
                }
            }

            if i > 0 {
                spi.write(&buf[..i])?;
            }

            Ok(())
        }
        DataFormat::U16LEIter(iter) => {
            use byte_slice_cast::*;
            let mut buf = [0; 32];
            let mut i = 0;

            for v in iter.map(u16::to_le) {
                buf[i] = v;
                i += 1;

                if i == buf.len() {
                    spi.write(&buf.as_byte_slice())?;
                    i = 0;
                }
            }

            if i > 0 {
                spi.write(&buf[..i].as_byte_slice())?;
            }

            Ok(())
        }
        DataFormat::U16BEIter(iter) => {
            use byte_slice_cast::*;
            let mut buf = [0; 64];
            let mut i = 0;
            let len = buf.len();

            for v in iter.map(u16::to_be) {
                buf[i] = v;
                i += 1;

                if i == len {
                    spi.write(&buf.as_byte_slice())?;
                    i = 0;
                }
            }

            if i > 0 {
                spi.write(&buf[..i].as_byte_slice())?;
            }

            Ok(())
        }
        _ => unimplemented!(),
    }
}
