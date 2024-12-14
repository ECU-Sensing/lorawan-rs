use heapless::Vec;

/// Data rate identifier
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataRate {
    SF10BW125, // DR0
    SF9BW125,  // DR1
    SF8BW125,  // DR2
    SF7BW125,  // DR3
    SF8BW500,  // DR4
    RFU,       // DR5-7 Reserved for future use
}

impl DataRate {
    /// Get spreading factor
    pub fn spreading_factor(&self) -> u8 {
        match self {
            DataRate::SF10BW125 => 10,
            DataRate::SF9BW125 => 9,
            DataRate::SF8BW125 => 8,
            DataRate::SF7BW125 => 7,
            DataRate::SF8BW500 => 8,
            DataRate::RFU => 7,
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

/// US915 channel configuration
#[derive(Debug, Clone)]
pub struct Channel {
    /// Channel frequency in Hz
    pub frequency: u32,
    /// Minimum data rate
    pub min_dr: DataRate,
    /// Maximum data rate
    pub max_dr: DataRate,
    /// Whether the channel is enabled
    pub enabled: bool,
}

/// US915 region configuration
#[derive(Debug)]
pub struct US915 {
    /// Upstream channels (64 + 8 channels)
    channels: Vec<Channel, 72>,
    /// Current sub-band (0-7)
    sub_band: u8,
    /// Current data rate
    data_rate: DataRate,
}

impl Default for US915 {
    fn default() -> Self {
        let mut channels = Vec::new();
        
        // Initialize 64 125 kHz upstream channels
        for i in 0..64 {
            let freq = 902_300_000 + (i as u32 * 200_000);
            channels.push(Channel {
                frequency: freq,
                min_dr: DataRate::SF10BW125,
                max_dr: DataRate::SF7BW125,
                enabled: true,
            }).unwrap();
        }

        // Initialize 8 500 kHz upstream channels
        for i in 0..8 {
            let freq = 903_000_000 + (i as u32 * 1_600_000);
            channels.push(Channel {
                frequency: freq,
                min_dr: DataRate::SF8BW500,
                max_dr: DataRate::SF8BW500,
                enabled: true,
            }).unwrap();
        }

        Self {
            channels,
            sub_band: 0,
            data_rate: DataRate::SF10BW125,
        }
    }
}

impl US915 {
    /// Create a new US915 region configuration
    pub fn new() -> Self {
        Self::default()
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

    /// Set the data rate
    pub fn set_data_rate(&mut self, data_rate: DataRate) {
        self.data_rate = data_rate;
    }

    /// Get the current data rate
    pub fn data_rate(&self) -> DataRate {
        self.data_rate
    }

    /// Get enabled channels
    pub fn enabled_channels(&self) -> impl Iterator<Item = &Channel> {
        self.channels.iter().filter(|c| c.enabled)
    }

    /// Get RX1 window parameters
    pub fn rx1_window(&self, uplink_channel: &Channel) -> (u32, DataRate) {
        // RX1 frequency is uplink frequency - 500MHz
        let frequency = uplink_channel.frequency.saturating_sub(500_000_000);
        
        // RX1 data rate follows the data rate offset table
        // For US915, RX1DROffset is typically 0, meaning same DR as uplink
        let data_rate = self.data_rate;
        
        (frequency, data_rate)
    }

    /// Get RX2 window parameters
    pub fn rx2_window(&self) -> (u32, DataRate) {
        // RX2 uses a fixed frequency and data rate in US915
        (923_300_000, DataRate::SF9BW125)
    }

    /// Get join frequencies
    pub fn join_frequencies(&self) -> impl Iterator<Item = u32> + '_ {
        // For join requests, use 125 kHz channels
        self.channels[0..64]
            .iter()
            .filter(|c| c.enabled)
            .map(|c| c.frequency)
    }

    /// Get maximum payload size for current data rate
    pub fn max_payload_size(&self) -> usize {
        match self.data_rate {
            DataRate::SF10BW125 => 11,
            DataRate::SF9BW125 => 53,
            DataRate::SF8BW125 => 125,
            DataRate::SF7BW125 => 242,
            DataRate::SF8BW500 => 242,
            DataRate::RFU => 0,
        }
    }
}

/// Generic region trait
pub trait Region {
    /// Set the data rate
    fn set_data_rate(&mut self, data_rate: DataRate);
    
    /// Get the current data rate
    fn data_rate(&self) -> DataRate;
    
    /// Get enabled channels
    fn enabled_channels(&self) -> impl Iterator<Item = &Channel>;
    
    /// Get RX1 window parameters
    fn rx1_window(&self, uplink_channel: &Channel) -> (u32, DataRate);
    
    /// Get RX2 window parameters
    fn rx2_window(&self) -> (u32, DataRate);
    
    /// Get maximum payload size for current data rate
    fn max_payload_size(&self) -> usize;
}

impl Region for US915 {
    fn set_data_rate(&mut self, data_rate: DataRate) {
        self.set_data_rate(data_rate);
    }
    
    fn data_rate(&self) -> DataRate {
        self.data_rate()
    }
    
    fn enabled_channels(&self) -> impl Iterator<Item = &Channel> {
        self.enabled_channels()
    }
    
    fn rx1_window(&self, uplink_channel: &Channel) -> (u32, DataRate) {
        self.rx1_window(uplink_channel)
    }
    
    fn rx2_window(&self) -> (u32, DataRate) {
        self.rx2_window()
    }
    
    fn max_payload_size(&self) -> usize {
        self.max_payload_size()
    }
} 