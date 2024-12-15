#![no_std]

use heapless::Vec;
use lorawan::radio::traits::{Radio, RxConfig, TxConfig};

/// Mock radio error type
#[derive(Debug)]
pub enum MockError {
    /// Generic error
    Error,
}

/// Mock radio for testing
pub struct MockRadio {
    frequency: u32,
    power: i8,
    last_tx: Option<Vec<u8, 256>>,
    rx_data: Option<Vec<u8, 256>>,
}

impl MockRadio {
    /// Create new mock radio
    pub fn new() -> Self {
        Self {
            frequency: 0,
            power: 0,
            last_tx: None,
            rx_data: None,
        }
    }

    /// Set data to be returned by next receive call
    pub fn set_rx_data(&mut self, data: &[u8]) {
        let mut rx_data = Vec::new();
        rx_data.extend_from_slice(data).unwrap();
        self.rx_data = Some(rx_data);
    }

    /// Get last transmitted data
    pub fn get_last_tx(&self) -> Option<&[u8]> {
        self.last_tx.as_ref().map(|v| v.as_slice())
    }
}

impl Radio for MockRadio {
    type Error = MockError;

    fn init(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_frequency(&mut self, freq: u32) -> Result<(), Self::Error> {
        self.frequency = freq;
        Ok(())
    }

    fn set_tx_power(&mut self, power: i8) -> Result<(), Self::Error> {
        self.power = power;
        Ok(())
    }

    fn configure_tx(&mut self, config: TxConfig) -> Result<(), Self::Error> {
        self.frequency = config.frequency;
        self.power = config.power;
        Ok(())
    }

    fn configure_rx(&mut self, config: RxConfig) -> Result<(), Self::Error> {
        self.frequency = config.frequency;
        Ok(())
    }

    fn transmit(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        let mut tx_data = Vec::new();
        tx_data.extend_from_slice(data).unwrap();
        self.last_tx = Some(tx_data);
        Ok(())
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        if let Some(rx_data) = self.rx_data.take() {
            let len = rx_data.len().min(buffer.len());
            buffer[..len].copy_from_slice(&rx_data[..len]);
            Ok(len)
        } else {
            Ok(0)
        }
    }

    fn get_rssi(&mut self) -> Result<i16, Self::Error> {
        Ok(-50) // Mock RSSI value
    }

    fn get_snr(&mut self) -> Result<i8, Self::Error> {
        Ok(10) // Mock SNR value
    }

    fn is_transmitting(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }

    fn set_rx_gain(&mut self, _gain: u8) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_low_power_mode(&mut self, _enabled: bool) -> Result<(), Self::Error> {
        Ok(())
    }

    fn sleep(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
