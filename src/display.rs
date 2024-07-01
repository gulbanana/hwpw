//! Driver for the Pimoroni Pico Display Pack, an ST7789-based SPI LCD

use core::cell::RefCell;
use display_interface::{DataFormat, DisplayError, WriteOnlyDataCommand};
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_rp::gpio::{AnyPin, Level, Output};
use embassy_rp::peripherals as p;
use embassy_rp::spi::{Blocking, Config, Phase, Polarity, Spi};
use embassy_sync::blocking_mutex::*;
use embassy_time::{Delay, Timer};
use embedded_graphics::prelude::*;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_hal::digital::OutputPin; // could these be embassy hal traits instead?
use embedded_hal::spi::SpiDevice;
use st7789::{Orientation, ST7789};

const DISPLAY_FREQ: u32 = 64_000_000;

pub struct LCDPeripherals {
    pub spi: p::SPI0,
    /// 0 = command, 1 = data
    pub dc: p::PIN_16,
    pub cs: p::PIN_17,
    pub sclk: p::PIN_18,
    pub mosi: p::PIN_19,
    pub bl_en: p::PIN_20,
}

#[embassy_executor::task]
pub async fn task(p: LCDPeripherals) {
    let mut display_config = Config::default();
    display_config.frequency = DISPLAY_FREQ;
    display_config.phase = Phase::CaptureOnSecondTransition;
    display_config.polarity = Polarity::IdleHigh;

    let spi: Spi<'_, _, Blocking> =
        Spi::new_blocking_txonly(p.spi, p.sclk, p.mosi, display_config.clone());
    let spi_bus: Mutex<raw::NoopRawMutex, _> = Mutex::new(RefCell::new(spi));
    let device = SpiDeviceWithConfig::new(&spi_bus, Output::new(p.cs, Level::High), display_config);

    // write mode toggle
    let dc = Output::new(p.dc, Level::Low);

    // backlight toggle
    let bl_en = Output::new(p.bl_en, Level::High);

    // MIPIDSI wrapper
    let mut display = ST7789::new(
        Driver {
            spi: device,
            dcx: dc,
        },
        None::<Output<AnyPin>>,
        Some(bl_en),
        240,
        135,
    );

    display.init(&mut Delay).unwrap();

    display.set_orientation(Orientation::Landscape).unwrap();

    loop { 
        display.clear(Rgb565::BLUE).unwrap();
        Timer::after_secs(1).await;

        display.clear(Rgb565::BLACK).unwrap();
        Timer::after_secs(1).await;
    }
}

struct Driver<SPI: SpiDevice, DCX: OutputPin> {
    spi: SPI,
    dcx: DCX,
}

impl<SPI: SpiDevice, DC: OutputPin> WriteOnlyDataCommand for Driver<SPI, DC> {
    fn send_commands(
        &mut self,
        cmd: display_interface::DataFormat<'_>,
    ) -> Result<(), display_interface::DisplayError> {
        self.dcx.set_low().map_err(|_| DisplayError::DCError)?;

        send_bytes(&mut self.spi, cmd).map_err(|_| DisplayError::BusWriteError)?;
        Ok(())
    }

    fn send_data(
        &mut self,
        buf: display_interface::DataFormat<'_>,
    ) -> Result<(), display_interface::DisplayError> {
        self.dcx.set_high().map_err(|_| DisplayError::DCError)?;

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
