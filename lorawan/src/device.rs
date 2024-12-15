//! High-level LoRaWAN device interface
//!
//! This module provides the main device interface for LoRaWAN communication.
//! It handles device configuration, activation, and message handling.

use crate::{
    class::{class_a::ClassA, class_b::ClassB, class_c::ClassC, DeviceClass, OperatingMode},
    config::device::{AESKey, DeviceConfig, SessionState},
    lorawan::{
        mac::{MacError, MacLayer},
        region::Region,
    },
    radio::traits::Radio,
};

/// LoRaWAN device error type
#[derive(Debug)]
pub enum DeviceError<E> {
    /// MAC layer error
    Mac(MacError<E>),
    /// Invalid configuration
    InvalidConfig,
    /// Invalid state for operation
    InvalidState,
}

impl<E> From<MacError<E>> for DeviceError<E> {
    fn from(error: MacError<E>) -> Self {
        DeviceError::Mac(error)
    }
}

/// LoRaWAN device implementation
pub struct LoRaWANDevice<R: Radio + Clone, REG: Region> {
    /// Current operating mode
    mode: OperatingMode,
    /// Class A implementation
    class_a: ClassA<R, REG>,
    /// Class B implementation
    class_b: Option<ClassB<R, REG>>,
    /// Class C implementation
    class_c: Option<ClassC<R, REG>>,
}

impl<R: Radio + Clone, REG: Region> LoRaWANDevice<R, REG> {
    /// Create new LoRaWAN device
    pub fn new(
        radio: R,
        config: DeviceConfig,
        region: REG,
        mode: OperatingMode,
    ) -> Result<Self, DeviceError<R::Error>> {
        // Initialize session state based on device configuration
        let session = match (config.dev_addr, config.nwk_skey, config.app_skey) {
            (Some(addr), Some(nwk), Some(app)) => {
                // ABP activation - use provided keys
                SessionState::new_abp(addr, nwk, app)
            }
            _ => {
                // OTAA activation - start with empty session
                SessionState::new()
            }
        };

        let mac = MacLayer::new(radio.clone(), region.clone(), session.clone());
        let class_a = ClassA::new(mac);

        let mut device = Self {
            mode,
            class_a,
            class_b: None,
            class_c: None,
        };

        // Initialize additional device classes if needed
        match mode {
            OperatingMode::ClassB => {
                let mac = MacLayer::new(radio.clone(), region.clone(), session.clone());
                device.class_b = Some(ClassB::new(mac));
            }
            OperatingMode::ClassC => {
                let mac = MacLayer::new(radio, region.clone(), session.clone());
                device.class_c = Some(ClassC::new(
                    mac,
                    region.rx2_frequency(),
                    region.rx2_data_rate(),
                ));
            }
            _ => {}
        }

        Ok(device)
    }

    /// Get current operating mode
    pub fn operating_mode(&self) -> OperatingMode {
        self.mode
    }

    /// Set operating mode
    pub fn set_operating_mode(&mut self, mode: OperatingMode) -> Result<(), DeviceError<R::Error>> {
        // Don't do anything if mode isn't changing
        if self.mode == mode {
            return Ok(());
        }

        // Get current session state from active class
        let session = match self.mode {
            OperatingMode::ClassA => self.class_a.get_session_state(),
            OperatingMode::ClassB => self
                .class_b
                .as_ref()
                .ok_or(DeviceError::InvalidState)?
                .get_session_state(),
            OperatingMode::ClassC => self
                .class_c
                .as_ref()
                .ok_or(DeviceError::InvalidState)?
                .get_session_state(),
        };

        // Get radio and region from current class
        let (radio, region) = match self.mode {
            OperatingMode::ClassA => {
                let mac = self.class_a.get_mac_layer();
                (mac.get_radio().clone(), mac.get_region().clone())
            }
            OperatingMode::ClassB => {
                let class_b = self.class_b.as_ref().ok_or(DeviceError::InvalidState)?;
                let mac = class_b.get_mac_layer();
                (mac.get_radio().clone(), mac.get_region().clone())
            }
            OperatingMode::ClassC => {
                let class_c = self.class_c.as_ref().ok_or(DeviceError::InvalidState)?;
                let mac = class_c.get_mac_layer();
                (mac.get_radio().clone(), mac.get_region().clone())
            }
        };

        // Initialize new class based on requested mode
        match mode {
            OperatingMode::ClassA => {
                let mac = MacLayer::new(radio, region, session);
                self.class_a = ClassA::new(mac);
                self.class_b = None;
                self.class_c = None;
            }
            OperatingMode::ClassB => {
                self.class_a = ClassA::new(MacLayer::new(
                    radio.clone(),
                    region.clone(),
                    session.clone(),
                ));
                let mac = MacLayer::new(radio, region.clone(), session);
                self.class_b = Some(ClassB::new(mac));
                self.class_c = None;
            }
            OperatingMode::ClassC => {
                self.class_a = ClassA::new(MacLayer::new(
                    radio.clone(),
                    region.clone(),
                    session.clone(),
                ));
                let mac = MacLayer::new(radio, region.clone(), session);
                self.class_c = Some(ClassC::new(
                    mac,
                    region.rx2_frequency(),
                    region.rx2_data_rate(),
                ));
                self.class_b = None;
            }
        }

        self.mode = mode;
        Ok(())
    }

    /// Process device operations
    pub fn process(&mut self) -> Result<(), DeviceError<R::Error>> {
        match self.mode {
            OperatingMode::ClassA => self.class_a.process()?,
            OperatingMode::ClassB => {
                if let Some(class_b) = &mut self.class_b {
                    class_b.process()?;
                }
            }
            OperatingMode::ClassC => {
                if let Some(class_c) = &mut self.class_c {
                    class_c.process()?;
                }
            }
        }
        Ok(())
    }

    /// Send data
    pub fn send_data(
        &mut self,
        port: u8,
        data: &[u8],
        confirmed: bool,
    ) -> Result<(), DeviceError<R::Error>> {
        match self.mode {
            OperatingMode::ClassA => self.class_a.send_data(port, data, confirmed)?,
            OperatingMode::ClassB => {
                if let Some(class_b) = &mut self.class_b {
                    class_b.send_data(port, data, confirmed)?;
                }
            }
            OperatingMode::ClassC => {
                if let Some(class_c) = &mut self.class_c {
                    class_c.send_data(port, data, confirmed)?;
                }
            }
        }
        Ok(())
    }

    /// Join network using OTAA
    pub fn join_otaa(
        &mut self,
        dev_eui: [u8; 8],
        app_eui: [u8; 8],
        app_key: AESKey,
    ) -> Result<(), DeviceError<R::Error>> {
        match self.mode {
            OperatingMode::ClassA => self.class_a.send_join_request(dev_eui, app_eui, app_key)?,
            OperatingMode::ClassB => {
                if let Some(class_b) = &mut self.class_b {
                    class_b.send_join_request(dev_eui, app_eui, app_key)?;
                }
            }
            OperatingMode::ClassC => {
                if let Some(class_c) = &mut self.class_c {
                    class_c.send_join_request(dev_eui, app_eui, app_key)?;
                }
            }
        }
        Ok(())
    }

    /// Receive data
    pub fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, DeviceError<R::Error>> {
        match self.mode {
            OperatingMode::ClassA => Ok(self.class_a.receive(buffer)?),
            OperatingMode::ClassB => {
                if let Some(class_b) = &mut self.class_b {
                    Ok(class_b.receive(buffer)?)
                } else {
                    Ok(0)
                }
            }
            OperatingMode::ClassC => {
                if let Some(class_c) = &mut self.class_c {
                    Ok(class_c.receive(buffer)?)
                } else {
                    Ok(0)
                }
            }
        }
    }

    /// Get current session state
    pub fn get_session_state(&self) -> SessionState {
        match self.mode {
            OperatingMode::ClassA => self.class_a.get_session_state(),
            OperatingMode::ClassB => self
                .class_b
                .as_ref()
                .expect("Class B not initialized")
                .get_session_state(),
            OperatingMode::ClassC => self
                .class_c
                .as_ref()
                .expect("Class C not initialized")
                .get_session_state(),
        }
    }
}
