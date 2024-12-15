//! LoRaWAN Class C device implementation
//!
//! Class C devices extend Class A by keeping the RX2 window open continuously
//! when not transmitting. This allows for minimal downlink latency at the cost
//! of increased power consumption.

use super::{DeviceClass, OperatingMode};
use crate::config::device::{AESKey, SessionState};
use crate::lorawan::mac::MacError;
use crate::lorawan::mac::MacLayer;
use crate::lorawan::region::{DataRate, Region};
use crate::radio::traits::Radio;
use core::fmt::Debug;

/// Class C device implementation
pub struct ClassC<R, REG>
where
    R: Radio,
    REG: Region + Debug + Clone,
{
    /// MAC layer
    mac: MacLayer<R, REG>,
    /// RX2 frequency
    rx2_frequency: u32,
    /// RX2 data rate
    rx2_data_rate: u8,
}

impl<R, REG> ClassC<R, REG>
where
    R: Radio,
    REG: Region + Debug + Clone,
{
    /// Create new Class C device
    pub fn new(mac: MacLayer<R, REG>, rx2_frequency: u32, rx2_data_rate: u8) -> Self {
        Self {
            mac,
            rx2_frequency,
            rx2_data_rate,
        }
    }

    /// Configure RX2 window
    pub fn configure_rx2(&mut self, frequency: u32, data_rate: u8) -> Result<(), MacError<R::Error>> {
        self.rx2_frequency = frequency;
        self.rx2_data_rate = data_rate;
        self.mac.set_rx_config(frequency, DataRate::from_index(data_rate), 0)
    }
}

impl<R, REG> DeviceClass<R, REG> for ClassC<R, REG>
where
    R: Radio,
    REG: Region + Debug + Clone,
{
    type Error = MacError<R::Error>;

    fn operating_mode(&self) -> OperatingMode {
        OperatingMode::ClassC
    }

    fn process(&mut self) -> Result<(), MacError<R::Error>> {
        // Configure continuous RX2
        self.mac.set_rx_config(self.rx2_frequency, DataRate::from_index(self.rx2_data_rate), 0)?;

        // Process any received data
        let mut buffer = [0u8; 256];
        if let Ok(len) = self.mac.receive(&mut buffer) {
            // Only process if we received data
            if len > 0 {
                // Decrypt and verify payload
                let payload = self.mac.decrypt_payload(&buffer[..len])?;

                // Extract MAC commands if present (port 0)
                if let Some(port) = payload.first() {
                    if *port == 0 {
                        // Extract and process MAC commands from FRMPayload
                        if let Some(commands) = self.mac.extract_mac_commands(&payload[1..]) {
                            for command in commands {
                                self.mac.process_mac_command(command)?;
                            }
                        }
                    }
                }

                // Increment frame counter after successful reception
                self.mac.increment_frame_counter_down();
            }
        }

        Ok(())
    }

    fn send_data(&mut self, port: u8, data: &[u8], confirmed: bool) -> Result<(), MacError<R::Error>> {
        if confirmed {
            self.mac.send_confirmed(port, data)
        } else {
            self.mac.send_unconfirmed(port, data)
        }
    }

    fn send_join_request(
        &mut self,
        dev_eui: [u8; 8],
        app_eui: [u8; 8],
        app_key: AESKey,
    ) -> Result<(), MacError<R::Error>> {
        self.mac.join_request(dev_eui, app_eui, app_key)
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, MacError<R::Error>> {
        self.mac.receive(buffer)
    }

    fn get_session_state(&self) -> SessionState {
        self.mac.get_session_state().clone()
    }

    fn get_mac_layer(&self) -> &MacLayer<R, REG> {
        &self.mac
    }
}
