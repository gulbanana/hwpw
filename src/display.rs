//! Driver for the Pimoroni Pico Display Pack, an ST7789-based SPI LCD

use core::cell::RefCell;
use display_interface::{DataFormat, DisplayError, WriteOnlyDataCommand};
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_rp::{
    gpio::{Level, Output, Pin},
    peripherals,
    spi::{Config, Phase, Polarity, Spi},
};
use embassy_sync::{
    blocking_mutex::{
        raw::{NoopRawMutex, ThreadModeRawMutex},
        Mutex,
    },
    channel::Channel,
};
use embassy_time::Delay;
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
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
    Left,
    Right,
    Up,
    Down,
}

#[embassy_executor::task]
pub async fn task(io: LCDPeripherals, msg: &'static Channel<ThreadModeRawMutex, Message, 2>) {
    // backlight toggle
    let _bl_en = Output::new(io.bl_en, Level::High);

    // write mode toggle
    let dc = Output::new(io.dc, Level::Low);

    // SPI interface
    let mut display_config = Config::default();
    display_config.frequency = 64_000_000u32;
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
        .display_offset(40, 53)
        .invert_colors(ColorInversion::Inverted)
        .init(&mut Delay)
        .unwrap();

    driver
        .set_orientation(
            Orientation::default()
                .rotate(Rotation::Deg270)
                .flip_vertical()
                .flip_horizontal(),
        )
        .unwrap();
    driver
        .set_tearing_effect(TearingEffect::HorizontalAndVertical)
        .unwrap();

    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::BLACK);
    let mut text_x = 60i32;
    let mut text_y = 15i32;

    loop {
        // three-colour background - fill_solid() and therefore clear() currently doesn't work (might be trying to use size without offset)
        driver
            .set_pixels(0, 0, 79, 134, (0..(135u32 * 80u32)).map(|_| Rgb565::GREEN))
            .unwrap();

        driver
            .set_pixels(160, 0, 239, 134, (0..(135u32 * 80u32)).map(|_| Rgb565::RED))
            .unwrap();

        driver
            .set_pixels(80, 0, 159, 134, (0..(135u32 * 80u32)).map(|_| Rgb565::WHITE))
            .unwrap();

        // floating text, movable with switches
        Text::new("Ciao, mondo!", Point::new(text_x, text_y), text_style)
            .draw(&mut driver)
            .unwrap();

        match msg.receive().await {
            Message::Left => text_x = text_x - 5,
            Message::Right => text_x = text_x + 5,
            Message::Up => text_y = text_y - 5,
            Message::Down => text_y = text_y + 5,
        }
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
