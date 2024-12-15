use super::region::{Channel, DataRate, Region};
use crate::radio::traits::{ModulationParams, Radio, RxConfig, TxConfig};

/// PHY layer timing parameters
#[derive(Debug, Clone, Copy)]
pub struct TimingParams {
    /// RX1 delay in seconds
    pub rx1_delay: u32,
    /// RX2 delay in seconds
    pub rx2_delay: u32,
    /// Join accept delay 1 in seconds
    pub join_accept_delay1: u32,
    /// Join accept delay 2 in seconds
    pub join_accept_delay2: u32,
}

impl Default for TimingParams {
    fn default() -> Self {
        Self {
            rx1_delay: 1,
            rx2_delay: 2,
            join_accept_delay1: 5,
            join_accept_delay2: 6,
        }
    }
}

/// PHY layer configuration
#[derive(Debug, Clone)]
pub struct PhyConfig {
    /// Timing parameters
    pub timing: TimingParams,
}

impl Default for PhyConfig {
    fn default() -> Self {
        Self {
            timing: TimingParams::default(),
        }
    }
}

/// PHY layer
pub struct PhyLayer<R: Radio> {
    /// Radio driver
    pub radio: R,
    /// Configuration
    pub config: PhyConfig,
}

impl<R: Radio> PhyLayer<R> {
    /// Create new PHY layer
    pub fn new(radio: R) -> Self {
        Self {
            radio,
            config: PhyConfig::default(),
        }
    }

    /// Initialize radio
    pub fn init(&mut self) -> Result<(), R::Error> {
        self.radio.init()
    }

    /// Configure radio for transmission
    pub fn configure_tx<REG: Region>(
        &mut self,
        channel: &Channel,
        data_rate: DataRate,
    ) -> Result<(), R::Error> {
        let config = TxConfig {
            frequency: channel.frequency,
            power: 14, // Default to 14 dBm
            modulation: ModulationParams {
                spreading_factor: data_rate.spreading_factor(),
                bandwidth: data_rate.bandwidth(),
                coding_rate: 5,
            },
        };
        self.radio.configure_tx(config)
    }

    /// Configure radio for reception
    pub fn configure_rx<REG: Region>(
        &mut self,
        frequency: u32,
        data_rate: DataRate,
        timeout_ms: u32,
    ) -> Result<(), R::Error> {
        let config = RxConfig {
            frequency,
            modulation: ModulationParams {
                spreading_factor: data_rate.spreading_factor(),
                bandwidth: data_rate.bandwidth(),
                coding_rate: 5,
            },
            timeout_ms,
        };
        self.radio.configure_rx(config)
    }

    /// Transmit data
    pub fn transmit(&mut self, data: &[u8]) -> Result<(), R::Error> {
        self.radio.transmit(data)
    }

    /// Receive data
    pub fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, R::Error> {
        self.radio.receive(buffer)
    }

    /// Get RSSI
    pub fn get_rssi(&mut self) -> Result<i16, R::Error> {
        self.radio.get_rssi()
    }

    /// Get SNR
    pub fn get_snr(&mut self) -> Result<i8, R::Error> {
        self.radio.get_snr()
    }

    /// Check if transmitting
    pub fn is_transmitting(&mut self) -> Result<bool, R::Error> {
        self.radio.is_transmitting()
    }
}
