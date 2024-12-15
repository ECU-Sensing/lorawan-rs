#[cfg(feature = "sx126x")]
use embedded_hal::{
    blocking::spi::{Transfer, Write},
    digital::v2::{InputPin, OutputPin},
    blocking::delay::DelayMs,
};

#[cfg(feature = "sx126x")]
use crate::radio::traits::{ModulationParams, Radio, RxConfig, TxConfig};

// SX126x Register Map
#[cfg(feature = "sx126x")]
mod registers {
    pub const REG_WHITENING_INITIAL_MSB: u16 = 0x06B8;
    pub const REG_WHITENING_INITIAL_LSB: u16 = 0x06B9;
    pub const REG_CRC_INITIAL_MSB: u16 = 0x06BC;
    pub const REG_CRC_INITIAL_LSB: u16 = 0x06BD;
    pub const REG_CRC_POLYNOMIAL_MSB: u16 = 0x06BE;
    pub const REG_CRC_POLYNOMIAL_LSB: u16 = 0x06BF;
    pub const REG_SYNC_WORD_0: u16 = 0x06C0;
    pub const REG_SYNC_WORD_1: u16 = 0x06C1;
    pub const REG_SYNC_WORD_2: u16 = 0x06C2;
    pub const REG_SYNC_WORD_3: u16 = 0x06C3;
    pub const REG_SYNC_WORD_4: u16 = 0x06C4;
    pub const REG_SYNC_WORD_5: u16 = 0x06C5;
    pub const REG_SYNC_WORD_6: u16 = 0x06C6;
    pub const REG_SYNC_WORD_7: u16 = 0x06C7;
    pub const REG_NODE_ADDRESS: u16 = 0x06CD;
    pub const REG_BROADCAST_ADDRESS: u16 = 0x06CE;
    pub const REG_IQ_POLARITY_SETUP: u16 = 0x0736;
    pub const REG_LORA_SYNC_WORD_MSB: u16 = 0x0740;
    pub const REG_LORA_SYNC_WORD_LSB: u16 = 0x0741;
}

#[cfg(feature = "sx126x")]
mod commands {
    pub const SET_SLEEP: u8 = 0x84;
    pub const SET_STANDBY: u8 = 0x80;
    pub const SET_FS: u8 = 0xC1;
    pub const SET_TX: u8 = 0x83;
    pub const SET_RX: u8 = 0x82;
    pub const STOP_TIMER_ON_PREAMBLE: u8 = 0x9F;
    pub const SET_RX_DUTY_CYCLE: u8 = 0x94;
    pub const SET_CAD: u8 = 0xC5;
    pub const SET_TX_CONTINUOUS_WAVE: u8 = 0xD1;
    pub const SET_TX_INFINITE_PREAMBLE: u8 = 0xD2;
    pub const SET_REGULATOR_MODE: u8 = 0x96;
    pub const CALIBRATE: u8 = 0x89;
    pub const CALIBRATE_IMAGE: u8 = 0x98;
    pub const SET_PA_CONFIG: u8 = 0x95;
    pub const SET_RX_TX_FALLBACK_MODE: u8 = 0x93;
    pub const WRITE_REGISTER: u8 = 0x0D;
    pub const READ_REGISTER: u8 = 0x1D;
    pub const WRITE_BUFFER: u8 = 0x0E;
    pub const READ_BUFFER: u8 = 0x1E;
    pub const SET_DIO_IRQ_PARAMS: u8 = 0x08;
    pub const GET_IRQ_STATUS: u8 = 0x12;
    pub const CLR_IRQ_STATUS: u8 = 0x02;
    pub const SET_DIO2_AS_RF_SWITCH_CTRL: u8 = 0x9D;
    pub const SET_DIO3_AS_TCXO_CTRL: u8 = 0x97;
    pub const SET_RF_FREQUENCY: u8 = 0x86;
    pub const SET_PKT_TYPE: u8 = 0x8A;
    pub const GET_PKT_TYPE: u8 = 0x11;
    pub const SET_TX_PARAMS: u8 = 0x8E;
    pub const SET_MODULATION_PARAMS: u8 = 0x8B;
    pub const SET_PKT_PARAMS: u8 = 0x8C;
    pub const GET_PKT_STATUS: u8 = 0x14;
    pub const GET_RSSI_INST: u8 = 0x15;
    pub const GET_STATS: u8 = 0x10;
    pub const RESET_STATS: u8 = 0x00;
}

#[cfg(feature = "sx126x")]
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

#[cfg(feature = "sx126x")]
pub struct SX126x<SPI, CS, RESET, BUSY, DIO1, DELAY>
where
    SPI: Transfer<u8> + Write<u8>,
    CS: OutputPin,
    RESET: OutputPin,
    BUSY: InputPin,
    DIO1: InputPin,
    DELAY: DelayMs<u32>,
{
    spi: SPI,
    cs: CS,
    reset: RESET,
    busy: BUSY,
    dio1: DIO1,
    delay: DELAY,
    frequency: u32,
}

#[cfg(feature = "sx126x")]
impl<SPI, CS, RESET, BUSY, DIO1, DELAY> SX126x<SPI, CS, RESET, BUSY, DIO1, DELAY>
where
    SPI: Transfer<u8> + Write<u8>,
    CS: OutputPin,
    RESET: OutputPin,
    BUSY: InputPin,
    DIO1: InputPin,
    DELAY: DelayMs<u32>,
{
    /// Create new SX126x driver instance
    ///
    /// # Arguments
    /// * `spi` - SPI interface
    /// * `cs` - Chip select pin
    /// * `reset` - Reset pin
    /// * `busy` - Busy pin
    /// * `dio1` - DIO1 interrupt pin
    /// * `delay` - Delay implementation
    pub fn new(
        spi: SPI,
        cs: CS,
        reset: RESET,
        busy: BUSY,
        dio1: DIO1,
        delay: DELAY,
    ) -> Result<Self, RadioError> {
        let mut radio = Self {
            spi,
            cs,
            reset,
            busy,
            dio1,
            delay,
            frequency: 0,
        };

        // Reset sequence
        radio.reset.set_high().map_err(|_| RadioError::Gpio)?;
        radio.delay.delay_ms(2); // 2ms high pulse
        radio.reset.set_low().map_err(|_| RadioError::Gpio)?;
        radio.delay.delay_ms(10); // 10ms low for reset

        // Wait for busy to go low indicating device is ready
        radio.wait_busy()?;

        Ok(radio)
    }

    fn wait_busy(&mut self) -> Result<(), RadioError> {
        for _ in 0..1000 {
            if self.busy.is_low().map_err(|_| RadioError::Gpio)? {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(RadioError::Timeout)
    }

    fn write_command(&mut self, command: u8, data: &[u8]) -> Result<(), RadioError> {
        self.cs.set_low().map_err(|_| RadioError::Gpio)?;
        self.spi.write(&[command]).map_err(|_| RadioError::Spi)?;
        if !data.is_empty() {
            self.spi.write(data).map_err(|_| RadioError::Spi)?;
        }
        self.cs.set_high().map_err(|_| RadioError::Gpio)?;
        self.wait_busy()
    }

    fn read_command(&mut self, command: u8, data: &mut [u8]) -> Result<(), RadioError> {
        self.cs.set_low().map_err(|_| RadioError::Gpio)?;
        self.spi.write(&[command]).map_err(|_| RadioError::Spi)?;
        self.spi.write(&[0]).map_err(|_| RadioError::Spi)?; // NOP for response
        if !data.is_empty() {
            self.spi.transfer(data).map_err(|_| RadioError::Spi)?;
        }
        self.cs.set_high().map_err(|_| RadioError::Gpio)?;
        self.wait_busy()
    }

    fn write_register(&mut self, address: u16, data: &[u8]) -> Result<(), RadioError> {
        let addr_bytes = [(address >> 8) as u8, address as u8];
        self.write_command(commands::WRITE_REGISTER, &[&addr_bytes, data].concat())
    }

    fn read_register(&mut self, address: u16, data: &mut [u8]) -> Result<(), RadioError> {
        let addr_bytes = [(address >> 8) as u8, address as u8];
        self.cs.set_low().map_err(|_| RadioError::Gpio)?;
        self.spi
            .write(&[commands::READ_REGISTER])
            .map_err(|_| RadioError::Spi)?;
        self.spi.write(&addr_bytes).map_err(|_| RadioError::Spi)?;
        self.spi.write(&[0]).map_err(|_| RadioError::Spi)?; // NOP
        self.spi.transfer(data).map_err(|_| RadioError::Spi)?;
        self.cs.set_high().map_err(|_| RadioError::Gpio)?;
        self.wait_busy()
    }
}

#[cfg(feature = "sx126x")]
impl<SPI, CS, RESET, BUSY, DIO1, DELAY> Radio for SX126x<SPI, CS, RESET, BUSY, DIO1, DELAY>
where
    SPI: Transfer<u8> + Write<u8>,
    CS: OutputPin,
    RESET: OutputPin,
    BUSY: InputPin,
    DIO1: InputPin,
    DELAY: DelayMs<u32>,
{
    type Error = RadioError;

    fn init(&mut self) -> Result<(), Self::Error> {
        // Set to standby mode
        self.write_command(commands::SET_STANDBY, &[0])?; // STDBY_RC

        // Set packet type to LoRa
        self.write_command(commands::SET_PKT_TYPE, &[0x01])?;

        // Set DIO2 as RF switch control
        self.write_command(commands::SET_DIO2_AS_RF_SWITCH_CTRL, &[0x01])?;

        // Configure for LoRa operation
        self.write_register(registers::REG_LORA_SYNC_WORD_MSB, &[0x34, 0x44])?;

        // Set regulator mode to DC-DC
        self.write_command(commands::SET_REGULATOR_MODE, &[0x01])?;

        // Calibrate all blocks
        self.write_command(commands::CALIBRATE, &[0x7F])?;

        Ok(())
    }

    fn set_frequency(&mut self, freq: u32) -> Result<(), Self::Error> {
        self.frequency = freq;
        let frf = ((freq as u64) << 25) / 32000000;
        let freq_bytes = [
            ((frf >> 24) & 0xFF) as u8,
            ((frf >> 16) & 0xFF) as u8,
            ((frf >> 8) & 0xFF) as u8,
            (frf & 0xFF) as u8,
        ];
        self.write_command(commands::SET_RF_FREQUENCY, &freq_bytes)
    }

    fn set_tx_power(&mut self, power: i8) -> Result<(), Self::Error> {
        let power = power.clamp(2, 22) as u8;
        // Configure PA
        self.write_command(commands::SET_PA_CONFIG, &[0x04, 0x07, 0x00, 0x01])?;
        // Set power
        self.write_command(commands::SET_TX_PARAMS, &[power, 0x04])
    }

    fn transmit(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
        // Write data to buffer
        self.write_command(commands::WRITE_BUFFER, &[0, &buffer[..]].concat())?;

        // Set packet parameters
        let packet_params = [
            0x00,               // Preamble length MSB
            0x08,               // Preamble length LSB
            0x00,               // Header type (explicit)
            buffer.len() as u8, // Payload length
            0x01,               // CRC on
            0x00,               // Standard IQ
        ];
        self.write_command(commands::SET_PKT_PARAMS, &packet_params)?;

        // Start transmission
        self.write_command(commands::SET_TX, &[0x00, 0x00, 0x00])?;

        // Wait for TX done interrupt
        while !self.dio1.is_high().map_err(|_| RadioError::Gpio)? {
            core::hint::spin_loop();
        }

        // Clear IRQ status
        self.write_command(commands::CLR_IRQ_STATUS, &[0xFF, 0xFF])?;

        Ok(())
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        // Set to RX mode
        self.write_command(commands::SET_RX, &[0x00, 0x00, 0x00])?;

        // Wait for RX done interrupt
        while !self.dio1.is_high().map_err(|_| RadioError::Gpio)? {
            core::hint::spin_loop();
        }

        // Get the packet status
        let mut status = [0u8; 2];
        self.read_command(commands::GET_PKT_STATUS, &mut status)?;

        // Read the received data
        let mut rx_len = [0u8];
        self.read_command(commands::READ_BUFFER, &mut rx_len)?;
        let len = rx_len[0] as usize;
        if len > buffer.len() {
            return Err(RadioError::Config);
        }

        self.cs.set_low().map_err(|_| RadioError::Gpio)?;
        self.spi
            .write(&[commands::READ_BUFFER, 0x00])
            .map_err(|_| RadioError::Spi)?;
        self.spi
            .transfer(&mut buffer[..len])
            .map_err(|_| RadioError::Spi)?;
        self.cs.set_high().map_err(|_| RadioError::Gpio)?;

        // Clear IRQ status
        self.write_command(commands::CLR_IRQ_STATUS, &[0xFF, 0xFF])?;

        Ok(len)
    }

    fn configure_tx(&mut self, config: TxConfig) -> Result<(), Self::Error> {
        self.set_frequency(config.frequency)?;
        self.set_tx_power(config.power)?;

        // Set modulation parameters
        let sf = config.modulation.spreading_factor.clamp(5, 12);
        let bw = match config.modulation.bandwidth {
            b if b <= 10_400 => 0x00,
            b if b <= 15_600 => 0x01,
            b if b <= 20_800 => 0x02,
            b if b <= 31_250 => 0x03,
            b if b <= 41_700 => 0x04,
            b if b <= 62_500 => 0x05,
            b if b <= 125_000 => 0x06,
            b if b <= 250_000 => 0x07,
            _ => 0x08,
        };
        let cr = config.modulation.coding_rate.clamp(5, 8) - 4;

        let mod_params = [
            sf,   // SF5-SF12
            bw,   // Bandwidth
            cr,   // Coding rate
            0x00, // Low data rate optimize off
        ];

        self.write_command(commands::SET_MODULATION_PARAMS, &mod_params)
    }

    fn configure_rx(&mut self, config: RxConfig) -> Result<(), Self::Error> {
        self.set_frequency(config.frequency)?;

        // Set modulation parameters (similar to TX)
        let sf = config.modulation.spreading_factor.clamp(5, 12);
        let bw = match config.modulation.bandwidth {
            b if b <= 10_400 => 0x00,
            b if b <= 15_600 => 0x01,
            b if b <= 20_800 => 0x02,
            b if b <= 31_250 => 0x03,
            b if b <= 41_700 => 0x04,
            b if b <= 62_500 => 0x05,
            b if b <= 125_000 => 0x06,
            b if b <= 250_000 => 0x07,
            _ => 0x08,
        };
        let cr = config.modulation.coding_rate.clamp(5, 8) - 4;

        let mod_params = [
            sf, bw, cr, 0x00, // Low data rate optimize off
        ];

        self.write_command(commands::SET_MODULATION_PARAMS, &mod_params)?;

        // Set to RX continuous mode
        self.write_command(commands::SET_RX, &[0xFF, 0xFF, 0xFF])
    }

    fn is_receiving(&mut self) -> Result<bool, Self::Error> {
        let mut irq_status = [0u8; 2];
        self.read_command(commands::GET_IRQ_STATUS, &mut irq_status)?;
        Ok((irq_status[0] & 0x02) != 0) // RX done bit
    }

    fn get_rssi(&mut self) -> Result<i16, Self::Error> {
        let mut rssi = [0u8];
        self.read_command(commands::GET_RSSI_INST, &mut rssi)?;
        Ok(-i16::from(rssi[0]) / 2)
    }

    fn get_snr(&mut self) -> Result<i8, Self::Error> {
        let mut status = [0u8; 2];
        self.read_command(commands::GET_PKT_STATUS, &mut status)?;
        Ok((status[1] as i8) / 4)
    }

    fn sleep(&mut self) -> Result<(), Self::Error> {
        self.write_command(commands::SET_SLEEP, &[0x00])
    }

    fn standby(&mut self) -> Result<(), Self::Error> {
        self.write_command(commands::SET_STANDBY, &[0x00])
    }

    fn is_transmitting(&mut self) -> Result<bool, Self::Error> {
        let mut status = [0u8; 2];
        self.read_command(commands::GET_IRQ_STATUS, &mut status)?;
        Ok((status[0] & 0x01) != 0) // TX done bit
    }
}
