use core::time::Duration;

use crate::lorawan::{
    mac::{MacError, MacLayer},
    region::Region,
};
use crate::radio::Radio;
use super::{ClassCState, DeviceClass, OperatingMode};

/// RX2 window configuration
#[derive(Debug, Clone)]
pub struct RX2Config {
    /// Frequency for RX2 window
    pub frequency: u32,
    /// Data rate for RX2 window
    pub data_rate: u8,
    /// Whether continuous reception is enabled
    pub continuous: bool,
}

impl RX2Config {
    /// Create new RX2 configuration
    pub fn new(frequency: u32, data_rate: u8) -> Self {
        Self {
            frequency,
            data_rate,
            continuous: false,
        }
    }

    /// Enable continuous reception
    pub fn enable_continuous(&mut self) {
        self.continuous = true;
    }

    /// Disable continuous reception
    pub fn disable_continuous(&mut self) {
        self.continuous = false;
    }
}

/// Power management states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerState {
    /// Full power - continuous reception
    Active,
    /// Low power - periodic wake-up
    Sleep,
    /// Minimal power - only wake on external trigger
    DeepSleep,
}

/// Class C device implementation
pub struct ClassC<R: Radio, REG: Region> {
    /// MAC layer
    mac: MacLayer<R, REG>,
    /// RX2 configuration
    rx2_config: RX2Config,
    /// Current power state
    power_state: PowerState,
    /// Last uplink timestamp
    last_uplink: Option<Duration>,
    /// RX1 window duration in milliseconds
    rx1_window_duration: u32,
    /// Current time (would be provided by timer in real implementation)
    current_time: Duration,
    /// Battery level (0-255, 0 = unknown, 1 = min, 254 = max, 255 = external power)
    battery_level: u8,
}

impl<R: Radio, REG: Region> ClassC<R, REG> {
    /// Create new Class C device
    pub fn new(mac: MacLayer<R, REG>, rx2_frequency: u32, rx2_data_rate: u8) -> Self {
        Self {
            mac,
            rx2_config: RX2Config::new(rx2_frequency, rx2_data_rate),
            power_state: PowerState::Active,
            last_uplink: None,
            rx1_window_duration: 500, // Default RX1 window is 500ms
            current_time: Duration::from_secs(0),
            battery_level: 0,
        }
    }

    /// Configure radio for continuous reception
    fn configure_continuous_rx(&mut self) -> Result<(), MacError<R::Error>> {
        // Configure radio for RX2 parameters
        self.mac.set_rx_config(
            self.rx2_config.frequency,
            self.rx2_config.data_rate,
            true, // Continuous mode
        )?;

        // Enable continuous reception
        self.rx2_config.enable_continuous();
        
        Ok(())
    }

    /// Configure radio for RX1
    fn configure_rx1(&mut self) -> Result<(), MacError<R::Error>> {
        if let Some(last_uplink) = self.last_uplink {
            let elapsed = self.current_time - last_uplink;
            
            // Get RX1 parameters from region config
            let rx1_params = self.mac.get_rx1_params()?;
            
            // Configure radio for RX1
            self.mac.set_rx_config(
                rx1_params.frequency,
                rx1_params.data_rate,
                false, // Not continuous
            )?;
        }
        Ok(())
    }

    /// Configure sleep mode
    fn configure_sleep(&mut self) -> Result<(), MacError<R::Error>> {
        // Disable continuous reception
        self.rx2_config.disable_continuous();

        // Configure radio for periodic wake-up
        let radio = self.mac.get_radio_mut();
        radio.set_low_power_mode(true)?;

        // Set wake-up interval (e.g., every 1 second)
        let wake_interval = Duration::from_secs(1);
        // TODO: Configure timer for periodic wake-up

        Ok(())
    }

    /// Configure deep sleep
    fn configure_deep_sleep(&mut self) -> Result<(), MacError<R::Error>> {
        // Disable all reception
        self.rx2_config.disable_continuous();

        // Put radio in lowest power mode
        let radio = self.mac.get_radio_mut();
        radio.sleep()?;

        // Disable all timers
        // TODO: Disable wake-up timer

        Ok(())
    }

    /// Process received data
    fn handle_received_data(&mut self, data: &[u8]) -> Result<(), MacError<R::Error>> {
        // Verify MIC and decrypt payload
        if let Ok(payload) = self.mac.decrypt_payload(data) {
            // Extract port number
            let f_port = payload[0];
            
            // Handle MAC commands if present
            if f_port == 0 {
                if let Some(commands) = self.mac.extract_mac_commands(&payload[1..]) {
                    for cmd in commands {
                        self.mac.process_mac_command(cmd)?;
                    }
                }
            } else {
                // Process application data
                // Forward to application layer
                // TODO: Implement callback mechanism
            }

            // Update frame counter
            self.mac.increment_frame_counter_down();
        }
        Ok(())
    }

    /// Process RX windows
    fn process_rx_windows(&mut self) -> Result<(), MacError<R::Error>> {
        if let Some(last_uplink) = self.last_uplink {
            let elapsed = self.current_time - last_uplink;

            // Check if in RX1 window
            if elapsed.as_millis() <= self.rx1_window_duration as u128 {
                // Configure radio for RX1
                self.configure_rx1()?;

                // Try to receive in RX1
                let mut buffer = [0u8; 256];
                if let Ok(len) = self.mac.receive(&mut buffer) {
                    self.handle_received_data(&buffer[..len])?;
                    return Ok(());
                }
            }
        }

        // Default to continuous RX2
        match self.power_state {
            PowerState::Active => {
                self.configure_continuous_rx()?;
                
                // Monitor RSSI in continuous mode
                let radio = self.mac.get_radio();
                if let Ok(rssi) = radio.get_rssi_continuous() {
                    // Adjust gain if needed
                    if rssi < -120 {
                        radio.set_rx_gain(0)?; // Max gain
                    } else if rssi > -60 {
                        radio.set_rx_gain(6)?; // Reduced gain
                    }
                }
            }
            PowerState::Sleep => {
                self.configure_sleep()?;
            }
            PowerState::DeepSleep => {
                self.configure_deep_sleep()?;
            }
        }
        
        Ok(())
    }

    /// Set power state
    pub fn set_power_state(&mut self, state: PowerState) -> Result<(), MacError<R::Error>> {
        match state {
            PowerState::Active => {
                // Enable continuous reception
                self.configure_continuous_rx()?;
            }
            PowerState::Sleep => {
                // Disable continuous reception but allow periodic wake-up
                self.rx2_config.disable_continuous();
                // TODO: Configure sleep parameters
            }
            PowerState::DeepSleep => {
                // Disable all reception until external wake-up
                self.rx2_config.disable_continuous();
                // TODO: Configure deep sleep
            }
        }

        self.power_state = state;
        Ok(())
    }

    /// Update battery level
    pub fn update_battery_level(&mut self, level: u8) {
        self.battery_level = level;
    }

    /// Get current battery level
    pub fn get_battery_level(&self) -> u8 {
        self.battery_level
    }

    /// Update timing
    pub fn update_time(&mut self, time: Duration) {
        self.current_time = time;
    }
}

impl<R: Radio, REG: Region> DeviceClass for ClassC<R, REG> {
    type Error = MacError<R::Error>;

    fn operating_mode(&self) -> OperatingMode {
        OperatingMode::ClassC
    }

    fn process(&mut self) -> Result<(), Self::Error> {
        // Process RX windows
        self.process_rx_windows()?;

        // Try to receive data if in active state
        if self.power_state == PowerState::Active {
            let mut buffer = [0u8; 256];
            if let Ok(len) = self.mac.receive(&mut buffer) {
                self.handle_received_data(&buffer[..len])?;
            }
        }

        Ok(())
    }

    fn send_data(&mut self, port: u8, data: &[u8], confirmed: bool) -> Result<(), Self::Error> {
        // Disable continuous reception during transmission
        self.rx2_config.disable_continuous();

        // Send data using MAC layer
        if confirmed {
            self.mac.send_confirmed(port, data)?;
        } else {
            self.mac.send_unconfirmed(port, data)?;
        }

        // Record transmission time
        self.last_uplink = Some(self.current_time);

        // Re-enable continuous reception
        self.configure_continuous_rx()?;

        Ok(())
    }

    fn send_join_request(&mut self, dev_eui: [u8; 8], app_eui: [u8; 8], app_key: [u8; 16]) -> Result<(), Self::Error> {
        // Disable continuous reception during join
        self.rx2_config.disable_continuous();

        // Send join request using MAC layer
        self.mac.join_request(dev_eui, app_eui, app_key)?;

        // Re-enable continuous reception after join
        self.configure_continuous_rx()?;

        Ok(())
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        // Only receive if in active state
        if self.power_state != PowerState::Active {
            return Ok(0);
        }

        // Receive using MAC layer
        self.mac.receive(buffer)
    }
} 