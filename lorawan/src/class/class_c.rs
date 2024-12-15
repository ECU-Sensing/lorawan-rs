//! LoRaWAN Class C device implementation
//!
//! Class C devices extend Class A by keeping the RX2 window open continuously
//! when not transmitting. This allows for minimal downlink latency at the cost
//! of increased power consumption.

use super::{DeviceClass, OperatingMode};
use crate::config::device::{AESKey, SessionState};
use crate::lorawan::mac::{MacError, MacLayer};
use crate::lorawan::region::{DataRate, Region};
use crate::radio::traits::Radio;
use core::fmt::Debug;

/// Battery level monitoring thresholds
const BATTERY_CRITICAL_THRESHOLD: u8 = 10;
const BATTERY_LOW_THRESHOLD: u8 = 30;

/// RX window states
#[derive(Debug, Clone, Copy, PartialEq)]
enum RxWindowState {
    /// RX1 window active
    Rx1Active,
    /// RX2 window active (continuous)
    Rx2Active,
    /// Temporarily suspended (e.g. during TX)
    Suspended,
}

/// Power management state
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerState {
    /// Battery level (0-255, 0=external power)
    battery_level: u8,
    /// Power saving mode enabled
    power_save: bool,
    /// Last RSSI reading
    last_rssi: i16,
    /// Last SNR reading
    last_snr: i8,
}

impl PowerState {
    fn new() -> Self {
        Self {
            battery_level: 255,
            power_save: false,
            last_rssi: 0,
            last_snr: 0,
        }
    }

    fn is_battery_critical(&self) -> bool {
        self.battery_level > 0 && self.battery_level <= BATTERY_CRITICAL_THRESHOLD
    }

    fn is_battery_low(&self) -> bool {
        self.battery_level > 0 && self.battery_level <= BATTERY_LOW_THRESHOLD
    }
}

/// Class C device implementation
pub struct ClassC<R, REG>
where
    R: Radio + Clone,
    REG: Region + Debug + Clone,
{
    /// MAC layer
    mac: MacLayer<R, REG>,
    /// RX2 frequency
    rx2_frequency: u32,
    /// RX2 data rate
    rx2_data_rate: u8,
    /// Current RX window state
    rx_state: RxWindowState,
    /// Power management state
    power_state: PowerState,
    /// Error recovery attempts
    recovery_attempts: u8,
}

impl<R, REG> ClassC<R, REG>
where
    R: Radio + Clone,
    REG: Region + Debug + Clone,
{
    /// Create new Class C device
    pub fn new(mac: MacLayer<R, REG>, rx2_frequency: u32, rx2_data_rate: u8) -> Self {
        Self {
            mac,
            rx2_frequency,
            rx2_data_rate,
            rx_state: RxWindowState::Rx2Active,
            power_state: PowerState::new(),
            recovery_attempts: 0,
        }
    }

    /// Configure RX2 window parameters
    pub fn configure_rx2(&mut self, frequency: u32, data_rate: u8) -> Result<(), MacError<R::Error>> {
        self.rx2_frequency = frequency;
        self.rx2_data_rate = data_rate;
        self.resume_rx2()
    }

    /// Start RX1 window
    fn start_rx1(&mut self, frequency: u32, data_rate: u8) -> Result<(), MacError<R::Error>> {
        self.rx_state = RxWindowState::Rx1Active;
        self.mac.set_rx_config(
            frequency,
            DataRate::from_index(data_rate),
            1000, // 1 second RX1 window
        )
    }

    /// Resume RX2 continuous reception
    fn resume_rx2(&mut self) -> Result<(), MacError<R::Error>> {
        // Only resume if not in power saving mode
        if !self.power_state.power_save {
            self.rx_state = RxWindowState::Rx2Active;
            self.mac.set_rx_config(
                self.rx2_frequency,
                DataRate::from_index(self.rx2_data_rate),
                0, // Continuous reception
            )?;
        }
        Ok(())
    }

    /// Suspend reception (e.g. for transmission)
    fn suspend_rx(&mut self) {
        self.rx_state = RxWindowState::Suspended;
    }

    /// Update power state
    pub fn update_power_state(&mut self, battery_level: u8) {
        self.power_state.battery_level = battery_level;
        
        // Enable power saving if battery is low
        if self.power_state.is_battery_low() {
            self.power_state.power_save = true;
        }
    }

    /// Update signal quality metrics
    fn update_signal_metrics(&mut self) -> Result<(), MacError<R::Error>> {
        self.power_state.last_rssi = self.mac.get_radio_mut().get_rssi()?;
        self.power_state.last_snr = self.mac.get_radio_mut().get_snr()?;
        Ok(())
    }

    /// Handle radio errors with automatic recovery
    fn handle_radio_error(&mut self, error: MacError<R::Error>) -> Result<(), MacError<R::Error>> {
        self.recovery_attempts += 1;
        
        if self.recovery_attempts > 3 {
            // Too many recovery attempts, return error
            self.recovery_attempts = 0;
            Err(error)
        } else {
            // Try to recover by resetting radio and resuming RX2
            self.mac.get_radio_mut().reset()?;
            self.resume_rx2()
        }
    }
}

impl<R, REG> DeviceClass<R, REG> for ClassC<R, REG>
where
    R: Radio + Clone,
    REG: Region + Debug + Clone,
{
    type Error = MacError<R::Error>;

    fn operating_mode(&self) -> OperatingMode {
        OperatingMode::ClassC
    }

    fn process(&mut self) -> Result<(), MacError<R::Error>> {
        // Update signal metrics periodically
        if let Err(e) = self.update_signal_metrics() {
            self.handle_radio_error(e)?;
        }

        // Process received data
        let mut buffer = [0u8; 256];
        match self.mac.receive(&mut buffer) {
            Ok(len) if len > 0 => {
                // Reset recovery counter on successful reception
                self.recovery_attempts = 0;

                // Process received data
                let payload = self.mac.decrypt_payload(&buffer[..len])?;
                
                // Handle MAC commands if present
                if let Some(port) = payload.first() {
                    if *port == 0 {
                        if let Some(commands) = self.mac.extract_mac_commands(&payload[1..]) {
                            for command in commands {
                                self.mac.process_mac_command(command)?;
                            }
                        }
                    }
                }

                // Update frame counter
                self.mac.increment_frame_counter_down();
            }
            Err(e) => {
                self.handle_radio_error(e)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn send_data(&mut self, port: u8, data: &[u8], confirmed: bool) -> Result<(), MacError<R::Error>> {
        // Suspend RX2 during transmission
        self.suspend_rx();

        // Send data
        let result = if confirmed {
            self.mac.send_confirmed(port, data)
        } else {
            self.mac.send_unconfirmed(port, data)
        };

        // Resume RX2 after transmission
        self.resume_rx2()?;

        result
    }

    fn send_join_request(
        &mut self,
        dev_eui: [u8; 8],
        app_eui: [u8; 8],
        app_key: AESKey,
    ) -> Result<(), MacError<R::Error>> {
        // Suspend RX2 during join
        self.suspend_rx();

        // Send join request
        let result = self.mac.join_request(dev_eui, app_eui, app_key);

        // Resume RX2 after join
        self.resume_rx2()?;

        result
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, MacError<R::Error>> {
        match self.rx_state {
            RxWindowState::Suspended => Ok(0),
            _ => self.mac.receive(buffer),
        }
    }

    fn get_session_state(&self) -> SessionState {
        self.mac.get_session_state().clone()
    }

    fn get_mac_layer(&self) -> &MacLayer<R, REG> {
        &self.mac
    }
}
