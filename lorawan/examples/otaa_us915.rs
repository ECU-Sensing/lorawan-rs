#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;
use stm32f4xx_hal as hal;

use hal::{
    gpio::{
        gpioa::{PA5, PA6, PA7},
        gpiob::PB6,
        gpioc::{PC7, PC8, PC9},
        Alternate, Input, Output, PushPull,
    },
    prelude::*,
    spi::Spi1,
};

use lorawan::{
    config::device::DeviceConfig,
    device::LoRaWANDevice,
    class::OperatingMode,
    lorawan::region::US915,
    radio::sx127x::SX127x,
};

// Type aliases for SPI and GPIO configurations
type SpiPins = (
    PA5<Alternate<5>>,  // SCK
    PA6<Alternate<5>>,  // MISO
    PA7<Alternate<5>>,  // MOSI
);

type Spi = Spi1<
    (
        PA5<Alternate<5>>,
        PA6<Alternate<5>>,
        PA7<Alternate<5>>,
    )
>;

type RadioPins = (
    PB6<Output<PushPull>>,   // CS
    PC7<Output<PushPull>>,   // RESET
    PC8<Input>,              // DIO0
    PC9<Input>,              // DIO1
);

#[entry]
fn main() -> ! {
    // Get peripherals
    let dp = hal::stm32::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Set up clocks
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze();

    // Set up delay
    let mut delay = cp.SYST.delay(&clocks);

    // Configure GPIO pins
    let gpioa = dp.GPIOA.split();
    let gpiob = dp.GPIOB.split();
    let gpioc = dp.GPIOC.split();

    // Configure SPI pins
    let sck = gpioa.pa5.into_alternate();
    let miso = gpioa.pa6.into_alternate();
    let mosi = gpioa.pa7.into_alternate();

    // Configure radio control pins
    let cs = gpiob.pb6.into_push_pull_output();
    let reset = gpioc.pc7.into_push_pull_output();
    let dio0 = gpioc.pc8.into_floating_input();
    let dio1 = gpioc.pc9.into_floating_input();

    // Initialize SPI
    let spi = Spi::new(
        dp.SPI1,
        (sck, miso, mosi),
        hal::spi::Mode {
            polarity: hal::spi::Polarity::IdleLow,
            phase: hal::spi::Phase::CaptureOnFirstTransition,
        },
        1.MHz(),
        &clocks,
    );

    // Initialize radio
    let radio = SX127x::new(spi, (cs, reset, dio0, dio1));

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
                delay.delay_ms(5000_u32);
            }
        }
    }

    // Main loop - send periodic uplinks
    let mut counter = 0u32;
    loop {
        // Prepare uplink data
        let data = counter.to_be_bytes();
        
        // Send unconfirmed uplink on port 1
        if let Ok(_) = device.send_uplink(1, &data, false) {
            // Process device (handle receive windows)
            device.process().ok();
        }

        // Wait 60 seconds before next transmission
        delay.delay_ms(60_000_u32);
        counter = counter.wrapping_add(1);
    }
} 