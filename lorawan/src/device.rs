use core::time::Duration;

use crate::{
    class::{ClassA, ClassB, ClassC, DeviceClass, OperatingMode},
    config::device::{DeviceConfig, SessionState},
    lorawan::{
        commands::{CommandHandler, DownlinkCommand},
        mac::{MacError, MacLayer},
        region::{Region, US915},
    },
    radio::Radio,
};

/// LoRaWAN device error types
#[derive(Debug)]
pub enum DeviceError<E> {
    /// Radio/MAC layer error
    Mac(MacError<E>),
    /// Device not activated
    NotActivated,
    /// Invalid configuration
    InvalidConfig,
    /// Command processing error
    CommandError,
}

impl<E> From<MacError<E>> for DeviceError<E> {
    fn from(error: MacError<E>) -> Self {
        DeviceError::Mac(error)
    }
}

/// LoRaWAN device state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceState {
    /// Device is idle
    Idle,
    /// Device is joining network
    Joining,
    /// Device is activated and ready
    Ready,
    /// Device is transmitting
    Transmitting,
    /// Device is receiving
    Receiving,
}

/// High-level LoRaWAN device
pub struct LoRaWANDevice<R: Radio, REG: Region> {
    /// Device configuration
    config: DeviceConfig,
    /// Current device state
    state: DeviceState,
    /// Current operating mode (Class A/B/C)
    mode: OperatingMode,
    /// Radio driver
    radio: Option<R>,
    /// Region configuration
    region: REG,
    /// Class A implementation
    class_a: Option<ClassA<R, REG>>,
    /// Class B implementation
    class_b: Option<ClassB<R, REG>>,
    /// Class C implementation
    class_c: Option<ClassC<R, REG>>,
    /// Session state
    session: Option<SessionState>,
}

impl<R: Radio, REG: Region> LoRaWANDevice<R, REG> {
    /// Create a new LoRaWAN device
    pub fn new(
        radio: R,
        config: DeviceConfig,
        region: REG,
        mode: OperatingMode,
    ) -> Result<Self, DeviceError<R::Error>> {
        let mut device = Self {
            config,
            state: DeviceState::Idle,
            mode,
            radio: Some(radio),
            region,
            class_a: None,
            class_b: None,
            class_c: None,
            session: None,
        };

        // Initialize appropriate device class
        device.set_class(mode)?;

        Ok(device)
    }

    /// Join network using OTAA
    pub fn join_otaa(&mut self) -> Result<(), DeviceError<R::Error>> {
        self.state = DeviceState::Joining;

        // Send join request based on current class
        match self.mode {
            OperatingMode::ClassA => {
                if let Some(class_a) = &mut self.class_a {
                    // Send join request
                    class_a.send_join_request(
                        self.config.dev_eui,
                        self.config.app_eui,
                        self.config.app_key,
                    )?;

                    // Wait for join accept in RX windows
                    let mut buffer = [0u8; 256];
                    if let Ok(len) = class_a.receive(&mut buffer) {
                        // Process join accept
                        // TODO: Parse join accept and derive session keys
                        self.state = DeviceState::Ready;
                    }
                }
            }
            OperatingMode::ClassB => {
                if let Some(class_b) = &mut self.class_b {
                    // Send join request
                    class_b.send_join_request(
                        self.config.dev_eui,
                        self.config.app_eui,
                        self.config.app_key,
                    )?;

                    // Wait for join accept in RX windows
                    let mut buffer = [0u8; 256];
                    if let Ok(len) = class_b.receive(&mut buffer) {
                        // Process join accept
                        // TODO: Parse join accept and derive session keys
                        self.state = DeviceState::Ready;
                    }
                }
            }
            OperatingMode::ClassC => {
                if let Some(class_c) = &mut self.class_c {
                    // Send join request
                    class_c.send_join_request(
                        self.config.dev_eui,
                        self.config.app_eui,
                        self.config.app_key,
                    )?;

                    // Wait for join accept in RX windows
                    let mut buffer = [0u8; 256];
                    if let Ok(len) = class_c.receive(&mut buffer) {
                        // Process join accept
                        // TODO: Parse join accept and derive session keys
                        self.state = DeviceState::Ready;
                    }
                }
            }
        }

        Ok(())
    }

    /// Activate device using ABP
    pub fn activate_abp(
        &mut self,
        dev_addr: [u8; 4],
        nwk_skey: [u8; 16],
        app_skey: [u8; 16],
    ) -> Result<(), DeviceError<R::Error>> {
        // Create session state for ABP
        let session = SessionState::new_abp(dev_addr, nwk_skey, app_skey);
        self.session = Some(session);
        self.state = DeviceState::Ready;
        Ok(())
    }

    /// Send uplink data
    pub fn send_uplink(
        &mut self,
        port: u8,
        data: &[u8],
        confirmed: bool,
    ) -> Result<(), DeviceError<R::Error>> {
        if self.session.is_none() {
            return Err(DeviceError::NotActivated);
        }

        self.state = DeviceState::Transmitting;

        // Send data based on current class
        match self.mode {
            OperatingMode::ClassA => {
                if let Some(class_a) = &mut self.class_a {
                    class_a.send_data(port, data, confirmed)?;
                }
            }
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

        self.state = DeviceState::Ready;
        Ok(())
    }

    /// Process device operations
    pub fn process(&mut self) -> Result<(), DeviceError<R::Error>> {
        // Process based on current class
        match self.mode {
            OperatingMode::ClassA => {
                if let Some(class_a) = &mut self.class_a {
                    class_a.process()?;
                }
            }
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

    /// Set device class
    pub fn set_class(&mut self, class: OperatingMode) -> Result<(), DeviceError<R::Error>> {
        // Only allow class change if device is ready or idle
        if self.state != DeviceState::Ready && self.state != DeviceState::Idle {
            return Err(DeviceError::InvalidConfig);
        }

        self.mode = class;

        // Initialize appropriate class implementation
        match class {
            OperatingMode::ClassA => {
                // Initialize Class A
                if self.class_a.is_none() {
                    let mac = MacLayer::new(
                        self.radio.take().ok_or(DeviceError::InvalidConfig)?,
                        self.region.clone(),
                        self.session.clone().ok_or(DeviceError::NotActivated)?,
                    );
                    self.class_a = Some(ClassA::new(mac));
                }
                self.class_b = None;
                self.class_c = None;
            }
            OperatingMode::ClassB => {
                // Initialize Class B
                if self.class_b.is_none() {
                    let mac = MacLayer::new(
                        self.radio.take().ok_or(DeviceError::InvalidConfig)?,
                        self.region.clone(),
                        self.session.clone().ok_or(DeviceError::NotActivated)?,
                    );
                    self.class_b = Some(ClassB::new(mac));
                }
                self.class_a = None;
                self.class_c = None;
            }
            OperatingMode::ClassC => {
                // Initialize Class C
                if self.class_c.is_none() {
                    let mac = MacLayer::new(
                        self.radio.take().ok_or(DeviceError::InvalidConfig)?,
                        self.region.clone(),
                        self.session.clone().ok_or(DeviceError::NotActivated)?,
                    );
                    // Use default RX2 parameters for Class C
                    let rx2_frequency = 923_300_000; // Default US915 RX2 frequency
                    let rx2_data_rate = 8; // Default US915 RX2 data rate
                    self.class_c = Some(ClassC::new(mac, rx2_frequency, rx2_data_rate));
                }
                self.class_a = None;
                self.class_b = None;
            }
        }

        Ok(())
    }

    /// Get current device state
    pub fn state(&self) -> DeviceState {
        self.state
    }

    /// Get current operating mode
    pub fn operating_mode(&self) -> OperatingMode {
        self.mode
    }
}

impl<R: Radio, REG: Region> CommandHandler for LoRaWANDevice<R, REG> {
    type Error = DeviceError<R::Error>;

    fn handle_downlink_cmd(&mut self, command: DownlinkCommand) -> Result<(), Self::Error> {
        match command {
            DownlinkCommand::SetInterval(interval) => {
                // TODO: Update device interval
                Ok(())
            }
            DownlinkCommand::ShowFirmwareVersion => {
                // TODO: Prepare firmware version response
                Ok(())
            }
            DownlinkCommand::Reboot => {
                // TODO: Handle device reboot
                Ok(())
            }
            DownlinkCommand::Custom(port, data) => {
                // TODO: Handle custom command
                Ok(())
            }
        }
    }
} 