use embedded_hal::blocking::spi;
use embedded_hal::digital::v2::OutputPin;

/// Radio modulation parameters
#[derive(Debug, Clone, Copy)]
pub struct ModulationParams {
    /// Spreading factor (SF7-SF12)
    pub spreading_factor: u8,
    /// Bandwidth in Hz
    pub bandwidth: u32,
    /// Coding rate (4/5, 4/6, 4/7, 4/8)
    pub coding_rate: u8,
}

/// Radio transmission parameters
#[derive(Debug, Clone, Copy)]
pub struct TxConfig {
    /// Transmission power in dBm
    pub power: i8,
    /// Frequency in Hz
    pub frequency: u32,
    /// Modulation parameters
    pub modulation: ModulationParams,
}

/// Radio receive parameters
#[derive(Debug, Clone, Copy)]
pub struct RxConfig {
    /// Frequency in Hz
    pub frequency: u32,
    /// Modulation parameters
    pub modulation: ModulationParams,
    /// Receive timeout in milliseconds
    pub timeout_ms: u32,
}

/// Generic radio interface trait
pub trait Radio {
    /// Error type for radio operations
    type Error;

    /// Initialize the radio
    fn init(&mut self) -> Result<(), Self::Error>;

    /// Set the radio frequency
    fn set_frequency(&mut self, freq: u32) -> Result<(), Self::Error>;

    /// Set the radio's transmission power
    fn set_tx_power(&mut self, power: i8) -> Result<(), Self::Error>;

    /// Transmit data
    fn transmit(&mut self, buffer: &[u8]) -> Result<(), Self::Error>;

    /// Receive data into the provided buffer
    /// Returns the number of bytes received
    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;

    /// Configure the radio for transmission
    fn configure_tx(&mut self, config: TxConfig) -> Result<(), Self::Error>;

    /// Configure the radio for reception
    fn configure_rx(&mut self, config: RxConfig) -> Result<(), Self::Error>;

    /// Check if the radio is currently receiving a packet
    fn is_receiving(&mut self) -> Result<bool, Self::Error>;

    /// Get the last packet's RSSI (Received Signal Strength Indicator)
    fn get_rssi(&mut self) -> Result<i16, Self::Error>;

    /// Get the last packet's SNR (Signal to Noise Ratio)
    fn get_snr(&mut self) -> Result<i8, Self::Error>;

    /// Put the radio into sleep mode
    fn sleep(&mut self) -> Result<(), Self::Error>;

    /// Put the radio into standby mode
    fn standby(&mut self) -> Result<(), Self::Error>;

    /// Check if the radio is currently transmitting
    fn is_transmitting(&mut self) -> Result<bool, Self::Error>;
} 