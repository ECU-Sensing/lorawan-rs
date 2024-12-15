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

use atsamd21_hal as hal;
use cortex_m_rt::entry;
use panic_halt as _;

use core::sync::atomic::{AtomicU32, Ordering};
use heapless::Vec;

use hal::{
    clock::GenericClockController,
    delay::Delay,
    gpio::{Floating, Input, Output, Pa10, Pa11, Pa12, Pa13, Pa14, Pa17, Pa8, Pa9, PushPull},
    prelude::*,
    sercom::{I2CMaster4, SPIMaster0},
    time::Hertz,
};

use lorawan::{
    config::device::{AESKey, DeviceConfig, SessionState},
    lorawan::{
        mac::MacLayer,
        region::{Region, US915},
    },
    radio::{
        sx127x::SX127x,
        traits::{Radio, RxConfig},
    },
};

// Metrics reporting interval (60 seconds)
const METRICS_INTERVAL_MS: u32 = 60_000;

// Packet counter (static to share between main loop and command handler)
static PACKET_COUNTER: AtomicU32 = AtomicU32::new(0);

// Type definitions for SPI and GPIO
type Spi = SPIMaster0<
    hal::sercom::Sercom0Pad2<Pa10<hal::gpio::PfD>>, // MISO - MI pin
    hal::sercom::Sercom0Pad3<Pa11<hal::gpio::PfD>>, // MOSI - MO pin
    hal::sercom::Sercom0Pad1<Pa9<hal::gpio::PfD>>,  // SCK - SCK pin
>;

type RadioPins = (
    Pa8<Output<PushPull>>,  // CS - D8
    Pa14<Output<PushPull>>, // RESET - D4
    Pa9<Input<Floating>>,   // DIO0 - D3
    Pa10<Input<Floating>>,  // DIO1 - D6
);

// LED type aliases
type RedLed = Pa17<Output<PushPull>>; // Built-in red LED on pin 13
type BlueLed = Pa10<Output<PushPull>>; // Built-in blue LED on pin 32

/// LED status patterns
struct StatusLeds {
    red: RedLed,
    blue: BlueLed,
}

impl StatusLeds {
    fn new(red: RedLed, blue: BlueLed) -> Self {
        Self { red, blue }
    }

    /// Indicate radio initialization
    fn indicate_init_success(&mut self, delay: &mut Delay) {
        // Blink both LEDs twice
        for _ in 0..2 {
            self.blue.set_high().ok();
            self.red.set_high().ok();
            delay.delay_ms(100u32);
            self.blue.set_low().ok();
            self.red.set_low().ok();
            delay.delay_ms(100u32);
        }
    }

    /// Indicate radio initialization failure
    fn indicate_init_failure(&mut self, delay: &mut Delay) {
        // Rapid red LED blinks
        for _ in 0..5 {
            self.red.set_high().ok();
            delay.delay_ms(50u32);
            self.red.set_low().ok();
            delay.delay_ms(50u32);
        }
    }

    /// Indicate listening mode
    fn indicate_listening(&mut self) {
        self.blue.set_low().ok();
        // Slow breathing pattern on red LED
        self.red.toggle().ok();
    }

    /// Indicate packet reception
    fn indicate_packet_received(&mut self) {
        self.blue.set_high().ok();
        self.red.set_high().ok();
    }

    /// Indicate packet forwarding
    fn indicate_packet_forwarding(&mut self) {
        self.blue.set_high().ok();
        self.red.set_low().ok();
    }

    /// Indicate packet forwarded successfully
    fn indicate_packet_forwarded(&mut self) {
        self.blue.set_low().ok();
        // Quick double blink on success
        self.red.set_high().ok();
        self.red.set_low().ok();
    }

    /// Indicate error
    fn indicate_error(&mut self, delay: &mut Delay) {
        // Quick triple blink of both LEDs
        for _ in 0..3 {
            self.blue.set_high().ok();
            self.red.set_high().ok();
            delay.delay_ms(50u32);
            self.blue.set_low().ok();
            self.red.set_low().ok();
            delay.delay_ms(50u32);
        }
    }

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
        buffer
            .extend_from_slice(&self.packets_forwarded.to_be_bytes())
            .unwrap();
        // Last RSSI (2 bytes)
        buffer
            .extend_from_slice(&self.last_rssi.to_be_bytes())
            .unwrap();
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

    // Configure pins and SPI
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
    let radio = match SX127x::new(spi, (cs, reset, dio0, dio1)) {
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

    // Create device configuration for metrics reporting
    let config = DeviceConfig::new_otaa(
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // DevEUI
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // AppEUI
        AESKey::new([0x00; 16]), // AppKey
    );

    // Create region configuration
    let region = US915::new();
    
    // Create initial session state
    let session = SessionState::new();

    // Create MAC layer
    let mut mac = MacLayer::new(radio, region, session);

    // Configure for TTN US915
    mac.configure_for_ttn().unwrap();

    // Join network using OTAA for metrics reporting
    loop {
        match mac.join_request(
            config.dev_eui,
            config.app_eui,
            config.app_key.clone(),
        ) {
            Ok(_) => {
                break;
            }
            Err(_) => {
                delay.delay_ms(5000u32);
            }
        }
    }

    // Configure radio for continuous receive
    let base_freq = 903_900_000; // Start of sub-band 2
    mac.get_radio_mut().set_frequency(base_freq).unwrap();
    mac.get_radio_mut()
        .configure_rx(RxConfig {
            frequency: base_freq,
            modulation: lorawan::radio::traits::ModulationParams {
                spreading_factor: 7,
                bandwidth: 125_000,
                coding_rate: 5,
            },
            timeout_ms: 0, // Continuous receive
        })
        .unwrap();

    // Set PA config for RFM95 (high power settings)
    mac.get_radio_mut().set_tx_power(20).unwrap();

    // Initialize metrics
    let mut metrics = RepeaterMetrics::default();
    let mut last_metrics_time = 0u32;

    // Main loop
    let mut rx_buffer = [0u8; 255];
    loop {
        // Show listening status
        status_leds.indicate_listening();

        // Receive packet
        match mac.get_radio_mut().receive(&mut rx_buffer) {
            Ok(len) if len > 0 => {
                status_leds.indicate_packet_received();

                // Update metrics
                metrics.last_rssi = mac.get_radio_mut().get_rssi().unwrap_or(0);
                metrics.last_snr = mac.get_radio_mut().get_snr().unwrap_or(0);

                // Validate packet
                if let Some(valid) = validate_lorawan_packet(&rx_buffer[..len]) {
                    if valid {
                        status_leds.indicate_packet_forwarding();

                        // Forward packet
                        if let Ok(_) = mac.get_radio_mut().transmit(&rx_buffer[..len]) {
                            status_leds.indicate_packet_forwarded();
                            metrics.packets_forwarded = metrics.packets_forwarded.wrapping_add(1);
                            PACKET_COUNTER.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }
            Err(_) => {
                status_leds.indicate_error(&mut delay);
            }
            _ => {} // No packet received
        }

        // Send metrics if interval elapsed
        let current_time = cortex_m::peripheral::SYST::get_current()
            .expect("SYST counter should be available");
        if current_time.wrapping_sub(last_metrics_time) >= METRICS_INTERVAL_MS {
            status_leds.indicate_metrics_tx(&mut delay);

            // Send metrics on port 2
            if let Ok(_) = mac.send_unconfirmed(2, &metrics.to_bytes()) {
                last_metrics_time = current_time;
            }
        }

        // Small delay to prevent tight loop
        delay.delay_ms(10u32);
    }
}

/// Validate a LoRaWAN packet
/// Returns Some(true) if packet should be forwarded, Some(false) if not, None if invalid
fn validate_lorawan_packet(data: &[u8]) -> Option<bool> {
    if data.len() < 8 {
        return None; // Packet too short to be valid LoRaWAN
    }

    let mtype = data[0] & 0xE0;
    // Accept uplink data (0x40) and downlink data (0x80) messages
    Some(mtype == 0x40 || mtype == 0x80)
}
