use heapless::Vec;

/// EUI-64 (8 bytes)
pub type EUI64 = [u8; 8];
/// AES-128 key (16 bytes)
pub type AESKey = [u8; 16];
/// Device Address (4 bytes)
pub type DevAddr = [u8; 4];

/// LoRaWAN device class
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceClass {
    /// Class A: Uplink followed by two receive windows
    A,
    /// Class B: Scheduled receive slots (beaconing)
    B,
    /// Class C: Continuously listening except when transmitting
    C,
}

/// Device activation state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActivationState {
    /// Device is not activated
    Idle,
    /// Device is activated through OTAA
    OTAAActivated,
    /// Device is activated through ABP
    ABPActivated,
}

/// Device configuration for both OTAA and ABP activation
#[derive(Debug, Clone)]
pub struct DeviceConfig {
    /// Device EUI (unique device identifier)
    pub dev_eui: EUI64,
    /// Application EUI
    pub app_eui: EUI64,
    /// Application key (used for OTAA)
    pub app_key: AESKey,
    /// Device address (used for ABP or assigned during OTAA)
    pub dev_addr: Option<DevAddr>,
    /// Network session key (used for ABP or derived during OTAA)
    pub nwk_skey: Option<AESKey>,
    /// Application session key (used for ABP or derived during OTAA)
    pub app_skey: Option<AESKey>,
}

/// Session state for an activated device
#[derive(Debug, Clone)]
pub struct SessionState {
    /// Current activation state
    pub activation_state: ActivationState,
    /// Current device class
    pub device_class: DeviceClass,
    /// Device address (assigned during activation)
    pub dev_addr: DevAddr,
    /// Network session key
    pub nwk_skey: AESKey,
    /// Application session key
    pub app_skey: AESKey,
    /// Uplink frame counter
    pub fcnt_up: u32,
    /// Downlink frame counter
    pub fcnt_down: u32,
    /// Last used device nonce (for OTAA)
    pub dev_nonce: u16,
}

impl DeviceConfig {
    /// Create a new OTAA device configuration
    pub fn new_otaa(dev_eui: EUI64, app_eui: EUI64, app_key: AESKey) -> Self {
        Self {
            dev_eui,
            app_eui,
            app_key,
            dev_addr: None,
            nwk_skey: None,
            app_skey: None,
        }
    }

    /// Create a new ABP device configuration
    pub fn new_abp(
        dev_eui: EUI64,
        app_eui: EUI64,
        dev_addr: DevAddr,
        nwk_skey: AESKey,
        app_skey: AESKey,
    ) -> Self {
        Self {
            dev_eui,
            app_eui,
            app_key: [0; 16], // Not used in ABP
            dev_addr: Some(dev_addr),
            nwk_skey: Some(nwk_skey),
            app_skey: Some(app_skey),
        }
    }
}

impl SessionState {
    /// Create a new session state for ABP activation
    pub fn new_abp(dev_addr: DevAddr, nwk_skey: AESKey, app_skey: AESKey) -> Self {
        Self {
            activation_state: ActivationState::ABPActivated,
            device_class: DeviceClass::A, // Default to Class A
            dev_addr,
            nwk_skey,
            app_skey,
            fcnt_up: 0,
            fcnt_down: 0,
            dev_nonce: 0,
        }
    }

    /// Create a new session state for OTAA activation
    pub fn new_otaa(dev_addr: DevAddr, nwk_skey: AESKey, app_skey: AESKey) -> Self {
        Self {
            activation_state: ActivationState::OTAAActivated,
            device_class: DeviceClass::A, // Default to Class A
            dev_addr,
            nwk_skey,
            app_skey,
            fcnt_up: 0,
            fcnt_down: 0,
            dev_nonce: 0,
        }
    }

    /// Increment the uplink frame counter
    pub fn increment_fcnt_up(&mut self) {
        self.fcnt_up = self.fcnt_up.wrapping_add(1);
    }

    /// Increment the downlink frame counter
    pub fn increment_fcnt_down(&mut self) {
        self.fcnt_down = self.fcnt_down.wrapping_add(1);
    }

    /// Set the device class
    pub fn set_device_class(&mut self, class: DeviceClass) {
        self.device_class = class;
    }
} 