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
    config::device::{AESKey, DeviceConfig, SessionState},
    lorawan::{
        mac::MacLayer,
        region::{Region, US915},
    },
    radio::sx127x::SX127x,
};

// Type aliases for SPI and GPIO configurations
type SpiPins = (
    PA5<Alternate<5>>, // SCK
    PA6<Alternate<5>>, // MISO
    PA7<Alternate<5>>, // MOSI
);

type Spi = Spi1<(PA5<Alternate<5>>, PA6<Alternate<5>>, PA7<Alternate<5>>)>;

type RadioPins = (
    PB6<Output<PushPull>>, // CS
    PC7<Output<PushPull>>, // RESET
    PC8<Input>,            // DIO0
    PC9<Input>,            // DIO1
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
        // DevEUI in LSB format
        [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01],
        // AppEUI/JoinEUI in LSB format
        [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
        // AppKey in MSB format
        AESKey::new([
            0x2B, 0x7E, 0x15, 0x16, 0x28, 0xAE, 0xD2, 0xA6,
            0xAB, 0xF7, 0x15, 0x88, 0x09, 0xCF, 0x4F, 0x3C,
        ]),
    );

    // Create region configuration
    let region = US915::new();
    
    // Create initial session state
    let session = SessionState::new();

    // Create MAC layer
    let mut mac = MacLayer::new(radio, region, session);

    // Configure for TTN US915
    mac.configure_for_ttn().unwrap();

    // Join network using OTAA
    loop {
        match mac.join_request(
            config.dev_eui,
            config.app_eui,
            config.app_key.clone(),
        ) {
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
        if let Ok(_) = mac.send_unconfirmed(1, &data) {
            // Receive in RX1 window
            let mut rx_buffer = [0u8; 256];
            if let Ok(size) = mac.receive(&mut rx_buffer) {
                if size > 0 {
                    // Process received data
                    if let Ok(payload) = mac.decrypt_payload(&rx_buffer[..size]) {
                        // Extract and process MAC commands if any
                        if let Some(commands) = mac.extract_mac_commands(&payload) {
                            for cmd in commands {
                                mac.process_mac_command(cmd).ok();
                            }
                        }
                    }
                }
            }
        }

        // Wait 60 seconds before next transmission
        delay.delay_ms(60_000_u32);
        counter = counter.wrapping_add(1);
    }
}
