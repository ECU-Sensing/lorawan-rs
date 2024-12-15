# Setting up lorawan-rs on Adafruit Feather M0 with RFM95 LoRa Radio

This guide will walk you through the process of setting up and running the lorawan-rs crate on an Adafruit Feather M0 with RFM95 LoRa Radio (900MHz).

## Prerequisites

1. **Hardware Requirements**
   - Adafruit Feather M0 with RFM95W (900 MHz)
   - USB cable for programming
   - Antenna (915MHz compatible)

2. **Software Requirements**
   - Rust toolchain (install from https://rustup.rs)
   - ARM Cortex-M0+ toolchain
   - BOSSA flasher tool (for Feather M0)

## Installation Steps

1. **Install Rust and Required Components**
   ```bash
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Add ARM Cortex-M0+ target
   rustup target add thumbv6m-none-eabi
   
   # Install cargo-hf2 for easier flashing
   cargo install cargo-hf2
   ```

2. **Install BOSSA**
   - macOS: `brew install bossa`
   - Linux: `sudo apt-get install bossa-cli`
   - Windows: Download from https://github.com/shumatech/BOSSA/releases

3. **Create Project Configuration**
   Create a `.cargo/config.toml` file in your project root:
   ```toml
   [build]
   target = "thumbv6m-none-eabi"

   [target.thumbv6m-none-eabi]
   runner = "hf2 upload"
   rustflags = [
     "-C", "link-arg=-Tlink.x",
   ]
   ```

## Hardware Setup

1. **Pin Connections**
   The Feather M0 RFM95 has the following default pin connections:
   - SPI SCK: Pin 24
   - SPI MOSI: Pin 23
   - SPI MISO: Pin 22
   - CS (Chip Select): Pin 8
   - RST: Pin 4
   - DIO0: Pin 6
   - DIO1: Pin 5
   - DIO2: Pin 3

2. **Antenna Connection**
   - Attach a 915MHz compatible antenna to the uFL connector
   - Or solder a quarter-wave wire antenna (~8.2cm for 915MHz)

## Building and Flashing

1. **Build the Project**
   ```bash
   # Build for release
   cargo build --release
   
   # Or build for debugging
   cargo build
   ```

2. **Enter Bootloader Mode**
   - Double-press the reset button on the Feather M0
   - The red LED should pulse, indicating bootloader mode

3. **Flash the Board**
   ```bash
   # Using cargo-hf2
   cargo run --release
   
   # Or manually with BOSSA
   bossac -p /dev/ttyACM0 -e -w -v -R target/thumbv6m-none-eabi/release/your-project-name
   ```

## Troubleshooting

1. **Board Not Detected**
   - Ensure the board is in bootloader mode (pulsing red LED)
   - Check USB cable connection
   - Verify USB port permissions (Linux users may need to add udev rules)

2. **Build Errors**
   - Ensure all required dependencies are in `Cargo.toml`
   - Verify the correct target is selected in `.cargo/config.toml`
   - Check that all pin assignments match your board revision

3. **Runtime Issues**
   - Verify antenna connection
   - Check LoRaWAN credentials and frequency settings
   - Monitor serial output for debugging (115200 baud)

## Example Usage

Here's a minimal example to test your setup:

```rust
#![no_std]
#![no_main]

use panic_halt as _;
use cortex_m_rt::entry;
use lorawan_node::{
    radio::sx127x::SX127x,
    config::{DeviceConfig, RegionConfig},
    class::class_a::ClassA,
};

#[entry]
fn main() -> ! {
    // Your initialization code here
    loop {
        // Your application code here
    }
}
```

## Next Steps

1. Review the `Examples.md` file for more detailed code examples
2. Configure your LoRaWAN network credentials
3. Test basic connectivity with your network server
4. Implement your specific application logic

## Support

For issues related to:
- Hardware setup: Refer to Adafruit's Feather M0 documentation
- lorawan-rs crate: Open an issue on the GitHub repository
- LoRaWAN network: Contact your network provider 