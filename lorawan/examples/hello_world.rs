//! Basic LoRaWAN "Hello World" Example
//!
//! This example demonstrates the fundamental usage of the LoRaWAN stack:
//! - Hardware initialization for Adafruit Feather M0 with RFM95
//! - OTAA device activation with TTN
//! - Basic uplink messaging with a counter
//! - Simple downlink processing
//! - LED status indication for debugging
//!   * Fast blink: Radio initialization error
//!   * Double blink: Device initialization error
//!   * Triple blink: Join error
//!   * Solid LED: Transmitting
//!   * Single blink: Successful transmission
//!
//! The device sends "Hello, LoRaWAN! #<counter>" every 30 seconds using
//! unconfirmed uplinks on port 1. Error handling demonstrates proper
//! recovery patterns for embedded systems.

#![no_std]
#![no_main]

use lorawan::{
    class::OperatingMode,
    config::device::{AESKey, DeviceConfig},
    device::LoRaWANDevice,
    lorawan::region::US915,
    radio::sx127x::SX127x,
};

use cortex_m_rt::entry;
use panic_halt as _;

// Example DevEUI, AppEUI and AppKey - replace with your own from TTN console
const DEVEUI: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]; // LSB
const APPEUI: [u8; 8] = [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]; // LSB
const APPKEY: [u8; 16] = [
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
]; // MSB

#[entry]
fn main() -> ! {
    // Initialize LED pins for status indication
    let peripherals = hal::Peripherals::take().unwrap();
    let pins = hal::Pins::new(peripherals.PORT);
    let mut red_led = pins.d13.into_push_pull_output();

    // Initialize SPI for radio
    let spi = hal::spi_master(
        &mut peripherals.PM,
        peripherals.SERCOM4,
        &mut peripherals.GCLK,
        pins.sck,
        pins.mosi,
        pins.miso,
        hal::spi::Polarity::IdleLow,
        hal::spi::Phase::CaptureOnFirstTransition,
        1.mhz(),
    );

    // Initialize radio
    let cs = pins.rfm_cs.into_push_pull_output();
    let reset = pins.rfm_rst.into_push_pull_output();
    let dio0 = pins.d3.into_floating_input();
    let dio1 = pins.d6.into_floating_input();
    let dio2 = pins.d9.into_floating_input();
    let radio = match SX127x::new(spi, cs, reset, dio0, dio1, dio2) {
        Ok(r) => r,
        Err(_) => {
            // Rapid blink on radio init error
            loop {
                red_led.toggle().ok();
                hal::delay::Delay::new().delay_ms(100u32);
            }
        }
    };

    // Create device configuration
    let config = DeviceConfig::new_otaa(DEVEUI, APPEUI, AESKey::new(APPKEY));

    // Initialize LoRaWAN device
    let mut device = match LoRaWANDevice::new(radio, config, US915::new(), OperatingMode::ClassA) {
        Ok(d) => d,
        Err(_) => {
            // Double blink on device init error
            loop {
                for _ in 0..2 {
                    red_led.set_high().ok();
                    hal::delay::Delay::new().delay_ms(100u32);
                    red_led.set_low().ok();
                    hal::delay::Delay::new().delay_ms(100u32);
                }
                hal::delay::Delay::new().delay_ms(500u32);
            }
        }
    };

    // Join network
    red_led.set_high().ok();
    if let Err(_) = device.join_otaa(DEVEUI, APPEUI, AESKey::new(APPKEY)) {
        // Triple blink on join error
        loop {
            for _ in 0..3 {
                red_led.toggle().ok();
                hal::delay::Delay::new().delay_ms(100u32);
            }
            hal::delay::Delay::new().delay_ms(500u32);
        }
    }
    red_led.set_low().ok();

    // Main loop - send "Hello, LoRaWAN!" every 30 seconds
    let mut delay = hal::delay::Delay::new();
    let mut counter = 0u32;
    loop {
        // Format message with counter
        let mut message = [0u8; 32];
        let msg = b"Hello, LoRaWAN! #";
        message[..msg.len()].copy_from_slice(msg);
        let count_str = counter.to_string();
        message[msg.len()..msg.len() + count_str.len()].copy_from_slice(count_str.as_bytes());

        // Send unconfirmed uplink on port 1
        red_led.set_high().ok();
        if let Err(_) = device.send_data(1, &message[..msg.len() + count_str.len()], false) {
            // Slow blink on send error
            loop {
                red_led.toggle().ok();
                delay.delay_ms(500u32);
            }
        }
        red_led.set_low().ok();

        // Process any downlink
        device.process().ok();

        // Increment counter and wait
        counter = counter.wrapping_add(1);
        delay.delay_ms(30_000u32);
    }
}
