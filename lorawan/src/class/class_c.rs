use core::time::Duration;

use crate::lorawan::{
    mac::{MacError, MacLayer},
    region::Region,
};
use crate::radio::Radio;
use super::{ClassCState, DeviceClass, OperatingMode};

/// Class C state machine states
#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    /// Continuous receive on RX2
    Receiving,
    /// Transmitting data
    Transmitting,
    /// Processing RX1 window after transmission
    InRX1Window,
}

/// Class C device implementation
pub struct ClassC<R: Radio, REG: Region> {
    /// MAC layer
    mac: MacLayer<R, REG>,
    /// Current state
    state: State,
    /// Class C specific state
    class_c: ClassCState,
    /// Last uplink timestamp
    last_uplink: Option<Duration>,
    /// RX1 window duration in milliseconds
    rx1_window_duration: u32,
    /// Current time (would be provided by timer in real implementation)
    current_time: Duration,
}

impl<R: Radio, REG: Region> ClassC<R, REG> {
    /// Create new Class C device
    pub fn new(mac: MacLayer<R, REG>, rx2_frequency: u32, rx2_data_rate: u8) -> Self {
        Self {
            mac,
            state: State::Receiving,
            class_c: ClassCState::new(rx2_frequency, rx2_data_rate),
            last_uplink: None,
            rx1_window_duration: 500, // Default RX1 window is 500ms
            current_time: Duration::from_secs(0),
        }
    }

    /// Set RX1 window duration
    pub fn set_rx1_window_duration(&mut self, duration: u32) {
        self.rx1_window_duration = duration;
    }

    /// Configure radio for continuous RX2
    fn configure_rx2(&mut self) -> Result<(), MacError<R::Error>> {
        // TODO: Configure radio for RX2 with class_c parameters
        Ok(())
    }

    /// Process RX windows
    fn process_rx_windows(&mut self) -> Result<(), MacError<R::Error>> {
        match self.state {
            State::Transmitting => {
                // After transmission, switch to RX1 window
                self.state = State::InRX1Window;
                self.last_uplink = Some(self.current_time);
                // TODO: Configure radio for RX1
            }
            State::InRX1Window => {
                if let Some(uplink_time) = self.last_uplink {
                    let elapsed = self.current_time - uplink_time;
                    if elapsed >= Duration::from_millis(self.rx1_window_duration as u64) {
                        // RX1 window expired, switch back to continuous RX2
                        self.state = State::Receiving;
                        self.configure_rx2()?;
                    }
                }
            }
            State::Receiving => {
                // Stay in continuous receive mode
            }
        }
        Ok(())
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

        // Try to receive data
        match self.state {
            State::Receiving | State::InRX1Window => {
                let mut buffer = [0u8; 256];
                if let Ok(len) = self.mac.receive(&mut buffer) {
                    // Process received data
                    // TODO: Handle received data
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn send_data(&mut self, port: u8, data: &[u8], confirmed: bool) -> Result<(), Self::Error> {
        // Switch to transmit mode
        self.state = State::Transmitting;

        // Send data using MAC layer
        if confirmed {
            self.mac.send_confirmed(port, data)?;
        } else {
            self.mac.send_unconfirmed(port, data)?;
        }

        Ok(())
    }

    fn send_join_request(&mut self, dev_eui: [u8; 8], app_eui: [u8; 8], app_key: [u8; 16]) -> Result<(), Self::Error> {
        // Switch to transmit mode
        self.state = State::Transmitting;

        // Send join request using MAC layer
        self.mac.join_request(dev_eui, app_eui, app_key)?;

        // After join request, switch to RX1 window
        self.state = State::InRX1Window;
        self.last_uplink = Some(self.current_time);

        Ok(())
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        // Only receive if in receive mode or RX1 window
        if self.state != State::Receiving && self.state != State::InRX1Window {
            return Ok(0);
        }

        // Receive using MAC layer
        self.mac.receive(buffer)
    }
} 