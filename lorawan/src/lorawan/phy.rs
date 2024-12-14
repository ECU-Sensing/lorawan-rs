use crate::radio::traits::{ModulationParams, Radio, RxConfig, TxConfig};
use super::region::{Channel, DataRate, Region};

/// PHY layer timing parameters
#[derive(Debug, Clone, Copy)]
pub struct TimingParams {
    /// Join accept delay 1 (in seconds)
    pub join_accept_delay1: u32,
    /// Join accept delay 2 (in seconds)
    pub join_accept_delay2: u32,
    /// RX1 delay (in seconds)
    pub rx1_delay: u32,
    /// RX2 delay (in seconds)
    pub rx2_delay: u32,
}

impl Default for TimingParams {
    fn default() -> Self {
        Self {
            join_accept_delay1: 5,
            join_accept_delay2: 6,
            rx1_delay: 1,
            rx2_delay: 2,
        }
    }
}

/// PHY layer configuration
#[derive(Debug)]
pub struct PhyConfig {
    /// Timing parameters
    pub timing: TimingParams,
    /// Maximum EIRP (dBm)
    pub max_eirp: i8,
    /// Antenna gain (dBi)
    pub antenna_gain: i8,
}

impl Default for PhyConfig {
    fn default() -> Self {
        Self {
            timing: TimingParams::default(),
            max_eirp: 30,    // Maximum EIRP for US915
            antenna_gain: 0, // Assume 0 dBi antenna gain by default
        }
    }
}

/// PHY layer state
pub struct PhyLayer<R: Radio> {
    /// Radio driver
    radio: R,
    /// PHY configuration
    config: PhyConfig,
}

impl<R: Radio> PhyLayer<R> {
    /// Create a new PHY layer
    pub fn new(radio: R, config: PhyConfig) -> Self {
        Self { radio, config }
    }

    /// Initialize the PHY layer
    pub fn init(&mut self) -> Result<(), R::Error> {
        self.radio.init()
    }

    /// Configure radio for transmission
    pub fn configure_tx<REG: Region>(
        &mut self,
        channel: &Channel,
        data_rate: DataRate,
    ) -> Result<(), R::Error> {
        // Calculate TX power considering EIRP limit and antenna gain
        let tx_power = self.config.max_eirp - self.config.antenna_gain;

        let config = TxConfig {
            frequency: channel.frequency,
            power: tx_power,
            modulation: ModulationParams {
                spreading_factor: data_rate.spreading_factor(),
                bandwidth: data_rate.bandwidth(),
                coding_rate: 5, // LoRaWAN uses 4/5 coding rate
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
            timeout_ms,
            modulation: ModulationParams {
                spreading_factor: data_rate.spreading_factor(),
                bandwidth: data_rate.bandwidth(),
                coding_rate: 5, // LoRaWAN uses 4/5 coding rate
            },
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

    /// Put radio to sleep
    pub fn sleep(&mut self) -> Result<(), R::Error> {
        self.radio.sleep()
    }

    /// Get RSSI of last received packet
    pub fn get_rssi(&mut self) -> Result<i16, R::Error> {
        self.radio.get_rssi()
    }

    /// Get SNR of last received packet
    pub fn get_snr(&mut self) -> Result<i8, R::Error> {
        self.radio.get_snr()
    }

    /// Configure RX1 window
    pub fn configure_rx1<REG: Region>(
        &mut self,
        region: &REG,
        uplink_channel: &Channel,
    ) -> Result<(), R::Error> {
        let (frequency, data_rate) = region.rx1_window(uplink_channel);
        self.configure_rx::<REG>(
            frequency,
            data_rate,
            self.config.timing.rx1_delay * 1000,
        )
    }

    /// Configure RX2 window
    pub fn configure_rx2<REG: Region>(
        &mut self,
        region: &REG,
    ) -> Result<(), R::Error> {
        let (frequency, data_rate) = region.rx2_window();
        self.configure_rx::<REG>(
            frequency,
            data_rate,
            self.config.timing.rx2_delay * 1000,
        )
    }

    /// Configure join accept RX1 window
    pub fn configure_join_rx1<REG: Region>(
        &mut self,
        region: &REG,
        uplink_channel: &Channel,
    ) -> Result<(), R::Error> {
        let (frequency, data_rate) = region.rx1_window(uplink_channel);
        self.configure_rx::<REG>(
            frequency,
            data_rate,
            self.config.timing.join_accept_delay1 * 1000,
        )
    }

    /// Configure join accept RX2 window
    pub fn configure_join_rx2<REG: Region>(
        &mut self,
        region: &REG,
    ) -> Result<(), R::Error> {
        let (frequency, data_rate) = region.rx2_window();
        self.configure_rx::<REG>(
            frequency,
            data_rate,
            self.config.timing.join_accept_delay2 * 1000,
        )
    }
} 