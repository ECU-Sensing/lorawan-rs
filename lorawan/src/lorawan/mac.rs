use heapless::Vec;

use super::commands::MacCommand;
use super::phy::PhyLayer;
use super::region::{Channel, DataRate, Region, US915};
use crate::config::device::{AESKey, DevAddr, SessionState};
use crate::crypto::{self, Direction, MIC_SIZE};
use crate::radio::traits::Radio;

/// Maximum MAC payload size
pub const MAX_MAC_PAYLOAD: usize = 242;

/// Maximum frame size
pub const MAX_FRAME_SIZE: usize = 256;

/// Maximum number of MAC commands
pub const MAX_MAC_COMMANDS: usize = 8;

/// MAC layer errors
#[derive(Debug)]
pub enum MacError<E> {
    /// Radio error
    Radio(E),
    /// Invalid frame format
    InvalidFrame,
    /// Invalid length
    InvalidLength,
    /// Invalid value
    InvalidValue,
    /// Unknown command
    UnknownCommand,
    /// Buffer too small
    BufferTooSmall,
    /// Not joined to network
    NotJoined,
    /// Invalid MIC
    InvalidMic,
    /// Invalid address
    InvalidAddress,
    /// Invalid frequency
    InvalidFrequency,
    /// Invalid data rate
    InvalidDataRate,
    /// Invalid channel
    InvalidChannel,
    /// Invalid port
    InvalidPort,
    /// Invalid payload size
    InvalidPayloadSize,
    /// Invalid configuration
    InvalidConfig,
    /// Timeout
    Timeout,
}

impl<E> From<E> for MacError<E> {
    fn from(error: E) -> Self {
        MacError::Radio(error)
    }
}

/// Frame control field
#[derive(Debug, Clone, Copy)]
pub struct FCtrl {
    /// Adaptive data rate enabled
    pub adr: bool,
    /// ADR acknowledgment request
    pub adr_ack_req: bool,
    /// Frame pending bit
    pub ack: bool,
    /// Frame pending bit
    pub fpending: bool,
    /// FOpts field length
    pub foptslen: u8,
}

impl FCtrl {
    /// Create a new frame control field with default values
    pub fn new() -> Self {
        Self {
            adr: false,
            adr_ack_req: false,
            ack: false,
            fpending: false,
            foptslen: 0,
        }
    }

    /// Convert frame control field to byte representation
    pub fn to_byte(&self) -> u8 {
        let mut byte = 0;
        if self.adr {
            byte |= 0x80;
        }
        if self.adr_ack_req {
            byte |= 0x40;
        }
        if self.ack {
            byte |= 0x20;
        }
        if self.fpending {
            byte |= 0x10;
        }
        byte |= self.foptslen & 0x0F;
        byte
    }
}

/// Frame header
#[derive(Debug)]
pub struct FHDR {
    /// Device address
    pub dev_addr: DevAddr,
    /// Frame control field
    pub f_ctrl: FCtrl,
    /// Frame counter
    pub f_cnt: u16,
    /// Frame options
    pub f_opts: Vec<u8, 15>,
}

impl FHDR {
    /// Serialize frame header to bytes
    pub fn serialize(&self) -> Vec<u8, 16> {
        let mut buffer = Vec::new();
        let addr_bytes = self.dev_addr.as_bytes();
        buffer.extend_from_slice(addr_bytes).unwrap();
        buffer.push(self.f_ctrl.to_byte()).unwrap();
        buffer.extend_from_slice(&self.f_cnt.to_le_bytes()).unwrap();
        buffer.extend_from_slice(&self.f_opts).unwrap();
        buffer
    }
}

/// MAC layer
pub struct MacLayer<R: Radio, REG: Region> {
    /// PHY layer
    phy: PhyLayer<R>,
    /// Region configuration
    region: REG,
    /// Session state
    session: SessionState,
    /// MAC commands to be sent
    pending_commands: Vec<MacCommand, MAX_MAC_COMMANDS>,
}

impl<R: Radio, REG: Region> MacLayer<R, REG> {
    /// Create new MAC layer
    pub fn new(radio: R, region: REG, session: SessionState) -> Self {
        Self {
            phy: PhyLayer::new(radio),
            region,
            session,
            pending_commands: Vec::new(),
        }
    }

    /// Get radio reference
    pub fn get_radio(&self) -> &R {
        &self.phy.radio
    }

    /// Get radio mutable reference
    pub fn get_radio_mut(&mut self) -> &mut R {
        &mut self.phy.radio
    }

    /// Get device address
    pub fn get_device_address(&self) -> Option<DevAddr> {
        Some(self.session.dev_addr)
    }

    /// Set RX configuration
    pub fn set_rx_config(
        &mut self,
        frequency: u32,
        data_rate: DataRate,
        timeout_ms: u32,
    ) -> Result<(), MacError<R::Error>> {
        self.phy
            .configure_rx::<REG>(frequency, data_rate, timeout_ms)
            .map_err(MacError::Radio)
    }

    /// Get RX1 parameters
    pub fn get_rx1_params(&mut self) -> Result<(u32, DataRate), MacError<R::Error>> {
        let channel = self
            .region
            .get_next_channel()
            .ok_or(MacError::InvalidChannel)?;
        Ok(self.region.rx1_window(&channel))
    }

    /// Send unconfirmed data
    pub fn send_unconfirmed(&mut self, f_port: u8, data: &[u8]) -> Result<(), MacError<R::Error>> {
        let mut buffer: Vec<u8, MAX_FRAME_SIZE> = Vec::new();

        // Add MAC header
        buffer.push(0x40).map_err(|_| MacError::BufferTooSmall)?; // Unconfirmed Data Up

        // Add frame header
        let fhdr = FHDR {
            dev_addr: self.session.dev_addr,
            f_ctrl: FCtrl::new(),
            f_cnt: self.session.fcnt_up as u16,
            f_opts: Vec::new(),
        };
        buffer
            .extend_from_slice(&fhdr.serialize())
            .map_err(|_| MacError::BufferTooSmall)?;

        // Add port
        buffer.push(f_port).map_err(|_| MacError::BufferTooSmall)?;

        // Add encrypted payload
        let encrypted = crypto::encrypt_payload(
            &self.session.app_skey,
            self.session.dev_addr,
            self.session.fcnt_up,
            Direction::Up,
            data,
        );
        buffer
            .extend_from_slice(&encrypted)
            .map_err(|_| MacError::BufferTooSmall)?;

        // Add MIC
        let mic = crypto::compute_mic(
            &self.session.nwk_skey,
            &buffer,
            self.session.dev_addr,
            self.session.fcnt_up,
            Direction::Up,
        );
        buffer
            .extend_from_slice(&mic)
            .map_err(|_| MacError::BufferTooSmall)?;

        // Transmit
        self.phy.transmit(&buffer).map_err(MacError::Radio)?;

        // Increment frame counter
        self.session.fcnt_up = self.session.fcnt_up.wrapping_add(1);

        Ok(())
    }

    /// Send confirmed data
    pub fn send_confirmed(&mut self, f_port: u8, data: &[u8]) -> Result<(), MacError<R::Error>> {
        let mut buffer: Vec<u8, MAX_FRAME_SIZE> = Vec::new();

        // Add MAC header
        buffer.push(0x80).map_err(|_| MacError::BufferTooSmall)?; // Confirmed Data Up

        // Add frame header
        let fhdr = FHDR {
            dev_addr: self.session.dev_addr,
            f_ctrl: FCtrl::new(),
            f_cnt: self.session.fcnt_up as u16,
            f_opts: Vec::new(),
        };
        buffer
            .extend_from_slice(&fhdr.serialize())
            .map_err(|_| MacError::BufferTooSmall)?;

        // Add port
        buffer.push(f_port).map_err(|_| MacError::BufferTooSmall)?;

        // Add encrypted payload
        let encrypted = crypto::encrypt_payload(
            &self.session.app_skey,
            self.session.dev_addr,
            self.session.fcnt_up,
            Direction::Up,
            data,
        );
        buffer
            .extend_from_slice(&encrypted)
            .map_err(|_| MacError::BufferTooSmall)?;

        // Add MIC
        let mic = crypto::compute_mic(
            &self.session.nwk_skey,
            &buffer,
            self.session.dev_addr,
            self.session.fcnt_up,
            Direction::Up,
        );
        buffer
            .extend_from_slice(&mic)
            .map_err(|_| MacError::BufferTooSmall)?;

        // Transmit
        self.phy.transmit(&buffer).map_err(MacError::Radio)?;

        // Increment frame counter
        self.session.fcnt_up = self.session.fcnt_up.wrapping_add(1);

        Ok(())
    }

    /// Decrypt payload
    pub fn decrypt_payload(
        &self,
        data: &[u8],
    ) -> Result<Vec<u8, MAX_MAC_PAYLOAD>, MacError<R::Error>> {
        if data.len() < MIC_SIZE {
            return Err(MacError::InvalidLength);
        }

        let payload = &data[..data.len() - MIC_SIZE];
        let mic = &data[data.len() - MIC_SIZE..];

        // Verify MIC
        let computed_mic = crypto::compute_mic(
            &self.session.nwk_skey,
            payload,
            self.session.dev_addr,
            self.session.fcnt_down,
            Direction::Down,
        );
        if mic != computed_mic {
            return Err(MacError::InvalidMic);
        }

        // Decrypt payload
        let decrypted = crypto::encrypt_payload(
            &self.session.app_skey,
            self.session.dev_addr,
            self.session.fcnt_down,
            Direction::Down,
            payload,
        );

        let mut result = Vec::new();
        result
            .extend_from_slice(&decrypted)
            .map_err(|_| MacError::BufferTooSmall)?;
        Ok(result)
    }

    /// Extract MAC commands
    pub fn extract_mac_commands(
        &self,
        payload: &[u8],
    ) -> Option<Vec<MacCommand, MAX_MAC_COMMANDS>> {
        let mut commands = Vec::new();
        let mut i = 0;
        while i < payload.len() {
            let cid = payload[i];
            i += 1;
            if let Some(cmd) = MacCommand::from_bytes(cid, &payload[i..]) {
                commands.push(cmd.clone()).ok()?;
                i += cmd.len();
            } else {
                return None;
            }
        }
        Some(commands)
    }

    /// Queue MAC command
    pub fn queue_mac_command(&mut self, command: MacCommand) -> Result<(), MacError<R::Error>> {
        self.pending_commands
            .push(command)
            .map_err(|_| MacError::BufferTooSmall)
    }

    /// Increment frame counter down
    pub fn increment_frame_counter_down(&mut self) {
        self.session.fcnt_down = self.session.fcnt_down.wrapping_add(1);
    }

    /// Receive data
    pub fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, MacError<R::Error>> {
        self.phy.receive(buffer).map_err(MacError::Radio)
    }

    /// Process MAC command
    pub fn process_mac_command(&mut self, command: MacCommand) -> Result<(), MacError<R::Error>> {
        match command {
            MacCommand::LinkCheckReq => {
                // TODO: Implement link check request
                Ok(())
            }
            MacCommand::LinkCheckAns { margin: _, gateway_count: _ } => {
                // TODO: Handle link check answer
                Ok(())
            }
            MacCommand::LinkADRReq { data_rate: _, tx_power: _, ch_mask: _, ch_mask_cntl: _, nb_trans: _ } => {
                // TODO: Handle link ADR request
                Ok(())
            }
            MacCommand::LinkADRAns { power_ack: _, data_rate_ack: _, channel_mask_ack: _ } => {
                // TODO: Handle link ADR answer
                Ok(())
            }
            MacCommand::DutyCycleReq { max_duty_cycle: _ } => {
                // TODO: Handle duty cycle request
                Ok(())
            }
            MacCommand::DutyCycleAns => {
                // TODO: Handle duty cycle answer
                Ok(())
            }
            MacCommand::RXParamSetupReq { rx1_dr_offset: _, rx2_data_rate: _, freq: _ } => {
                // TODO: Handle RX param setup request
                Ok(())
            }
            MacCommand::RXParamSetupAns { rx1_dr_offset_ack: _, rx2_data_rate_ack: _, channel_ack: _ } => {
                // TODO: Handle RX param setup answer
                Ok(())
            }
            MacCommand::DevStatusReq => {
                // TODO: Handle device status request
                Ok(())
            }
            MacCommand::DevStatusAns { battery: _, margin: _ } => {
                // TODO: Handle device status answer
                Ok(())
            }
            MacCommand::NewChannelReq { ch_index: _, freq: _, min_dr: _, max_dr: _ } => {
                // TODO: Handle new channel request
                Ok(())
            }
            MacCommand::NewChannelAns { channel_freq_ok: _, data_rate_ok: _ } => {
                // TODO: Handle new channel answer
                Ok(())
            }
            MacCommand::RXTimingSetupReq { delay: _ } => {
                // TODO: Handle RX timing setup request
                Ok(())
            }
            MacCommand::RXTimingSetupAns => {
                // TODO: Handle RX timing setup answer
                Ok(())
            }
            MacCommand::TxParamSetupReq { downlink_dwell_time: _, uplink_dwell_time: _, max_eirp: _ } => {
                // TODO: Handle TX param setup request
                Ok(())
            }
            MacCommand::TxParamSetupAns => {
                // TODO: Handle TX param setup answer
                Ok(())
            }
            MacCommand::DlChannelReq { ch_index: _, freq: _ } => {
                // TODO: Handle downlink channel request
                Ok(())
            }
            MacCommand::DlChannelAns { channel_freq_ok: _, uplink_freq_exists: _ } => {
                // TODO: Handle downlink channel answer
                Ok(())
            }
        }
    }

    /// Join request
    pub fn join_request(
        &mut self,
        _dev_eui: [u8; 8],
        _app_eui: [u8; 8],
        _app_key: AESKey,
    ) -> Result<(), MacError<R::Error>> {
        // TODO: Implement join request
        Ok(())
    }

    /// Configure for TTN
    pub fn configure_for_ttn(&mut self) -> Result<(), MacError<R::Error>> {
        if let Some(us915) = self.region.as_any_mut().downcast_mut::<US915>() {
            us915.configure_ttn_us915();
            Ok(())
        } else {
            Err(MacError::InvalidConfig)
        }
    }

    /// Get next channel
    pub fn get_next_channel(&mut self) -> Result<Channel, MacError<R::Error>> {
        self.region
            .get_next_channel()
            .ok_or(MacError::InvalidChannel)
    }

    /// Get beacon channels
    pub fn get_beacon_channels(&self) -> Vec<Channel, 8> {
        self.region.get_beacon_channels()
    }

    /// Get next beacon channel
    pub fn get_next_beacon_channel(&mut self) -> Option<Channel> {
        self.region.get_next_beacon_channel()
    }

    /// Get uplink frame counter
    pub fn get_frame_counter_up(&self) -> u32 {
        self.session.fcnt_up
    }

    /// Get downlink frame counter
    pub fn get_frame_counter_down(&self) -> u32 {
        self.session.fcnt_down
    }

    fn handle_mac_command(&mut self, command: MacCommand) -> Result<(), MacError<R::Error>> {
        match command {
            MacCommand::LinkCheckReq |
            MacCommand::LinkCheckAns { .. } |
            MacCommand::LinkADRReq { .. } |
            MacCommand::LinkADRAns { .. } |
            MacCommand::DutyCycleReq { .. } |
            MacCommand::DutyCycleAns |
            MacCommand::RXParamSetupReq { .. } |
            MacCommand::RXParamSetupAns { .. } |
            MacCommand::DevStatusReq |
            MacCommand::DevStatusAns { .. } |
            MacCommand::NewChannelAns { .. } |
            MacCommand::RXTimingSetupAns |
            MacCommand::TxParamSetupAns |
            MacCommand::DlChannelAns { .. } => Ok(()),

            MacCommand::NewChannelReq { ch_index, freq, min_dr: _, max_dr: _ } => {
                // Validate and configure new channel
                if !self.region.is_valid_frequency(freq) {
                    return Err(MacError::InvalidFrequency);
                }
                if ch_index as usize >= self.region.get_max_channels() {
                    return Err(MacError::InvalidChannel);
                }
                Ok(())
            },
            MacCommand::RXTimingSetupReq { delay: _ } => {
                // TODO: Store RX1 delay for future use
                Ok(())
            },
            MacCommand::TxParamSetupReq { downlink_dwell_time: _, uplink_dwell_time: _, max_eirp: _ } => {
                // TODO: Store TX parameters for future use
                Ok(())
            },
            MacCommand::DlChannelReq { ch_index: _, freq: _ } => {
                // TODO: Configure downlink channel
                Ok(())
            },
        }
    }
}
