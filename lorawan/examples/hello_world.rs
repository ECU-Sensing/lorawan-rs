#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;
use atsamd21_hal as hal;

use hal::{
    clock::GenericClockController,
    delay::Delay,
    gpio::{
        Pa8, Pa9, Pa10, Pa11, Pa12, Pa13, Pa14,
        Output, Input, Floating, PushPull,
    },
    prelude::*,
    sercom::{I2CMaster4, SPIMaster0},
    time::Hertz,
};

use lorawan::{
    config::device::DeviceConfig,
    device::LoRaWANDevice,
    class::OperatingMode,
    lorawan::region::US915,
    radio::sx127x::SX127x,
};

// Type aliases for SPI and GPIO configurations
type Spi = SPIMaster0<
    hal::sercom::Sercom0Pad2<Pa10<hal::gpio::PfD>>,  // MISO - MI pin
    hal::sercom::Sercom0Pad3<Pa11<hal::gpio::PfD>>,  // MOSI - MO pin
    hal::sercom::Sercom0Pad1<Pa9<hal::gpio::PfD>>,   // SCK - SCK pin
>;

type RadioPins = (
    Pa8<Output<PushPull>>,    // CS - D8
    Pa14<Output<PushPull>>,   // RESET - D4
    Pa9<Input<Floating>>,     // DIO0 - D3
    Pa10<Input<Floating>>,    // DIO1 - D6
);

#[entry]
fn main() -> ! {
    // Get peripherals
    let mut peripherals = hal::pac::Peripherals::take().unwrap();
    let core = hal::pac::CorePeripherals::take().unwrap();

    // Set up clocks
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );

    // Set up delay provider
    let mut delay = Delay::new(core.SYST, &mut clocks);

    // Configure pins
    let pins = hal::Pins::new(peripherals.PORT);

    // Configure SPI pins for RFM95
    let miso = pins.mi.into_pad(&mut peripherals.PORT);
    let mosi = pins.mo.into_pad(&mut peripherals.PORT);
    let sck = pins.sck.into_pad(&mut peripherals.PORT);

    // Configure radio control pins for RFM95
    let cs = pins.d8.into_push_pull_output();
    let reset = pins.d4.into_push_pull_output();
    let dio0 = pins.d3.into_floating_input();
    let dio1 = pins.d6.into_floating_input();

    // Initialize SPI with correct settings for RFM95
    let spi = SPIMaster0::new(
        &clocks.sercom0_core(&mut peripherals.GCLK).unwrap(),
        Hertz(8_000_000), // RFM95 supports up to 10MHz, using 8MHz for reliability
        hal::hal::spi::Mode {
            phase: hal::hal::spi::Phase::CaptureOnFirstTransition,
            polarity: hal::hal::spi::Polarity::IdleLow,
        },
        peripherals.SERCOM0,
        &mut peripherals.PM,
        (miso, mosi, sck),
    );

    // Initialize radio
    let radio = SX127x::new(
        spi,
        cs,
        reset,
        dio0,
        dio1,
        &mut delay,
    ).unwrap();

    // Create device configuration
    let config = DeviceConfig::new_otaa(
        [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08], // DevEUI
        [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01], // AppEUI
        [0x2B, 0x7E, 0x15, 0x16, 0x28, 0xAE, 0xD2, 0xA6,
         0xAB, 0xF7, 0x15, 0x88, 0x09, 0xCF, 0x4F, 0x3C], // AppKey
    );

    // Create region configuration
    let region = US915::new();

    // Create LoRaWAN device
    let mut device = LoRaWANDevice::new(
        radio,
        config,
        region,
        OperatingMode::ClassA,
    ).unwrap();

    // Join network using OTAA
    loop {
        match device.join_otaa() {
            Ok(_) => {
                // Successfully joined
                break;
            }
            Err(_) => {
                // Wait and retry
                delay.delay_ms(5000u32);
            }
        }
    }

    // Main loop - send "Hello World" every 30 seconds
    let message = b"Hello World!";
    loop {
        // Send unconfirmed uplink on port 1
        if let Ok(_) = device.send_uplink(1, message, false) {
            // Process device (handle receive windows)
            device.process().ok();
        }

        // Wait 30 seconds before next transmission
        delay.delay_ms(30_000u32);
    }
} 