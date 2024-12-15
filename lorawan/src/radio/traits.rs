/// Radio error type
#[derive(Debug)]
pub enum RadioError {
    /// SPI communication error
    Spi,
    /// GPIO error
    Gpio,
    /// Invalid configuration
    InvalidConfig,
    /// Operation timeout
    Timeout,
}

/// Radio modulation parameters
#[derive(Debug, Clone, Copy)]
pub struct ModulationParams {
    /// Spreading factor (7-12)
    pub spreading_factor: u8,
    /// Bandwidth in Hz
    pub bandwidth: u32,
    /// Coding rate (4/5, 4/6, 4/7, 4/8)
    pub coding_rate: u8,
}

/// Radio transmit configuration
#[derive(Debug, Clone, Copy)]
pub struct TxConfig {
    /// Frequency in Hz
    pub frequency: u32,
    /// Output power in dBm
    pub power: i8,
    /// Modulation parameters
    pub modulation: ModulationParams,
}

/// Radio receive configuration
#[derive(Debug, Clone, Copy)]
pub struct RxConfig {
    /// Frequency in Hz
    pub frequency: u32,
    /// Timeout in milliseconds
    pub timeout_ms: u32,
    /// Modulation parameters
    pub modulation: ModulationParams,
}

/// Radio trait for LoRaWAN devices
pub trait Radio {
    /// Error type returned by radio operations
    type Error;

    /// Initialize the radio
    fn init(&mut self) -> Result<(), Self::Error>;

    /// Set the radio frequency
    fn set_frequency(&mut self, freq: u32) -> Result<(), Self::Error>;

    /// Set the radio output power
    fn set_tx_power(&mut self, power: i8) -> Result<(), Self::Error>;

    /// Transmit data
    fn transmit(&mut self, data: &[u8]) -> Result<(), Self::Error>;

    /// Receive data
    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;

    /// Configure radio for transmission
    fn configure_tx(&mut self, config: TxConfig) -> Result<(), Self::Error>;

    /// Configure radio for reception
    fn configure_rx(&mut self, config: RxConfig) -> Result<(), Self::Error>;

    /// Get RSSI value
    fn get_rssi(&mut self) -> Result<i16, Self::Error>;

    /// Get SNR value
    fn get_snr(&mut self) -> Result<i8, Self::Error>;

    /// Check if radio is currently transmitting
    fn is_transmitting(&mut self) -> Result<bool, Self::Error>;

    /// Set RX gain
    fn set_rx_gain(&mut self, gain: u8) -> Result<(), Self::Error>;

    /// Set low power mode
    fn set_low_power_mode(&mut self, enabled: bool) -> Result<(), Self::Error>;

    /// Put radio in sleep mode
    fn sleep(&mut self) -> Result<(), Self::Error>;
}
