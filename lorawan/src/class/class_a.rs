//! LoRaWAN Class A device implementation
//!
//! Class A is the most basic device class, supporting bi-directional communication
//! where each uplink transmission is followed by two short downlink receive windows.

use super::{DeviceClass, OperatingMode};
use crate::config::device::AESKey;
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

impl<R: Radio, REG: Region> DeviceClass for ClassA<R, REG> {
    type Error = MacError<R::Error>;

    fn operating_mode(&self) -> OperatingMode {
        OperatingMode::ClassA
    }

    fn process(&mut self) -> Result<(), MacError<R::Error>> {
        // Process RX windows
        let mut buffer = [0u8; 256];
        if let Ok(_len) = self.mac.receive(&mut buffer) {
            // TODO: Process received data
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
}
