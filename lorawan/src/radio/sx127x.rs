use embedded_hal::blocking::spi::{Transfer, Write};
use embedded_hal::digital::v2::{InputPin, OutputPin};

use super::traits::{Radio, RxConfig, TxConfig};

// Register addresses
const REG_FIFO: u8 = 0x00;
const REG_OP_MODE: u8 = 0x01;
const REG_FRF_MSB: u8 = 0x06;
const REG_FRF_MID: u8 = 0x07;
const REG_FRF_LSB: u8 = 0x08;
const REG_PA_CONFIG: u8 = 0x09;
const REG_MODEM_CONFIG_1: u8 = 0x1D;
const REG_MODEM_CONFIG_2: u8 = 0x1E;
const REG_IRQ_FLAGS: u8 = 0x12;

// Operating modes
const MODE_SLEEP: u8 = 0x00;
const MODE_STDBY: u8 = 0x01;
const MODE_TX: u8 = 0x03;
const MODE_RX: u8 = 0x05;

// IRQ flags
const IRQ_TX_DONE_MASK: u8 = 0x08;
const IRQ_RX_DONE_MASK: u8 = 0x40;
const IRQ_RX_TIMEOUT_MASK: u8 = 0x80;

/// SPI error trait
pub trait SpiError: core::fmt::Debug {}

// Implement SpiError for embedded-hal SPI error types
impl<E: core::fmt::Debug> SpiError for E {}

/// Radio errors
#[derive(Debug)]
pub enum SX127xError<E, CSE, RESETE> {
    /// SPI error
    Spi(E),
    /// CS pin error
    Cs(CSE),
    /// Reset pin error
    Reset(RESETE),
    /// Invalid frequency
    InvalidFrequency,
    /// Invalid power
    InvalidPower,
    /// Invalid configuration
    InvalidConfig,
}

/// SX127x driver
pub struct SX127x<SPI, CS, RESET, BUSY, DIO0, DIO1>
where
    SPI: Transfer<u8> + Write<u8>,
    CS: OutputPin,
    RESET: OutputPin,
    BUSY: InputPin,
    DIO0: InputPin,
    DIO1: InputPin,
{
    spi: SPI,
    cs: CS,
    reset: RESET,
    busy: BUSY,
    dio0: DIO0,
    dio1: DIO1,
    frequency: u32,
}

impl<SPI, CS, RESET, BUSY, DIO0, DIO1, E, CSE, RESETE> SX127x<SPI, CS, RESET, BUSY, DIO0, DIO1>
where
    SPI: Transfer<u8, Error = E> + Write<u8, Error = E>,
    CS: OutputPin<Error = CSE>,
    RESET: OutputPin<Error = RESETE>,
    BUSY: InputPin,
    DIO0: InputPin,
    DIO1: InputPin,
    E: core::fmt::Debug,
    CSE: core::fmt::Debug,
    RESETE: core::fmt::Debug,
{
    /// Create new instance
    pub fn new(
        spi: SPI,
        cs: CS,
        reset: RESET,
        busy: BUSY,
        dio0: DIO0,
        dio1: DIO1,
    ) -> Result<Self, SX127xError<E, CSE, RESETE>> {
        let mut sx127x = Self {
            spi,
            cs,
            reset,
            busy,
            dio0,
            dio1,
            frequency: 0,
        };

        // Initialize the radio
        sx127x.init()?;

        Ok(sx127x)
    }

    /// Read register
    fn read_register(&mut self, addr: u8, buffer: &mut [u8], len: usize) -> Result<(), SX127xError<E, CSE, RESETE>> {
        // Set CS low to start transaction
        self.cs.set_low().map_err(SX127xError::Cs)?;

        // Send address and read command
        let mut read_cmd = [addr | 0x80];
        self.spi.transfer(&mut read_cmd).map_err(SX127xError::Spi)?;

        // Read data
        let mut rx_byte = [0u8];
        for i in 0..len {
            self.spi.transfer(&mut rx_byte).map_err(SX127xError::Spi)?;
            buffer[i] = rx_byte[0];
        }

        // Set CS high to end transaction
        self.cs.set_high().map_err(SX127xError::Cs)?;

        Ok(())
    }

    /// Write register
    fn write_register(&mut self, addr: u8, value: u8) -> Result<(), SX127xError<E, CSE, RESETE>> {
        self.cs.set_low().map_err(|e| SX127xError::Cs(e))?;
        let buffer = [addr | 0x80, value];
        self.spi.write(&buffer).map_err(|e| SX127xError::Spi(e))?;
        self.cs.set_high().map_err(|e| SX127xError::Cs(e))?;
        Ok(())
    }

    /// Set operating mode
    fn set_mode(&mut self, mode: u8) -> Result<(), SX127xError<E, CSE, RESETE>> {
        self.write_register(REG_OP_MODE, mode | 0x80)
    }

    /// Read from FIFO
    fn read_fifo(&mut self, buffer: &mut [u8]) -> Result<(), SX127xError<E, CSE, RESETE>> {
        // Read FIFO data into buffer
        self.cs.set_low().map_err(SX127xError::Cs)?;
        
        // First byte is the FIFO read command
        let mut read_cmd = [0x00];
        self.spi.transfer(&mut read_cmd).map_err(SX127xError::Spi)?;
        
        // Read the actual data
        for byte in buffer.iter_mut() {
            let mut rx_byte = [0x00];
            self.spi.transfer(&mut rx_byte).map_err(SX127xError::Spi)?;
            *byte = rx_byte[0];
        }
        
        self.cs.set_high().map_err(SX127xError::Cs)?;
        Ok(())
    }

    /// Write to FIFO
    fn write_fifo(&mut self, data: &[u8]) -> Result<(), SX127xError<E, CSE, RESETE>> {
        let spi_buffer = [REG_FIFO & 0x7F];
        self.cs.set_low().map_err(SX127xError::Cs)?;
        self.spi.write(&spi_buffer).map_err(SX127xError::Spi)?;
        self.spi.write(data).map_err(SX127xError::Spi)?;
        self.cs.set_high().map_err(SX127xError::Cs)?;
        Ok(())
    }
}

impl<SPI, CS, RESET, BUSY, DIO0, DIO1, E, CSE, RESETE> Radio for SX127x<SPI, CS, RESET, BUSY, DIO0, DIO1>
where
    SPI: Transfer<u8, Error = E> + Write<u8, Error = E>,
    CS: OutputPin<Error = CSE>,
    RESET: OutputPin<Error = RESETE>,
    BUSY: InputPin,
    DIO0: InputPin,
    DIO1: InputPin,
    E: core::fmt::Debug,
    CSE: core::fmt::Debug,
    RESETE: core::fmt::Debug,
{
    type Error = SX127xError<E, CSE, RESETE>;

    fn init(&mut self) -> Result<(), Self::Error> {
        // Reset radio
        self.reset.set_low().map_err(SX127xError::Reset)?;
        // Wait for reset
        for _ in 0..100 {
            if self.busy.is_low().unwrap_or(false) {
                break;
            }
        }
        self.reset.set_high().map_err(SX127xError::Reset)?;

        // Set sleep mode
        self.set_mode(MODE_SLEEP)?;

        Ok(())
    }

    fn set_frequency(&mut self, freq: u32) -> Result<(), Self::Error> {
        if freq < 137_000_000 || freq > 1_020_000_000 {
            return Err(SX127xError::InvalidFrequency);
        }

        self.frequency = freq;

        // Calculate register values
        let frf = (freq as u64 * (1 << 19) / 32_000_000) as u32;

        // Write frequency registers
        self.write_register(REG_FRF_MSB, ((frf >> 16) & 0xFF) as u8)?;
        self.write_register(REG_FRF_MID, ((frf >> 8) & 0xFF) as u8)?;
        self.write_register(REG_FRF_LSB, (frf & 0xFF) as u8)?;

        Ok(())
    }

    fn set_tx_power(&mut self, power: i8) -> Result<(), Self::Error> {
        if power < 2 || power > 20 {
            return Err(SX127xError::InvalidPower);
        }
        self.write_register(REG_PA_CONFIG, 0x80 | (power - 2) as u8)
    }

    fn configure_tx(&mut self, config: TxConfig) -> Result<(), Self::Error> {
        self.set_frequency(config.frequency)?;
        self.set_tx_power(config.power)?;

        // Configure modulation parameters
        let sf = config.modulation.spreading_factor.clamp(6, 12);
        let bw = match config.modulation.bandwidth {
            b if b <= 7800 => 0,
            b if b <= 10400 => 1,
            b if b <= 15600 => 2,
            b if b <= 20800 => 3,
            b if b <= 31250 => 4,
            b if b <= 41700 => 5,
            b if b <= 62500 => 6,
            b if b <= 125000 => 7,
            b if b <= 250000 => 8,
            _ => 9,
        };
        let cr = config.modulation.coding_rate.clamp(5, 8) - 4;

        let modem_config1 = (bw << 4) | (cr << 1) | 0x00; // Explicit header mode
        let modem_config2 = (sf << 4) | 0x04; // CRC on

        self.write_register(REG_MODEM_CONFIG_1, modem_config1)?;
        self.write_register(REG_MODEM_CONFIG_2, modem_config2)?;

        Ok(())
    }

    fn configure_rx(&mut self, config: RxConfig) -> Result<(), Self::Error> {
        self.set_frequency(config.frequency)?;

        // Configure modulation parameters
        let sf = config.modulation.spreading_factor.clamp(6, 12);
        let bw = match config.modulation.bandwidth {
            b if b <= 7800 => 0,
            b if b <= 10400 => 1,
            b if b <= 15600 => 2,
            b if b <= 20800 => 3,
            b if b <= 31250 => 4,
            b if b <= 41700 => 5,
            b if b <= 62500 => 6,
            b if b <= 125000 => 7,
            b if b <= 250000 => 8,
            _ => 9,
        };
        let cr = config.modulation.coding_rate.clamp(5, 8) - 4;

        let modem_config1 = (bw << 4) | (cr << 1) | 0x00;
        let modem_config2 = (sf << 4) | 0x04;

        self.write_register(REG_MODEM_CONFIG_1, modem_config1)?;
        self.write_register(REG_MODEM_CONFIG_2, modem_config2)?;

        // Set RX mode
        self.set_mode(MODE_RX)?;

        Ok(())
    }

    fn transmit(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        // Write data to FIFO
        self.write_fifo(data)?;

        // Set TX mode
        self.set_mode(MODE_TX)?;

        // Wait for TX done using DIO0
        while !self.dio0.is_high().unwrap_or(false) {}

        // Clear IRQ flags
        self.write_register(REG_IRQ_FLAGS, IRQ_TX_DONE_MASK)?;

        // Back to standby
        self.set_mode(MODE_STDBY)?;

        Ok(())
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        // Set RX mode
        self.set_mode(MODE_RX)?;

        // Wait for RX done or timeout using DIO0 and DIO1
        loop {
            if self.dio0.is_high().unwrap_or(false) {
                // RX done
                break;
            }
            if self.dio1.is_high().unwrap_or(false) {
                // RX timeout
                return Ok(0);
            }
        }

        // Read data from FIFO
        self.read_fifo(buffer)?;

        // Clear IRQ flags
        self.write_register(REG_IRQ_FLAGS, IRQ_RX_DONE_MASK | IRQ_RX_TIMEOUT_MASK)?;

        // Back to standby
        self.set_mode(MODE_STDBY)?;

        Ok(buffer.len())
    }

    fn get_rssi(&mut self) -> Result<i16, Self::Error> {
        let mut buffer = [0u8];
        self.read_register(0x1B, &mut buffer, 1)?;
        Ok(-157 + buffer[0] as i16)
    }

    fn get_snr(&mut self) -> Result<i8, Self::Error> {
        let mut buffer = [0u8];
        self.read_register(0x19, &mut buffer, 1)?;
        Ok((buffer[0] as i8) / 4)
    }

    fn is_transmitting(&mut self) -> Result<bool, Self::Error> {
        let mut buffer = [0u8];
        self.read_register(REG_IRQ_FLAGS, &mut buffer, 1)?;
        Ok((buffer[0] & IRQ_TX_DONE_MASK) != 0)
    }

    fn set_rx_gain(&mut self, gain: u8) -> Result<(), Self::Error> {
        // LNA gain setting
        let lna_gain = match gain {
            0 => 0x20, // Max gain
            1 => 0x40, // Max gain - 6dB
            2 => 0x60, // Max gain - 12dB
            3 => 0x80, // Max gain - 24dB
            4 => 0xA0, // Max gain - 36dB
            5 => 0xC0, // Max gain - 48dB
            _ => 0x20, // Default to max gain
        };
        self.write_register(0x0C, lna_gain)
    }

    fn set_low_power_mode(&mut self, enabled: bool) -> Result<(), Self::Error> {
        if enabled {
            self.set_mode(MODE_SLEEP)
        } else {
            self.set_mode(MODE_STDBY)
        }
    }

    fn sleep(&mut self) -> Result<(), Self::Error> {
        self.set_mode(MODE_SLEEP)
    }
}
