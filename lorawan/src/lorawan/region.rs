use core::any::Any;
use core::fmt::Debug;
use heapless::Vec;

/// Maximum number of channels
pub const MAX_CHANNELS: usize = 72;

/// Channel configuration
#[derive(Debug, Clone)]
pub struct Channel {
    /// Frequency in Hz
    pub frequency: u32,
    /// Minimum data rate
    pub min_dr: DataRate,
    /// Maximum data rate
    pub max_dr: DataRate,
    /// Channel enabled
    pub enabled: bool,
}

/// Data rate configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataRate {
    /// SF12/125kHz
    SF12BW125,
    /// SF11/125kHz
    SF11BW125,
    /// SF10/125kHz
    SF10BW125,
    /// SF9/125kHz
    SF9BW125,
    /// SF8/125kHz
    SF8BW125,
    /// SF7/125kHz
    SF7BW125,
    /// SF8/500kHz
    SF8BW500,
}

impl DataRate {
    /// Convert from data rate index to DataRate
    pub fn from_index(index: u8) -> Self {
        match index {
            0 => DataRate::SF12BW125,
            1 => DataRate::SF11BW125,
            2 => DataRate::SF10BW125,
            3 => DataRate::SF9BW125,
            4 => DataRate::SF8BW125,
            5 => DataRate::SF7BW125,
            6 => DataRate::SF8BW500,
            _ => DataRate::SF12BW125, // Default to slowest rate for invalid index
        }
    }

    /// Get spreading factor
    pub fn spreading_factor(&self) -> u8 {
        match self {
            DataRate::SF12BW125 => 12,
            DataRate::SF11BW125 => 11,
            DataRate::SF10BW125 => 10,
            DataRate::SF9BW125 => 9,
            DataRate::SF8BW125 | DataRate::SF8BW500 => 8,
            DataRate::SF7BW125 => 7,
        }
    }

    /// Get bandwidth in Hz
    pub fn bandwidth(&self) -> u32 {
        match self {
            DataRate::SF8BW500 => 500_000,
            _ => 125_000,
        }
    }
}

/// LoRaWAN region trait
pub trait Region: Any + Debug + Clone {
    /// Get region name
    fn name(&self) -> &'static str;

    /// Get number of channels
    fn channels(&self) -> usize;

    /// Get maximum number of channels
    fn get_max_channels(&self) -> usize;

    /// Check if frequency is valid for this region
    fn is_valid_frequency(&self, frequency: u32) -> bool;

    /// Get minimum frequency
    fn min_frequency(&self) -> u32;

    /// Get maximum frequency
    fn max_frequency(&self) -> u32;

    /// Get RX2 frequency
    fn rx2_frequency(&self) -> u32;

    /// Get RX2 data rate
    fn rx2_data_rate(&self) -> u8;

    /// Get maximum payload size for data rate
    fn max_payload_size(&self, data_rate: u8) -> u8;

    /// Get receive delay 1
    fn receive_delay1(&self) -> u32;

    /// Get receive delay 2
    fn receive_delay2(&self) -> u32;

    /// Get join accept delay 1
    fn join_accept_delay1(&self) -> u32;

    /// Get join accept delay 2
    fn join_accept_delay2(&self) -> u32;

    /// Get enabled channels
    fn enabled_channels(&self) -> impl Iterator<Item = &Channel>;

    /// Get next channel for transmission
    fn get_next_channel(&mut self) -> Option<Channel>;

    /// Get RX1 window parameters
    fn rx1_window(&self, tx_channel: &Channel) -> (u32, DataRate);

    /// Get RX2 window parameters
    fn rx2_window(&self) -> (u32, DataRate);

    /// Get beacon channels
    fn get_beacon_channels(&self) -> Vec<Channel, 8>;

    /// Get next beacon channel
    fn get_next_beacon_channel(&mut self) -> Option<Channel>;

    /// Convert to Any
    fn as_any(&self) -> &dyn Any;

    /// Convert to Any mut
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// US915 region implementation
#[derive(Debug, Clone)]
pub struct US915 {
    channels: Vec<Channel, MAX_CHANNELS>,
    data_rate: DataRate,
    sub_band: u8,
    last_channel: usize,
}

impl US915 {
    /// Create new US915 region
    pub fn new() -> Self {
        let mut channels = Vec::new();

        // Initialize 64 125 kHz upstream channels
        for i in 0..64 {
            let freq = 902_300_000 + (i as u32 * 200_000);
            channels
                .push(Channel {
                    frequency: freq,
                    min_dr: DataRate::SF10BW125,
                    max_dr: DataRate::SF7BW125,
                    enabled: true,
                })
                .unwrap();
        }

        // Initialize 8 500 kHz upstream channels
        for i in 0..8 {
            let freq = 903_000_000 + (i as u32 * 1_600_000);
            channels
                .push(Channel {
                    frequency: freq,
                    min_dr: DataRate::SF8BW500,
                    max_dr: DataRate::SF8BW500,
                    enabled: true,
                })
                .unwrap();
        }

        Self {
            channels,
            data_rate: DataRate::SF10BW125,
            sub_band: 0,
            last_channel: 0,
        }
    }

    /// Get current data rate
    pub fn get_data_rate(&self) -> DataRate {
        self.data_rate
    }

    /// Get enabled channels
    pub fn get_enabled_channels(&self) -> Vec<Channel, MAX_CHANNELS> {
        self.enabled_channels().map(|c| c.clone()).collect()
    }

    /// Set the sub-band (0-7)
    pub fn set_sub_band(&mut self, sub_band: u8) {
        self.sub_band = sub_band.min(7);

        // Enable only channels in the selected sub-band
        for (i, channel) in self.channels.iter_mut().enumerate() {
            let channel_sub_band = (i / 8) as u8;
            channel.enabled = channel_sub_band == self.sub_band;
        }
    }

    /// Configure for TTN US915
    pub fn configure_ttn_us915(&mut self) {
        // TTN US915 uses sub-band 2 (channels 8-15 and 65)
        self.set_sub_band(1); // 0-based index for sub-band 2

        // Enable only the 8 125 kHz channels and 1 500 kHz channel
        for (i, channel) in self.channels.iter_mut().enumerate() {
            channel.enabled = (i >= 8 && i < 16) || i == 65;
        }
    }
}

impl Region for US915 {
    fn name(&self) -> &'static str {
        "US915"
    }

    fn channels(&self) -> usize {
        self.channels.len()
    }

    fn get_max_channels(&self) -> usize {
        MAX_CHANNELS
    }

    fn is_valid_frequency(&self, frequency: u32) -> bool {
        frequency >= self.min_frequency() && frequency <= self.max_frequency()
    }

    fn min_frequency(&self) -> u32 {
        902_000_000
    }

    fn max_frequency(&self) -> u32 {
        928_000_000
    }

    fn rx2_frequency(&self) -> u32 {
        923_300_000
    }

    fn rx2_data_rate(&self) -> u8 {
        8 // DR8 (SF12/500kHz)
    }

    fn max_payload_size(&self, data_rate: u8) -> u8 {
        match data_rate {
            0 => 19,  // SF10/125kHz
            1 => 61,  // SF9/125kHz
            2 => 133, // SF8/125kHz
            3 => 250, // SF7/125kHz
            4 => 250, // SF8/500kHz
            _ => 0,   // Invalid data rate
        }
    }

    fn receive_delay1(&self) -> u32 {
        1_000 // 1 second
    }

    fn receive_delay2(&self) -> u32 {
        2_000 // 2 seconds
    }

    fn join_accept_delay1(&self) -> u32 {
        5_000 // 5 seconds
    }

    fn join_accept_delay2(&self) -> u32 {
        6_000 // 6 seconds
    }

    fn enabled_channels(&self) -> impl Iterator<Item = &Channel> {
        self.channels.iter().filter(|c| c.enabled)
    }

    fn get_next_channel(&mut self) -> Option<Channel> {
        let enabled_channels: Vec<Channel, MAX_CHANNELS> =
            self.enabled_channels().map(|c| c.clone()).collect();
        if enabled_channels.is_empty() {
            return None;
        }
        let next_channel = (self.last_channel + 1) % enabled_channels.len();
        let channel = enabled_channels[next_channel].clone();
        self.last_channel = next_channel;
        Some(channel)
    }

    fn rx1_window(&self, tx_channel: &Channel) -> (u32, DataRate) {
        // RX1 frequency is uplink frequency - 500MHz
        let frequency = tx_channel.frequency.saturating_sub(500_000_000);

        // RX1 data rate follows the data rate offset table
        // For US915, RX1DROffset is typically 0, meaning same DR as uplink
        let data_rate = self.data_rate;

        (frequency, data_rate)
    }

    fn rx2_window(&self) -> (u32, DataRate) {
        // RX2 uses fixed frequency and data rate
        (923_300_000, DataRate::SF12BW125)
    }

    fn get_beacon_channels(&self) -> Vec<Channel, 8> {
        let mut channels = Vec::new();
        // US915 beacon channels: 923.3 MHz + n * 600 kHz, n = 0..7
        for i in 0..8 {
            channels
                .push(Channel {
                    frequency: 923_300_000 + i * 600_000,
                    min_dr: DataRate::SF12BW125,
                    max_dr: DataRate::SF12BW125,
                    enabled: true,
                })
                .unwrap();
        }
        channels
    }

    fn get_next_beacon_channel(&mut self) -> Option<Channel> {
        let beacon_channels = self.get_beacon_channels();
        if beacon_channels.is_empty() {
            return None;
        }

        // Use a simple hash of the last channel as random source
        let index = (self.last_channel * 7919 + 17) % beacon_channels.len();
        self.last_channel = index;
        Some(beacon_channels[index].clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
