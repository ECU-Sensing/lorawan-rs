use embedded_hal::{
    blocking::spi::{Transfer, Write},
    digital::v2::{InputPin, OutputPin},
};

use crate::radio::traits::{ModulationParams, Radio, RxConfig, TxConfig};

// SX127x Register Map
const REG_FIFO: u8 = 0x00;
const REG_OP_MODE: u8 = 0x01;
const REG_FRF_MSB: u8 = 0x06;
const REG_FRF_MID: u8 = 0x07;
const REG_FRF_LSB: u8 = 0x08;
const REG_PA_CONFIG: u8 = 0x09;
const REG_PA_RAMP: u8 = 0x0A;
const REG_OCP: u8 = 0x0B;
const REG_LNA: u8 = 0x0C;
const REG_FIFO_ADDR_PTR: u8 = 0x0D;
const REG_FIFO_TX_BASE_ADDR: u8 = 0x0E;
const REG_FIFO_RX_BASE_ADDR: u8 = 0x0F;
const REG_FIFO_RX_CURRENT_ADDR: u8 = 0x10;
const REG_IRQ_FLAGS: u8 = 0x12;
const REG_RX_NB_BYTES: u8 = 0x13;
const REG_PKT_SNR_VALUE: u8 = 0x19;
const REG_PKT_RSSI_VALUE: u8 = 0x1A;
const REG_MODEM_CONFIG_1: u8 = 0x1D;
const REG_MODEM_CONFIG_2: u8 = 0x1E;
const REG_PREAMBLE_MSB: u8 = 0x20;
const REG_PREAMBLE_LSB: u8 = 0x21;
const REG_PAYLOAD_LENGTH: u8 = 0x22;
const REG_MODEM_CONFIG_3: u8 = 0x26;
const REG_FREQ_ERROR_MSB: u8 = 0x28;
const REG_FREQ_ERROR_MID: u8 = 0x29;
const REG_FREQ_ERROR_LSB: u8 = 0x2A;
const REG_RSSI_WIDEBAND: u8 = 0x2C;
const REG_DETECTION_OPTIMIZE: u8 = 0x31;
const REG_INVERTIQ: u8 = 0x33;
const REG_DETECTION_THRESHOLD: u8 = 0x37;
const REG_SYNC_WORD: u8 = 0x39;
const REG_INVERTIQ2: u8 = 0x3B;
const REG_DIO_MAPPING_1: u8 = 0x40;
const REG_VERSION: u8 = 0x42;
const REG_PA_DAC: u8 = 0x4D;

// Operating Mode bits
const MODE_LONG_RANGE_MODE: u8 = 0x80;
const MODE_SLEEP: u8 = 0x00;
const MODE_STDBY: u8 = 0x01;
const MODE_TX: u8 = 0x03;
const MODE_RX_CONTINUOUS: u8 = 0x05;
const MODE_RX_SINGLE: u8 = 0x06;

// PA Config
const PA_BOOST: u8 = 0x80;

// IRQ Flags
const IRQ_TX_DONE_MASK: u8 = 0x08;
const IRQ_PAYLOAD_CRC_ERROR_MASK: u8 = 0x20;
const IRQ_RX_DONE_MASK: u8 = 0x40;

/// Possible errors in radio operations
#[derive(Debug)]
pub enum RadioError {
    /// SPI transfer error
    Spi,
    /// GPIO error
    Gpio,
    /// Invalid configuration
    Config,
    /// Radio hardware error
    Hardware,
    /// Operation timeout
    Timeout,
}

/// SX127x Radio Driver
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

impl<SPI, CS, RESET, BUSY, DIO0, DIO1> SX127x<SPI, CS, RESET, BUSY, DIO0, DIO1>
where
    SPI: Transfer<u8> + Write<u8>,
    CS: OutputPin,
    RESET: OutputPin,
    BUSY: InputPin,
    DIO0: InputPin,
    DIO1: InputPin,
{
    /// Create new instance of SX127x driver
    pub fn new(
        spi: SPI,
        cs: CS,
        reset: RESET,
        busy: BUSY,
        dio0: DIO0,
        dio1: DIO1,
    ) -> Result<Self, RadioError> {
        let mut radio = Self {
            spi,
            cs,
            reset,
            busy,
            dio0,
            dio1,
            frequency: 0,
        };

        // Perform hardware reset
        radio.reset.set_high().map_err(|_| RadioError::Gpio)?;
        // Wait for reset
        // TODO: Use a proper delay
        for _ in 0..1000 {
            core::hint::spin_loop();
        }
        radio.reset.set_low().map_err(|_| RadioError::Gpio)?;
        // Wait for chip to start
        for _ in 0..1000 {
            core::hint::spin_loop();
        }

        // Check version
        let version = radio.read_register(REG_VERSION)?;
        if version != 0x12 {
            return Err(RadioError::Hardware);
        }

        Ok(radio)
    }

    /// Read a radio register
    fn read_register(&mut self, addr: u8) -> Result<u8, RadioError> {
        self.cs.set_low().map_err(|_| RadioError::Gpio)?;
        let mut buffer = [addr & 0x7F, 0];
        self.spi.transfer(&mut buffer).map_err(|_| RadioError::Spi)?;
        self.cs.set_high().map_err(|_| RadioError::Gpio)?;
        Ok(buffer[1])
    }

    /// Write to a radio register
    fn write_register(&mut self, addr: u8, value: u8) -> Result<(), RadioError> {
        self.cs.set_low().map_err(|_| RadioError::Gpio)?;
        let buffer = [addr | 0x80, value];
        self.spi.write(&buffer).map_err(|_| RadioError::Spi)?;
        self.cs.set_high().map_err(|_| RadioError::Gpio)?;
        Ok(())
    }

    /// Set operating mode
    fn set_mode(&mut self, mode: u8) -> Result<(), RadioError> {
        self.write_register(REG_OP_MODE, MODE_LONG_RANGE_MODE | mode)
    }

    /// Wait for busy flag to clear
    fn wait_busy(&mut self) -> Result<(), RadioError> {
        for _ in 0..1000 {
            if self.busy.is_low().map_err(|_| RadioError::Gpio)? {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(RadioError::Timeout)
    }
}

impl<SPI, CS, RESET, BUSY, DIO0, DIO1> Radio for SX127x<SPI, CS, RESET, BUSY, DIO0, DIO1>
where
    SPI: Transfer<u8> + Write<u8>,
    CS: OutputPin,
    RESET: OutputPin,
    BUSY: InputPin,
    DIO0: InputPin,
    DIO1: InputPin,
{
    type Error = RadioError;

    fn init(&mut self) -> Result<(), Self::Error> {
        // Set sleep mode
        self.set_mode(MODE_SLEEP)?;

        // Set modem config
        self.write_register(REG_MODEM_CONFIG_1, 0x72)?; // Bw = 125 kHz, Coding Rate = 4/5, Explicit Header mode
        self.write_register(REG_MODEM_CONFIG_2, 0x70)?; // SF = 7, CRC on
        self.write_register(REG_MODEM_CONFIG_3, 0x00)?; // Low Data Rate Optimize off

        // Set base addresses
        self.write_register(REG_FIFO_TX_BASE_ADDR, 0x00)?;
        self.write_register(REG_FIFO_RX_BASE_ADDR, 0x00)?;

        // Set LNA boost
        self.write_register(REG_LNA, self.read_register(REG_LNA)? | 0x03)?;

        // Set auto AGC
        self.write_register(REG_MODEM_CONFIG_3, 0x04)?;

        // Set output power to 17 dBm
        self.write_register(REG_PA_CONFIG, PA_BOOST | 0x70)?;
        self.write_register(REG_PA_DAC, 0x87)?;

        // Set Sync Word
        self.write_register(REG_SYNC_WORD, 0x34)?;

        self.set_mode(MODE_STDBY)?;

        Ok(())
    }

    fn set_frequency(&mut self, freq: u32) -> Result<(), Self::Error> {
        self.frequency = freq;
        
        // Calculate register values
        let frf = (freq as u64 * (1 << 19) / 32000000) as u32;
        
        // Write frequency registers
        self.write_register(REG_FRF_MSB, ((frf >> 16) & 0xFF) as u8)?;
        self.write_register(REG_FRF_MID, ((frf >> 8) & 0xFF) as u8)?;
        self.write_register(REG_FRF_LSB, (frf & 0xFF) as u8)?;

        Ok(())
    }

    fn set_tx_power(&mut self, power: i8) -> Result<(), Self::Error> {
        let power = power.clamp(2, 17) as u8;
        self.write_register(REG_PA_CONFIG, PA_BOOST | (power - 2))?;
        Ok(())
    }

    fn transmit(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
        // Set standby mode
        self.set_mode(MODE_STDBY)?;

        // Set payload length
        self.write_register(REG_PAYLOAD_LENGTH, buffer.len() as u8)?;

        // Reset FIFO address and payload length
        self.write_register(REG_FIFO_ADDR_PTR, 0)?;

        // Write payload to FIFO
        self.cs.set_low().map_err(|_| RadioError::Gpio)?;
        let mut spi_buffer = [REG_FIFO | 0x80];
        self.spi.write(&spi_buffer).map_err(|_| RadioError::Spi)?;
        self.spi.write(buffer).map_err(|_| RadioError::Spi)?;
        self.cs.set_high().map_err(|_| RadioError::Gpio)?;

        // Start transmission
        self.set_mode(MODE_TX)?;

        // Wait for TX done
        while !self.dio0.is_high().map_err(|_| RadioError::Gpio)? {
            core::hint::spin_loop();
        }

        // Clear IRQ flags
        self.write_register(REG_IRQ_FLAGS, 0xFF)?;

        Ok(())
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        // Set standby mode
        self.set_mode(MODE_STDBY)?;

        // Set FIFO address to current RX address
        let rx_addr = self.read_register(REG_FIFO_RX_CURRENT_ADDR)?;
        self.write_register(REG_FIFO_ADDR_PTR, rx_addr)?;

        // Get packet length
        let len = self.read_register(REG_RX_NB_BYTES)? as usize;
        if len > buffer.len() {
            return Err(RadioError::Config);
        }

        // Read payload
        self.cs.set_low().map_err(|_| RadioError::Gpio)?;
        let mut spi_buffer = [REG_FIFO & 0x7F];
        self.spi.write(&spi_buffer).map_err(|_| RadioError::Spi)?;
        self.spi.transfer(&mut buffer[..len]).map_err(|_| RadioError::Spi)?;
        self.cs.set_high().map_err(|_| RadioError::Gpio)?;

        // Clear IRQ flags
        self.write_register(REG_IRQ_FLAGS, 0xFF)?;

        Ok(len)
    }

    fn configure_tx(&mut self, config: TxConfig) -> Result<(), Self::Error> {
        self.set_frequency(config.frequency)?;
        self.set_tx_power(config.power)?;
        
        // Configure modulation parameters
        let sf = config.modulation.spreading_factor.clamp(6, 12);
        let bw = match config.modulation.bandwidth {
            b if b <= 7_800 => 0,
            b if b <= 10_400 => 1,
            b if b <= 15_600 => 2,
            b if b <= 20_800 => 3,
            b if b <= 31_250 => 4,
            b if b <= 41_700 => 5,
            b if b <= 62_500 => 6,
            b if b <= 125_000 => 7,
            b if b <= 250_000 => 8,
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
        
        // Configure modulation parameters (similar to TX)
        let sf = config.modulation.spreading_factor.clamp(6, 12);
        let bw = match config.modulation.bandwidth {
            b if b <= 7_800 => 0,
            b if b <= 10_400 => 1,
            b if b <= 15_600 => 2,
            b if b <= 20_800 => 3,
            b if b <= 31_250 => 4,
            b if b <= 41_700 => 5,
            b if b <= 62_500 => 6,
            b if b <= 125_000 => 7,
            b if b <= 250_000 => 8,
            _ => 9,
        };
        let cr = config.modulation.coding_rate.clamp(5, 8) - 4;

        let modem_config1 = (bw << 4) | (cr << 1) | 0x00;
        let modem_config2 = (sf << 4) | 0x04;
        
        self.write_register(REG_MODEM_CONFIG_1, modem_config1)?;
        self.write_register(REG_MODEM_CONFIG_2, modem_config2)?;

        // Start RX
        self.set_mode(MODE_RX_CONTINUOUS)?;

        Ok(())
    }

    fn is_receiving(&mut self) -> Result<bool, Self::Error> {
        let flags = self.read_register(REG_IRQ_FLAGS)?;
        Ok((flags & IRQ_RX_DONE_MASK) != 0)
    }

    fn get_rssi(&mut self) -> Result<i16, Self::Error> {
        let rssi_value = self.read_register(REG_PKT_RSSI_VALUE)?;
        Ok(-137 + rssi_value as i16)
    }

    fn get_snr(&mut self) -> Result<i8, Self::Error> {
        let snr = self.read_register(REG_PKT_SNR_VALUE)?;
        Ok((snr as i8) / 4)
    }

    fn sleep(&mut self) -> Result<(), Self::Error> {
        self.set_mode(MODE_SLEEP)
    }

    fn standby(&mut self) -> Result<(), Self::Error> {
        self.set_mode(MODE_STDBY)
    }

    fn is_transmitting(&mut self) -> Result<bool, Self::Error> {
        let op_mode = self.read_register(REG_OP_MODE)?;
        Ok((op_mode & 0x07) == MODE_TX)
    }
} 