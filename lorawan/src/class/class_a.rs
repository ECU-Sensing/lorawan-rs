//! LoRaWAN Class A device implementation
//!
//! Class A is the most basic device class, supporting bi-directional communication
//! where each uplink transmission is followed by two short downlink receive windows.

use super::{DeviceClass, OperatingMode};
use crate::config::device::{AESKey, SessionState};
use crate::lorawan::mac::{MacError, MacLayer};
use crate::lorawan::region::Region;
use crate::radio::traits::Radio;

/// Class A device implementation
pub struct ClassA<R: Radio, REG: Region> {
    /// MAC layer
    mac: MacLayer<R, REG>,
}

impl<R: Radio, REG: Region> ClassA<R, REG> {
    /// Create new Class A device
    pub fn new(mac: MacLayer<R, REG>) -> Self {
        Self { mac }
    }
}

impl<R: Radio, REG: Region> DeviceClass<R, REG> for ClassA<R, REG> {
    type Error = MacError<R::Error>;

    fn operating_mode(&self) -> OperatingMode {
        OperatingMode::ClassA
    }

    fn process(&mut self) -> Result<(), MacError<R::Error>> {
        // Process RX windows
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

    fn get_session_state(&self) -> SessionState {
        self.mac.get_session_state().clone()
    }

    fn get_mac_layer(&self) -> &MacLayer<R, REG> {
        &self.mac
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, MacError<R::Error>> {
        self.mac.receive(buffer)
    }
}
