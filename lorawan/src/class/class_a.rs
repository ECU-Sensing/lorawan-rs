use core::time::Duration;

use crate::lorawan::{
    mac::{MacError, MacLayer},
    region::Region,
};
use crate::radio::Radio;
use super::{DeviceClass, OperatingMode};

/// Class A state machine states
#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    /// Idle, waiting for uplink
    Idle,
    /// Waiting for RX1 window
    WaitingRx1,
    /// In RX1 window
    InRx1,
    /// Waiting for RX2 window
    WaitingRx2,
    /// In RX2 window
    InRx2,
}

/// Class A device implementation
pub struct ClassA<R: Radio, REG: Region> {
    /// MAC layer
    mac: MacLayer<R, REG>,
    /// Current state
    state: State,
    /// Last uplink timestamp
    last_uplink: Option<Duration>,
    /// RX1 delay in seconds
    rx1_delay: u32,
    /// RX2 delay in seconds
    rx2_delay: u32,
    /// RX window duration in milliseconds
    rx_window_duration: u32,
}

impl<R: Radio, REG: Region> ClassA<R, REG> {
    /// Create new Class A device
    pub fn new(mac: MacLayer<R, REG>) -> Self {
        Self {
            mac,
            state: State::Idle,
            last_uplink: None,
            rx1_delay: 1,  // Default RX1 delay is 1 second
            rx2_delay: 2,  // Default RX2 delay is 2 seconds
            rx_window_duration: 500, // Default RX window is 500ms
        }
    }

    /// Set RX1 delay
    pub fn set_rx1_delay(&mut self, delay: u32) {
        self.rx1_delay = delay;
    }

    /// Set RX2 delay
    pub fn set_rx2_delay(&mut self, delay: u32) {
        self.rx2_delay = delay;
    }

    /// Set RX window duration
    pub fn set_rx_window_duration(&mut self, duration: u32) {
        self.rx_window_duration = duration;
    }

    /// Process RX windows
    fn process_rx_windows(&mut self, current_time: Duration) -> Result<(), MacError<R::Error>> {
        match (self.state, self.last_uplink) {
            (State::WaitingRx1, Some(uplink_time)) => {
                let elapsed = current_time - uplink_time;
                if elapsed >= Duration::from_secs(self.rx1_delay as u64) {
                    // Open RX1 window
                    self.state = State::InRx1;
                    // Configure radio for RX1
                    // TODO: Configure radio with proper RX1 parameters
                }
            }
            (State::InRx1, Some(uplink_time)) => {
                let elapsed = current_time - uplink_time;
                if elapsed >= Duration::from_secs(self.rx1_delay as u64) + Duration::from_millis(self.rx_window_duration as u64) {
                    // RX1 window expired
                    self.state = State::WaitingRx2;
                }
            }
            (State::WaitingRx2, Some(uplink_time)) => {
                let elapsed = current_time - uplink_time;
                if elapsed >= Duration::from_secs(self.rx2_delay as u64) {
                    // Open RX2 window
                    self.state = State::InRx2;
                    // Configure radio for RX2
                    // TODO: Configure radio with proper RX2 parameters
                }
            }
            (State::InRx2, Some(uplink_time)) => {
                let elapsed = current_time - uplink_time;
                if elapsed >= Duration::from_secs(self.rx2_delay as u64) + Duration::from_millis(self.rx_window_duration as u64) {
                    // RX2 window expired
                    self.state = State::Idle;
                    self.last_uplink = None;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl<R: Radio, REG: Region> DeviceClass for ClassA<R, REG> {
    type Error = MacError<R::Error>;

    fn operating_mode(&self) -> OperatingMode {
        OperatingMode::ClassA
    }

    fn process(&mut self) -> Result<(), Self::Error> {
        // Get current time (in real implementation, this would come from a timer)
        let current_time = Duration::from_secs(0); // TODO: Get actual time

        // Process RX windows
        self.process_rx_windows(current_time)?;

        // Try to receive data if in RX window
        if self.state == State::InRx1 || self.state == State::InRx2 {
            let mut buffer = [0u8; 256];
            if let Ok(len) = self.mac.receive(&mut buffer) {
                // Data received, process it
                // TODO: Handle received data
            }
        }

        Ok(())
    }

    fn send_data(&mut self, port: u8, data: &[u8], confirmed: bool) -> Result<(), Self::Error> {
        // Send data using MAC layer
        if confirmed {
            self.mac.send_confirmed(port, data)?;
        } else {
            self.mac.send_unconfirmed(port, data)?;
        }

        // Update state for RX windows
        self.state = State::WaitingRx1;
        // Get current time (in real implementation, this would come from a timer)
        self.last_uplink = Some(Duration::from_secs(0)); // TODO: Get actual time

        Ok(())
    }

    fn send_join_request(&mut self, dev_eui: [u8; 8], app_eui: [u8; 8], app_key: [u8; 16]) -> Result<(), Self::Error> {
        // Send join request using MAC layer
        self.mac.join_request(dev_eui, app_eui, app_key)?;

        // Update state for RX windows
        self.state = State::WaitingRx1;
        // Get current time (in real implementation, this would come from a timer)
        self.last_uplink = Some(Duration::from_secs(0)); // TODO: Get actual time

        Ok(())
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        // Only receive if in RX window
        if self.state != State::InRx1 && self.state != State::InRx2 {
            return Ok(0);
        }

        // Receive using MAC layer
        self.mac.receive(buffer)
    }
} 