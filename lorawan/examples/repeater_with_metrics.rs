//! LoRaWAN Repeater with Metrics Example for Adafruit Feather M0 with RFM95
//! 
//! This example implements a LoRaWAN repeater that:
//! - Functions as a standard packet repeater using lazy frequency hopping
//! - Acts as a LoRaWAN end node to report metrics
//! - Reports number of forwarded packets periodically
//! - Supports remote reboot command
//! 
//! Lazy frequency hopping means the repeater retransmits packets on
//! the same frequency they were received on, letting the end devices
//! control the frequency hopping pattern.
//! 
//! # Hardware Setup
//! Same as repeater.rs, designed for Adafruit Feather M0 with RFM95 (Product #3178)
//! 
//! # LED Status Patterns
//! Combines patterns from both repeater and end-node:
//! 1. Radio Init & Join
//!    - Success: Both LEDs blink twice
//!    - Failure: Red LED rapid blink
//! 2. Operation Status
//!    - Listening: Red LED breathing pattern
//!    - Packet Received: Both LEDs on
//!    - Forwarding: Blue LED on
//!    - Forward Success: Red LED double blink
//!    - Metrics Transmission: Both LEDs alternate
//!    - Error: Both LEDs triple blink

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;
use atsamd21_hal as hal;

use core::sync::atomic::{AtomicU32, Ordering};
use heapless::Vec;

use hal::{
    clock::GenericClockController,
    delay::Delay,
    gpio::{
        Pa8, Pa9, Pa10, Pa11, Pa12, Pa13, Pa14, Pa17,
        Output, Input, Floating, PushPull,
    },
    prelude::*,
    sercom::{I2CMaster4, SPIMaster0},
    time::Hertz,
};

use lorawan::{
    config::device::DeviceConfig,
    device::{LoRaWANDevice, DeviceState},
    class::OperatingMode,
    lorawan::{
        region::US915,
        mac::{MacLayer, MacError},
        commands::{CommandHandler, DownlinkCommand},
    },
    radio::sx127x::SX127x,
};

// Metrics reporting interval (60 seconds)
const METRICS_INTERVAL_MS: u32 = 60_000;

// Packet counter (static to share between main loop and command handler)
static PACKET_COUNTER: AtomicU32 = AtomicU32::new(0);

// Type definitions for SPI and GPIO
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

// LED type aliases
type RedLed = Pa17<Output<PushPull>>;    // Built-in red LED on pin 13
type BlueLed = Pa10<Output<PushPull>>;   // Built-in blue LED on pin 32

/// LED status patterns
struct StatusLeds {
    red: RedLed,
    blue: BlueLed,
}

impl StatusLeds {
    fn new(red: RedLed, blue: BlueLed) -> Self {
        Self { red, blue }
    }

    // ... existing LED patterns ...

    /// Indicate metrics transmission
    fn indicate_metrics_tx(&mut self, delay: &mut Delay) {
        // Alternate LEDs three times
        for _ in 0..3 {
            self.red.set_high().ok();
            self.blue.set_low().ok();
            delay.delay_ms(100u32);
            self.red.set_low().ok();
            self.blue.set_high().ok();
            delay.delay_ms(100u32);
        }
        self.red.set_low().ok();
        self.blue.set_low().ok();
    }
}

/// Metrics data structure
#[derive(Default)]
struct RepeaterMetrics {
    packets_forwarded: u32,
    last_rssi: i16,
    last_snr: i8,
}

impl RepeaterMetrics {
    fn to_bytes(&self) -> Vec<u8, 32> {
        let mut buffer = Vec::new();
        // Packets forwarded (4 bytes)
        buffer.extend_from_slice(&self.packets_forwarded.to_be_bytes()).unwrap();
        // Last RSSI (2 bytes)
        buffer.extend_from_slice(&self.last_rssi.to_be_bytes()).unwrap();
        // Last SNR (1 byte)
        buffer.push(self.last_snr as u8).unwrap();
        buffer
    }
}

#[entry]
fn main() -> ! {
    // Initialize clocks, delay, and pins (same as repeater.rs)
    let mut peripherals = hal::pac::Peripherals::take().unwrap();
    let core = hal::pac::CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );
    let mut delay = Delay::new(core.SYST, &mut clocks);
    let pins = hal::Pins::new(peripherals.PORT);

    // Configure pins and SPI (same as repeater.rs)
    let miso = pins.mi.into_pad(&mut peripherals.PORT);
    let mosi = pins.mo.into_pad(&mut peripherals.PORT);
    let sck = pins.sck.into_pad(&mut peripherals.PORT);
    let cs = pins.d8.into_push_pull_output();
    let reset = pins.d4.into_push_pull_output();
    let dio0 = pins.d3.into_floating_input();
    let dio1 = pins.d6.into_floating_input();

    let spi = SPIMaster0::new(
        &clocks.sercom0_core(&mut peripherals.GCLK).unwrap(),
        Hertz(8_000_000),
        hal::hal::spi::Mode {
            phase: hal::hal::spi::Phase::CaptureOnFirstTransition,
            polarity: hal::hal::spi::Polarity::IdleLow,
        },
        peripherals.SERCOM0,
        &mut peripherals.PM,
        (miso, mosi, sck),
    );

    // Configure LEDs
    let red_led = pins.d13.into_push_pull_output();
    let blue_led = pins.d32.into_push_pull_output();
    let mut status_leds = StatusLeds::new(red_led, blue_led);

    // Initialize radio
    let radio = match SX127x::new(spi, cs, reset, dio0, dio1, &mut delay) {
        Ok(radio) => {
            status_leds.indicate_init_success(&mut delay);
            radio
        }
        Err(_) => {
            status_leds.indicate_init_failure(&mut delay);
            loop {
                status_leds.red.set_high().ok();
                delay.delay_ms(100u32);
                status_leds.red.set_low().ok();
                delay.delay_ms(900u32);
            }
        }
    };

    // Create LoRaWAN device configuration for metrics reporting
    let config = DeviceConfig::new_otaa(
        // DevEUI in LSB format (least significant byte first)
        // Example: If your DevEUI is "0123456789ABCDEF", enter it as:
        // [0xEF, 0xCD, 0xAB, 0x89, 0x67, 0x45, 0x23, 0x01]
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],

        // AppEUI/JoinEUI in LSB format (least significant byte first)
        // Example: If your AppEUI is "0123456789ABCDEF", enter it as:
        // [0xEF, 0xCD, 0xAB, 0x89, 0x67, 0x45, 0x23, 0x01]
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],

        // AppKey in MSB format (most significant byte first)
        // Example: If your AppKey is "0123456789ABCDEF0123456789ABCDEF", enter it as:
        // [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF,
        //  0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF]
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    );

    // Create region configuration and LoRaWAN device
    let region = US915::new();
    let mut lorawan_device = match LoRaWANDevice::new(
        radio,
        config,
        region,
        OperatingMode::ClassA,
    ) {
        Ok(device) => device,
        Err(_) => {
            status_leds.indicate_error(&mut delay);
            loop {
                delay.delay_ms(1000u32);
            }
        }
    };

    // Configure for TTN US915
    if let Err(_) = lorawan_device.mac().configure_for_ttn() {
        status_leds.indicate_error(&mut delay);
        loop {
            delay.delay_ms(1000u32);
        }
    }

    // Join network using OTAA
    loop {
        status_leds.indicate_joining(&mut delay);
        match lorawan_device.join_otaa() {
            Ok(_) => {
                status_leds.indicate_join_success(&mut delay);
                break;
            }
            Err(_) => {
                status_leds.indicate_join_failure(&mut delay);
                delay.delay_ms(5000u32);
            }
        }
    }

    // Initialize metrics
    let mut metrics = RepeaterMetrics::default();
    let mut last_metrics_time = 0u32;

    // Main loop
    let mut rx_buffer = [0u8; 255];
    loop {
        // Show listening status
        status_leds.indicate_listening();

        // Check if it's time to send metrics
        if last_metrics_time >= METRICS_INTERVAL_MS {
            // Update metrics from atomic counter
            metrics.packets_forwarded = PACKET_COUNTER.load(Ordering::Relaxed);
            
            // Send metrics
            status_leds.indicate_metrics_tx(&mut delay);
            if let Ok(_) = lorawan_device.send_uplink(2, &metrics.to_bytes(), true) {
                // Process any downlink commands
                if let Ok(Some(command)) = lorawan_device.process() {
                    match command {
                        DownlinkCommand::Reboot => {
                            // Trigger system reset
                            cortex_m::peripheral::SCB::sys_reset();
                        }
                        _ => {} // Ignore other commands
                    }
                }
            }
            
            last_metrics_time = 0;
        }

        // Repeater functionality
        match lorawan_device.radio().receive(&mut rx_buffer) {
            Ok(len) if len > 0 => {
                status_leds.indicate_packet_received();
                
                if let Some(valid) = validate_lorawan_packet(&rx_buffer[..len]) {
                    if valid {
                        // Get current frequency (the one we received on)
                        let current_freq = match lorawan_device.radio().get_frequency() {
                            Ok(freq) => freq,
                            Err(_) => {
                                status_leds.indicate_error(&mut delay);
                                continue;
                            }
                        };

                        status_leds.indicate_packet_forwarding();
                        
                        // Forward packet on same frequency
                        match lorawan_device.radio().transmit(&rx_buffer[..len]) {
                            Ok(_) => {
                                status_leds.indicate_packet_forwarded();
                                // Increment packet counter atomically
                                PACKET_COUNTER.fetch_add(1, Ordering::Relaxed);
                            }
                            Err(_) => {
                                status_leds.indicate_error(&mut delay);
                            }
                        }
                    }
                }
            }
            Err(_) => {
                status_leds.indicate_error(&mut delay);
            }
            _ => {}
        }

        // Update timing
        last_metrics_time = last_metrics_time.saturating_add(10);
        delay.delay_ms(10u32);
    }
}

// Validate LoRaWAN packet - only check if it's a valid LoRaWAN message type
fn validate_lorawan_packet(data: &[u8]) -> Option<bool> {
    if data.len() < 8 {
        return None;  // Packet too short to be valid LoRaWAN
    }

    let mtype = data[0] & 0xE0;
    // Accept uplink data (0x40) and downlink data (0x80) messages
    Some(mtype == 0x40 || mtype == 0x80)
} 