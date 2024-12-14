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

/// Radio error type
pub trait RadioError: core::fmt::Debug {}

/// Common trait for LoRa radio drivers
pub trait Radio {
    /// Radio-specific error type
    type Error: RadioError;

    /// Initialize the radio
    fn init(&mut self) -> Result<(), Self::Error>;

    /// Set frequency in Hz
    fn set_frequency(&mut self, freq: u32) -> Result<(), Self::Error>;

    /// Set spreading factor (SF7-SF12)
    fn set_spreading_factor(&mut self, sf: u8) -> Result<(), Self::Error>;

    /// Set bandwidth in Hz
    fn set_bandwidth(&mut self, bw: u32) -> Result<(), Self::Error>;

    /// Set coding rate (4/5-4/8)
    fn set_coding_rate(&mut self, cr: u8) -> Result<(), Self::Error>;

    /// Set preamble length
    fn set_preamble_length(&mut self, len: u16) -> Result<(), Self::Error>;

    /// Set sync word
    fn set_sync_word(&mut self, word: u8) -> Result<(), Self::Error>;

    /// Set TX power in dBm
    fn set_tx_power(&mut self, power: i8) -> Result<(), Self::Error>;

    /// Get current RSSI in dBm
    fn get_rssi(&self) -> Result<i32, Self::Error>;

    /// Get last packet SNR in dB
    fn get_snr(&self) -> Result<i32, Self::Error>;

    /// Send packet
    fn send(&mut self, data: &[u8]) -> Result<(), Self::Error>;

    /// Receive packet with timeout
    fn receive(&mut self, buffer: &mut [u8], timeout_ms: u32) -> Result<usize, Self::Error>;

    /// Put radio in sleep mode
    fn sleep(&mut self) -> Result<(), Self::Error>;

    /// Put radio in standby mode
    fn standby(&mut self) -> Result<(), Self::Error>;
}

/// Extended radio capabilities for Class B/C support
pub trait RadioExt: Radio {
    /// Enable continuous reception mode
    fn set_continuous_reception(&mut self, enabled: bool) -> Result<(), Self::Error>;

    /// Get RSSI in continuous reception mode
    fn get_rssi_continuous(&self) -> Result<i32, Self::Error>;

    /// Set automatic gain control
    fn set_auto_gain_control(&mut self, enabled: bool) -> Result<(), Self::Error>;

    /// Set low data rate optimization
    fn set_low_data_rate_opt(&mut self, enabled: bool) -> Result<(), Self::Error>;

    /// Get current temperature in Celsius
    fn get_temperature(&self) -> Result<i32, Self::Error>;

    /// Set low power mode
    fn set_low_power_mode(&mut self, enabled: bool) -> Result<(), Self::Error>;

    /// Configure RX gain
    fn set_rx_gain(&mut self, gain: u8) -> Result<(), Self::Error>;

    /// Get current power consumption in mA
    fn get_current_consumption(&self) -> Result<u32, Self::Error>;
}

/// Radio configuration
#[derive(Debug, Clone)]
pub struct RadioConfig {
    /// Frequency in Hz
    pub frequency: u32,
    /// Spreading factor (SF7-SF12)
    pub spreading_factor: u8,
    /// Bandwidth in Hz
    pub bandwidth: u32,
    /// Coding rate (4/5-4/8)
    pub coding_rate: u8,
    /// Preamble length
    pub preamble_length: u16,
    /// Sync word
    pub sync_word: u8,
    /// TX power in dBm
    pub tx_power: i8,
    /// Low data rate optimization enabled
    pub low_data_rate_opt: bool,
    /// Automatic gain control enabled
    pub auto_gain_control: bool,
}

impl Default for RadioConfig {
    fn default() -> Self {
        Self {
            frequency: 915_000_000,  // 915 MHz
            spreading_factor: 7,      // SF7
            bandwidth: 125_000,       // 125 kHz
            coding_rate: 5,          // 4/5
            preamble_length: 8,
            sync_word: 0x34,         // LoRaWAN sync word
            tx_power: 14,            // 14 dBm
            low_data_rate_opt: false,
            auto_gain_control: true,
        }
    }
}

/// Radio operating mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RadioMode {
    /// Sleep mode (minimal power consumption)
    Sleep,
    /// Standby mode (ready for TX/RX)
    Standby,
    /// Transmit mode
    Transmit,
    /// Receive mode (with timeout)
    Receive,
    /// Continuous receive mode (Class C)
    ContinuousReceive,
}

/// Radio events
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RadioEvent {
    /// TX done
    TxDone,
    /// RX done
    RxDone,
    /// RX timeout
    RxTimeout,
    /// RX error
    RxError,
    /// Channel activity detection done
    CadDone,
    /// Channel activity detected
    CadDetected,
} 