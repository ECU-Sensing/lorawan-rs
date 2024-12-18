#![no_std]

use lorawan::{
    class::OperatingMode,
    config::device::{AESKey, DeviceConfig},
    device::LoRaWANDevice,
    lorawan::region::US915,
    radio::sx127x::SX127x,
};

// Example DevEUI, AppEUI and AppKey - replace with your own from TTN console
const DEVEUI: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]; // LSB
const APPEUI: [u8; 8] = [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]; // LSB
const APPKEY: [u8; 16] = [
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
]; // MSB

#[cortex_m_rt::entry]
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
    let radio = SX127x::new(spi, cs, reset, dio0, dio1, dio2).expect("Failed to initialize radio");

    // Create device configuration
    let config = DeviceConfig::new_otaa(DEVEUI, APPEUI, AESKey::new(APPKEY));

    // Initialize US915 region with proper sub-band
    let mut region = US915::new();
    region.set_sub_band(2); // TTN US915 uses sub-band 2

    // Initialize LoRaWAN device
    let mut device = LoRaWANDevice::new(radio, config, region, OperatingMode::ClassA)
        .expect("Failed to initialize device");

    // Join network with OTAA
    status_led.set_high().ok();
    if let Err(_) = device.join_otaa(DEVEUI, APPEUI, AESKey::new(APPKEY)) {
        // Rapid blink on error
        loop {
            status_led.toggle().ok();
            hal::delay::Delay::new().delay_ms(100u32);
        }
    }
    status_led.set_low().ok();

    // Main loop - send data with frequency hopping
    let mut delay = hal::delay::Delay::new();
    let mut counter = 0u32;
    loop {
        // Prepare sensor data (example)
        let mut data = [0u8; 4];
        data[0..4].copy_from_slice(&counter.to_le_bytes());

        // Send data with frequency hopping (handled by region)
        status_led.set_high().ok();
        if let Err(_) = device.send_data(1, &data, true) {
            // Use confirmed messages
            loop {
                status_led.toggle().ok();
                delay.delay_ms(500u32);
            }
        }
        status_led.set_low().ok();

        // Process any downlink messages
        for _ in 0..2 {
            if let Ok(size) = device.receive(&mut [0u8; 256]) {
                if size > 0 {
                    // Received downlink - blink twice
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

        counter = counter.wrapping_add(1);
        delay.delay_ms(60_000u32); // Send every minute
    }
}
