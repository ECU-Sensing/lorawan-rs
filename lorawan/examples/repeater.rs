#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;
use atsamd21_hal as hal;

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

// Add LED type aliases
type RedLed = Pa17<Output<PushPull>>;    // Built-in red LED on pin 13
type BlueLed = Pa10<Output<PushPull>>;   // Built-in blue LED on pin 32

/// LED status patterns
struct StatusLeds {
    red: RedLed,
    blue: BlueLed,
    packet_count: u32,
}

impl StatusLeds {
    fn new(red: RedLed, blue: BlueLed) -> Self {
        Self { 
            red, 
            blue,
            packet_count: 0,
        }
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
        // Slow breathing pattern on red LED based on packet count
        if self.packet_count % 20 == 0 {
            self.red.toggle().ok();
        }
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
        self.packet_count = self.packet_count.wrapping_add(1);
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
}

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

    // Configure LED pins
    let red_led = pins.d13.into_push_pull_output();
    let blue_led = pins.d32.into_push_pull_output();
    let mut status_leds = StatusLeds::new(red_led, blue_led);

    // Initialize radio with debug output
    let mut radio = match SX127x::new(spi, cs, reset, dio0, dio1, &mut delay) {
        Ok(radio) => {
            status_leds.indicate_init_success(&mut delay);
            radio
        }
        Err(_) => {
            status_leds.indicate_init_failure(&mut delay);
            loop { // Halt with error pattern
                status_leds.red.set_high().ok();
                delay.delay_ms(100u32);
                status_leds.red.set_low().ok();
                delay.delay_ms(900u32);
            }
        }
    };

    // Configure radio with TTN US915 sub-band 2 settings
    if let Err(_) = radio.init() {
        status_leds.indicate_error(&mut delay);
        loop {
            delay.delay_ms(1000u32);
        }
    }

    // Configure for TTN US915 sub-band 2 (channels 8-15)
    let base_freq = 903_900_000; // Start of sub-band 2
    radio.set_frequency(base_freq).unwrap();
    radio.set_rx_config(
        lorawan::radio::traits::RxConfig {
            frequency: base_freq,
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

    // Main loop with LED status indicators
    let mut rx_buffer = [0u8; 255];
    loop {
        // Show listening status
        status_leds.indicate_listening();

        // Receive packet
        match radio.receive(&mut rx_buffer) {
            Ok(len) if len > 0 => {
                status_leds.indicate_packet_received();
                
                // Validate packet
                if let Some(valid) = validate_lorawan_packet(&rx_buffer[..len]) {
                    if valid {
                        // Get the frequency we received on
                        let current_freq = match radio.get_frequency() {
                            Ok(freq) => freq,
                            Err(_) => {
                                status_leds.indicate_error(&mut delay);
                                continue;
                            }
                        };

                        status_leds.indicate_packet_forwarding();
                        
                        // Forward packet on same frequency
                        match radio.transmit(&rx_buffer[..len]) {
                            Ok(_) => {
                                status_leds.indicate_packet_forwarded();
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
            _ => {} // No packet received
        }

        // Small delay to prevent tight loop
        delay.delay_ms(10u32);
    }
}

/// Validate a LoRaWAN packet
/// Returns Some(true) if packet should be forwarded, Some(false) if not, None if invalid
fn validate_lorawan_packet(data: &[u8]) -> Option<bool> {
    if data.len() < 8 {
        return None;  // Packet too short to be valid LoRaWAN
    }

    let mtype = data[0] & 0xE0;
    // Accept uplink data (0x40) and downlink data (0x80) messages
    Some(mtype == 0x40 || mtype == 0x80)
}

/// Helper function to check if a packet is a duplicate
/// (could be implemented to prevent forwarding the same packet multiple times)
fn is_duplicate(packet: &[u8]) -> bool {
    // Implement duplicate detection logic here if needed
    // For example, keep a rolling history of frame counters per DevAddr
    false
} 