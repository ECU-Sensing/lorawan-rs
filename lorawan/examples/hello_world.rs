//! Hello World Example for Adafruit Feather M0 with RFM95
//!
//! This example demonstrates basic LoRaWAN connectivity using OTAA with TTN.
//! It properly implements frequency hopping and uses TTN's US915 channel plan.
//!
//! # Hardware Setup
//! - Adafruit Feather M0 with RFM95 LoRa Radio (900MHz) - Product #3178
//! - Antenna (915MHz tuned)
//!
//! # LED Status Patterns
//! - Radio Init: Blue LED blinks twice
//! - Join Request: Both LEDs alternate
//! - Join Success: Both LEDs solid for 1 second
//! - Join Failed: Red LED rapid blink
//! - Transmitting: Blue LED on
//! - Transmission Success: Blue LED blinks once
//! - Error: Both LEDs blink three times
//!
//! # TTN Configuration
//! 1. Create a new application in TTN Console
//! 2. Register a new device with:
//!    - Frequency plan: US_915_928 (or your region)
//!    - LoRaWAN version: LoRaWAN Specification 1.0.3
//!    - Regional Parameters version: RP001 Regional Parameters 1.0.3 revision A
//! 3. Copy the device credentials below:
//!    - Device EUI
//!    - Application EUI (Join EUI)
//!    - App Key

#![no_std]
#![no_main]

use atsamd21_hal as hal;
use cortex_m_rt::entry;
use panic_halt as _;

use hal::{
    clock::GenericClockController,
    delay::Delay,
    gpio::{Floating, Input, Output, Pa10, Pa11, Pa12, Pa13, Pa14, Pa17, Pa8, Pa9, PushPull},
    prelude::*,
    sercom::{I2CMaster4, SPIMaster0},
    time::U32Ext,
};

use lorawan::{
    config::device::{AESKey, DeviceConfig},
    device::Device,
    lorawan::{
        class::OperatingMode,
        region::US915,
    },
    radio::sx127x::SX127x,
};

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
    fn indicate_init(&mut self, delay: &mut Delay) {
        // Blink blue LED twice
        for _ in 0..2 {
            self.blue.set_high().ok();
            delay.delay_ms(100u32);
            self.blue.set_low().ok();
            delay.delay_ms(100u32);
        }
    }

    /// Indicate join attempt
    fn indicate_joining(&mut self, delay: &mut Delay) {
        // Alternate LEDs
        self.red.set_high().ok();
        self.blue.set_low().ok();
        delay.delay_ms(100u32);
        self.red.set_low().ok();
        self.blue.set_high().ok();
        delay.delay_ms(100u32);
    }

    /// Indicate successful join
    fn indicate_join_success(&mut self, delay: &mut Delay) {
        // Both LEDs solid for 1 second
        self.red.set_high().ok();
        self.blue.set_high().ok();
        delay.delay_ms(1000u32);
        self.red.set_low().ok();
        self.blue.set_low().ok();
    }

    /// Indicate join failure
    fn indicate_join_failure(&mut self, delay: &mut Delay) {
        // Rapid red LED blink
        for _ in 0..5 {
            self.red.set_high().ok();
            delay.delay_ms(50u32);
            self.red.set_low().ok();
            delay.delay_ms(50u32);
        }
    }

    /// Indicate transmission
    fn indicate_transmitting(&mut self) {
        self.blue.set_high().ok();
    }

    /// Indicate transmission success
    fn indicate_tx_success(&mut self, delay: &mut Delay) {
        self.blue.set_low().ok();
        delay.delay_ms(100u32);
        self.blue.set_high().ok();
        delay.delay_ms(100u32);
        self.blue.set_low().ok();
    }

    /// Indicate error
    fn indicate_error(&mut self, delay: &mut Delay) {
        // Triple blink both LEDs
        for _ in 0..3 {
            self.red.set_high().ok();
            self.blue.set_high().ok();
            delay.delay_ms(100u32);
            self.red.set_low().ok();
            self.blue.set_low().ok();
            delay.delay_ms(100u32);
        }
    }
}

#[entry]
fn main() -> ! {
    // Initialize peripherals
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

    // Configure SPI pins
    let miso = pins.mi.into_pad(&mut peripherals.PORT);
    let mosi = pins.mo.into_pad(&mut peripherals.PORT);
    let sck = pins.sck.into_pad(&mut peripherals.PORT);

    // Configure radio control pins
    let cs = pins.d8.into_push_pull_output();
    let reset = pins.d4.into_push_pull_output();
    let dio0 = pins.d3.into_floating_input();
    let dio1 = pins.d6.into_floating_input();

    // Initialize SPI
    let spi = SPIMaster0::new(
        &clocks.sercom0_core(&mut peripherals.GCLK).unwrap(),
        8.mhz(),
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
            status_leds.indicate_init(&mut delay);
            radio
        }
        Err(_) => {
            status_leds.indicate_error(&mut delay);
            loop {
                delay.delay_ms(1000u32);
            }
        }
    };

    // Create LoRaWAN device configuration
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
        // Example: If your AppKey is "0123456789ABCDEF0123456789ABCDEF", enter it as is:
        AESKey::new([
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]),
    );

    // Create region configuration
    let mut region = US915::new();
    region.set_sub_band(2); // Configure for TTN US915 sub-band 2

    // Create LoRaWAN device
    let mut device = match Device::new(radio, config, region, OperatingMode::ClassA) {
        Ok(device) => device,
        Err(_) => {
            status_leds.indicate_error(&mut delay);
            loop {
                delay.delay_ms(1000u32);
            }
        }
    };

    // Join the network
    status_leds.indicate_joining(&mut delay);
    match device.join_otaa() {
        Ok(_) => status_leds.indicate_join_success(&mut delay),
        Err(_) => {
            status_leds.indicate_join_failure(&mut delay);
            loop {
                delay.delay_ms(1000u32);
            }
        }
    }

    // Main loop - send "Hello, LoRaWAN!" every 60 seconds
    let mut counter = 0u32;
    let mut message = [0u8; 32];
    loop {
        status_leds.indicate_transmitting();
        
        // Format message with counter
        let msg = b"Hello, LoRaWAN! Count: ";
        message[..msg.len()].copy_from_slice(msg);
        let count_str = counter.to_string();
        message[msg.len()..msg.len() + count_str.len()].copy_from_slice(count_str.as_bytes());
        
        match device.send_unconfirmed(1, &message[..msg.len() + count_str.len()]) {
            Ok(_) => status_leds.indicate_tx_success(&mut delay),
            Err(_) => status_leds.indicate_error(&mut delay),
        }

        counter = counter.wrapping_add(1);
        delay.delay_ms(60_000u32);
    }
}
