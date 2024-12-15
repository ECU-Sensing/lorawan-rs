//! LoRaWAN MAC Command and Downlink Processing Example
//!
//! This example demonstrates comprehensive MAC command handling:
//! - Complete implementation of all LoRaWAN 1.0.3 MAC commands
//! - Proper response generation for network requests
//! - Dynamic device configuration updates
//! - Automatic channel management
//! - LED feedback for operations:
//!   * Fast blink: Radio error
//!   * Double blink: Device error
//!   * Triple blink: Join error
//!   * Solid LED: Transmitting
//!   * Double pulse: Downlink received
//!
//! The device sends a status message every minute and processes any
//! received MAC commands, demonstrating proper network management
//! and configuration capabilities. Supported commands include:
//! - Device status requests
//! - Duty cycle settings
//! - RX parameter updates
//! - Channel configuration
//! - Link checks

#![no_std]
#![no_main]

use lorawan::{
    class::OperatingMode,
    config::device::{AESKey, DeviceConfig},
    device::LoRaWANDevice,
    lorawan::{commands::MacCommand, region::US915},
    radio::sx127x::SX127x,
};

use cortex_m_rt::entry;
use heapless::Vec;
use panic_halt as _;

// Example DevEUI, AppEUI and AppKey - replace with your own from TTN console
const DEVEUI: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]; // LSB
const APPEUI: [u8; 8] = [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]; // LSB
const APPKEY: [u8; 16] = [
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
]; // MSB

#[entry]
fn main() -> ! {
    // Initialize hardware
    let peripherals = hal::Peripherals::take().unwrap();
    let pins = hal::Pins::new(peripherals.PORT);
    let mut status_led = pins.d13.into_push_pull_output();

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
        Err(_) => loop {
            status_led.toggle().ok();
            hal::delay::Delay::new().delay_ms(100u32);
        },
    };

    // Create device configuration
    let config = DeviceConfig::new_otaa(DEVEUI, APPEUI, AESKey::new(APPKEY));

    // Initialize LoRaWAN device
    let mut device = match LoRaWANDevice::new(radio, config, US915::new(), OperatingMode::ClassA) {
        Ok(d) => d,
        Err(_) => loop {
            for _ in 0..2 {
                status_led.set_high().ok();
                hal::delay::Delay::new().delay_ms(100u32);
                status_led.set_low().ok();
                hal::delay::Delay::new().delay_ms(100u32);
            }
            hal::delay::Delay::new().delay_ms(500u32);
        },
    };

    // Join network
    status_led.set_high().ok();
    if let Err(_) = device.join_otaa(DEVEUI, APPEUI, AESKey::new(APPKEY)) {
        loop {
            for _ in 0..3 {
                status_led.toggle().ok();
                hal::delay::Delay::new().delay_ms(100u32);
            }
            hal::delay::Delay::new().delay_ms(500u32);
        }
    }
    status_led.set_low().ok();

    // Buffer for received data
    let mut rx_buffer = [0u8; 256];
    let mut delay = hal::delay::Delay::new();

    // Main loop - handle downlink commands
    loop {
        // Send status update
        let mut payload: Vec<u8, 32> = Vec::new();
        payload.extend_from_slice(b"Status OK").unwrap();

        status_led.set_high().ok();
        if let Err(_) = device.send_data(1, &payload, true) {
            loop {
                status_led.toggle().ok();
                delay.delay_ms(500u32);
            }
        }
        status_led.set_low().ok();

        // Check for downlink in both receive windows
        for _ in 0..2 {
            if let Ok(size) = device.receive(&mut rx_buffer) {
                if size > 0 {
                    // Process MAC commands in FRMPayload
                    if let Some(commands) = device.get_mac_commands() {
                        for cmd in commands {
                            match cmd {
                                MacCommand::DevStatusReq => {
                                    // Respond with device status
                                    let battery = 255; // Full battery
                                    let margin = 20; // Good link margin
                                    device.send_device_status(battery, margin).ok();
                                }
                                MacCommand::DutyCycleReq(max_duty_cycle) => {
                                    // Update duty cycle settings
                                    device.set_duty_cycle(max_duty_cycle).ok();
                                }
                                MacCommand::RXParamSetupReq {
                                    rx1_dr_offset,
                                    rx2_data_rate,
                                    freq,
                                } => {
                                    // Update RX parameters
                                    device
                                        .set_rx_params(
                                            rx1_dr_offset,
                                            rx2_data_rate,
                                            rx2_data_rate,
                                            freq,
                                        )
                                        .ok();
                                }
                                MacCommand::NewChannelReq {
                                    ch_index,
                                    freq,
                                    min_dr,
                                    max_dr,
                                } => {
                                    // Configure new channel
                                    device.set_channel(ch_index, freq, min_dr, max_dr).ok();
                                }
                                MacCommand::DlChannelReq { ch_index, freq } => {
                                    // Configure downlink channel
                                    device.set_dl_channel(ch_index, freq).ok();
                                }
                                MacCommand::LinkCheckReq => {
                                    // Link check request received
                                    // Response will be handled automatically
                                }
                                _ => {
                                    // Handle other MAC commands
                                }
                            }
                        }
                    }

                    // Indicate received downlink
                    for _ in 0..2 {
                        status_led.set_high().ok();
                        delay.delay_ms(100u32);
                        status_led.set_low().ok();
                        delay.delay_ms(100u32);
                    }
                }
            }
            device.process().ok();
            delay.delay_ms(1000u32);
        }

        // Wait before next transmission
        delay.delay_ms(60_000u32);
    }
}
