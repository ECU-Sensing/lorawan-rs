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
    lorawan::{
        region::US915,
        mac::{MacLayer, MacError},
    },
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

// Repeater configuration - adjusted for US915 band plan
const SOURCE_FREQUENCY: u32 = 903_900_000; // Source frequency (Hz) - US915 uplink
const DEST_FREQUENCY: u32 = 923_300_000;   // Destination frequency (Hz) - US915 downlink
const ALLOWED_DEVADDR: [u8; 4] = [0x01, 0x02, 0x03, 0x04]; // Optional: filter by DevAddr

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

    // Initialize radio with RFM95-specific settings
    let mut radio = SX127x::new(
        spi,
        cs,
        reset,
        dio0,
        dio1,
        &mut delay,
    ).unwrap();

    // Configure radio for initial receive with RFM95-optimized settings
    radio.init().unwrap();
    radio.set_frequency(SOURCE_FREQUENCY).unwrap();
    radio.set_rx_config(
        lorawan::radio::traits::RxConfig {
            frequency: SOURCE_FREQUENCY,
            modulation: lorawan::radio::traits::ModulationParams {
                spreading_factor: 7,
                bandwidth: 125_000,
                coding_rate: 5,
            },
            timeout_ms: 0, // Continuous receive
        }
    ).unwrap();

    // Set PA config for RFM95 (high power settings)
    radio.set_tx_power(20).unwrap(); // Set to 20dBm for maximum power

    // Main loop - receive and forward packets
    let mut rx_buffer = [0u8; 255];
    let mut packet_count = 0u32;

    loop {
        // Receive packet
        match radio.receive(&mut rx_buffer) {
            Ok(len) if len > 0 => {
                // Validate packet (optional)
                if let Some(valid) = validate_lorawan_packet(&rx_buffer[..len]) {
                    // Switch to transmit frequency
                    radio.set_frequency(DEST_FREQUENCY).unwrap();
                    
                    // Forward packet
                    radio.transmit(&rx_buffer[..len]).unwrap();
                    
                    // Switch back to receive frequency
                    radio.set_frequency(SOURCE_FREQUENCY).unwrap();
                    
                    packet_count = packet_count.wrapping_add(1);
                }
            }
            _ => {
                // No packet received or error, continue listening
            }
        }

        // Optional: small delay to prevent tight loop
        delay.delay_ms(10u32);
    }
}

/// Validate a LoRaWAN packet
/// Returns Some(true) if packet should be forwarded, Some(false) if not, None if invalid
fn validate_lorawan_packet(data: &[u8]) -> Option<bool> {
    if data.len() < 8 {
        return None; // Too short to be valid LoRaWAN
    }

    // Check MHDR (first byte)
    let mtype = data[0] & 0xE0;
    if mtype != 0x40 && mtype != 0x80 { // Only forward uplink data messages
        return Some(false);
    }

    // Extract DevAddr (bytes 1-4)
    let dev_addr = &data[1..5];
    
    // Optional: Check if this is a DevAddr we want to forward
    if dev_addr == ALLOWED_DEVADDR {
        Some(true)
    } else {
        // Forward all packets (remove this check for more selective forwarding)
        Some(true)
    }
}

/// Helper function to check if a packet is a duplicate
/// (could be implemented to prevent forwarding the same packet multiple times)
fn is_duplicate(packet: &[u8]) -> bool {
    // Implement duplicate detection logic here if needed
    // For example, keep a rolling history of frame counters per DevAddr
    false
} 