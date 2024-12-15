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
    timer::Timer,
};

use lorawan::{
    class::OperatingMode, config::device::DeviceConfig, device::LoRaWANDevice,
    lorawan::region::US915, radio::sx127x::SX127x,
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

// Sensor data structure
#[derive(Default)]
struct SensorData {
    temperature: i16,
    humidity: u8,
    pressure: u16,
}

impl SensorData {
    fn to_bytes(&self) -> [u8; 5] {
        let mut bytes = [0u8; 5];
        bytes[0..2].copy_from_slice(&self.temperature.to_be_bytes());
        bytes[2] = self.humidity;
        bytes[3..5].copy_from_slice(&self.pressure.to_be_bytes());
        bytes
    }
}

#[entry]
fn main() -> ! {
    // Get peripherals
    let dp = hal::stm32::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Set up clocks
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze();

    // Set up delay and timer
    let mut delay = cp.SYST.delay(&clocks);
    let mut timer = Timer::new(dp.TIM2, &clocks).start_count_down(60.hz());

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
        [
            0x2B, 0x7E, 0x15, 0x16, 0x28, 0xAE, 0xD2, 0xA6, 0xAB, 0xF7, 0x15, 0x88, 0x09, 0xCF,
            0x4F, 0x3C,
        ], // AppKey
    );

    // Create region configuration
    let region = US915::new();

    // Create LoRaWAN device
    let mut device = LoRaWANDevice::new(radio, config, region, OperatingMode::ClassA).unwrap();

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

    // Main loop - send periodic sensor data
    let mut sensor_data = SensorData::default();
    loop {
        // Simulate sensor readings
        sensor_data.temperature = 25; // 25°C
        sensor_data.humidity = 60; // 60%
        sensor_data.pressure = 1013; // 1013 hPa

        // Convert sensor data to bytes
        let data = sensor_data.to_bytes();

        // Send confirmed uplink on port 2 (sensor data port)
        if let Ok(_) = device.send_uplink(2, &data, true) {
            // Process device (handle receive windows and potential ACK)
            if let Ok(Some(downlink)) = device.process() {
                // Handle any downlink commands
                // (in this example we ignore them)
            }
        }

        // Wait for next transmission interval
        timer.wait().ok();
    }
}
