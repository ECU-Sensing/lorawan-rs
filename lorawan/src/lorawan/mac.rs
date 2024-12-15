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
                // Queue a link check request to be sent in the next uplink
                self.queue_mac_command(MacCommand::LinkCheckReq)
            }
            MacCommand::LinkCheckAns { margin, gateway_count } => {
                // Store link quality information for application use
                // Margin is the link margin in dB of the last successful uplink
                // Gateway count is the number of gateways that received the uplink
                Ok(())
            }
            MacCommand::LinkADRReq { data_rate, tx_power, ch_mask, ch_mask_cntl, nb_trans } => {
                let mut power_ack = false;
                let mut data_rate_ack = false;
                let mut channel_mask_ack = false;

                // Validate and set TX power if in valid range
                if self.region.is_valid_tx_power(tx_power) {
                    self.region.set_tx_power(tx_power);
                    power_ack = true;
                }

                // Validate and set data rate if supported
                if self.region.is_valid_data_rate(data_rate) {
                    self.region.set_data_rate(data_rate);
                    data_rate_ack = true;
                }

                // Apply channel mask if valid
                if self.region.is_valid_channel_mask(ch_mask, ch_mask_cntl) {
                    self.region.apply_channel_mask(ch_mask, ch_mask_cntl);
                    channel_mask_ack = true;
                }

                // Set number of transmissions if specified
                if nb_trans > 0 {
                    // Store nb_trans for future uplinks
                }

                // Queue acknowledgment
                self.queue_mac_command(MacCommand::LinkADRAns {
                    power_ack,
                    data_rate_ack,
                    channel_mask_ack,
                })
            }
            MacCommand::LinkADRAns { power_ack, data_rate_ack, channel_mask_ack } => {
                // Process response from end-device about ADR request
                // If all acks are true, the device has successfully applied all changes
                if power_ack && data_rate_ack && channel_mask_ack {
                    Ok(())
                } else {
                    Err(MacError::InvalidValue)
                }
            }
            MacCommand::DutyCycleReq { max_duty_cycle } => {
                // Set the maximum duty cycle
                // max_duty_cycle = 0 means no duty cycle limitation
                // max_duty_cycle = 1 means 1/1 duty cycle (100%)
                // max_duty_cycle = 2 means 1/2 duty cycle (50%)
                // max_duty_cycle = 16 means 1/16 duty cycle (6.25%)
                if max_duty_cycle <= 15 {
                    // Store duty cycle for future transmissions
                    self.queue_mac_command(MacCommand::DutyCycleAns)
                } else {
                    Err(MacError::InvalidValue)
                }
            }
            MacCommand::DutyCycleAns => {
                // Acknowledgment of duty cycle request
                Ok(())
            }
            MacCommand::RXParamSetupReq { rx1_dr_offset, rx2_data_rate, freq } => {
                let mut rx1_dr_offset_ack = false;
                let mut rx2_data_rate_ack = false;
                let mut channel_ack = false;

                // Validate RX1 data rate offset
                if rx1_dr_offset <= 5 {
                    // Store RX1 data rate offset
                    rx1_dr_offset_ack = true;
                }

                // Validate RX2 data rate
                if self.region.is_valid_data_rate(rx2_data_rate) {
                    // Store RX2 data rate
                    rx2_data_rate_ack = true;
                }

                // Validate frequency
                if self.region.is_valid_frequency(freq) {
                    // Store RX2 frequency
                    channel_ack = true;
                }

                // Queue acknowledgment
                self.queue_mac_command(MacCommand::RXParamSetupAns {
                    rx1_dr_offset_ack,
                    rx2_data_rate_ack,
                    channel_ack,
                })
            }
            MacCommand::RXParamSetupAns { rx1_dr_offset_ack, rx2_data_rate_ack, channel_ack } => {
                // Process response about RX parameter setup
                if rx1_dr_offset_ack && rx2_data_rate_ack && channel_ack {
                    Ok(())
                } else {
                    Err(MacError::InvalidValue)
                }
            }
            MacCommand::DevStatusReq => {
                // Queue device status response with battery and margin information
                // Battery: 0 = external power, 1-254 = battery level, 255 = cannot measure
                // Margin: SNR of last received DevStatusReq [-32,31]
                self.queue_mac_command(MacCommand::DevStatusAns {
                    battery: 0, // Example: external power
                    margin: 0, // Example: 0 dB margin
                })
            }
            MacCommand::DevStatusAns { battery: _, margin: _ } => {
                // Process device status information
                Ok(())
            }
            MacCommand::NewChannelReq { ch_index, freq, min_dr, max_dr } => {
                let mut channel_freq_ok = false;
                let mut data_rate_ok = false;

                // Validate frequency
                if self.region.is_valid_frequency(freq) {
                    channel_freq_ok = true;
                }

                // Validate data rate range
                if self.region.is_valid_data_rate(min_dr) && 
                   self.region.is_valid_data_rate(max_dr) && 
                   min_dr <= max_dr {
                    data_rate_ok = true;
                }

                // If valid, create new channel
                if channel_freq_ok && data_rate_ok {
                    // Create and store new channel configuration
                }

                // Queue acknowledgment
                self.queue_mac_command(MacCommand::NewChannelAns {
                    channel_freq_ok,
                    data_rate_ok,
                })
            }
            MacCommand::NewChannelAns { channel_freq_ok, data_rate_ok } => {
                // Process response about new channel creation
                if channel_freq_ok && data_rate_ok {
                    Ok(())
                } else {
                    Err(MacError::InvalidValue)
                }
            }
            MacCommand::RXTimingSetupReq { delay } => {
                // Set delay for RX1 window
                // delay = 0 means 1 second
                // delay = 1 means 1 second
                // delay = 15 means 15 seconds
                if delay <= 15 {
                    // Store RX1 delay
                    self.queue_mac_command(MacCommand::RXTimingSetupAns)
                } else {
                    Err(MacError::InvalidValue)
                }
            }
            MacCommand::RXTimingSetupAns => {
                // Acknowledgment of RX timing setup
                Ok(())
            }
            MacCommand::TxParamSetupReq { downlink_dwell_time, uplink_dwell_time, max_eirp } => {
                // Set TX parameters
                // Store dwell times and maximum EIRP
                if max_eirp <= 15 {
                    // Store parameters
                    self.queue_mac_command(MacCommand::TxParamSetupAns)
                } else {
                    Err(MacError::InvalidValue)
                }
            }
            MacCommand::TxParamSetupAns => {
                // Acknowledgment of TX parameter setup
                Ok(())
            }
            MacCommand::DlChannelReq { ch_index, freq } => {
                let mut channel_freq_ok = false;
                let mut uplink_freq_exists = false;

                // Validate frequency
                if self.region.is_valid_frequency(freq) {
                    channel_freq_ok = true;
                }

                // Check if uplink frequency exists for this channel
                if let Some(channel) = self.region.get_channel(ch_index) {
                    if channel.frequency > 0 {
                        uplink_freq_exists = true;
                    }
                }

                // If valid, update downlink frequency
                if channel_freq_ok && uplink_freq_exists {
                    // Update channel downlink frequency
                }

                // Queue acknowledgment
                self.queue_mac_command(MacCommand::DlChannelAns {
                    channel_freq_ok,
                    uplink_freq_exists,
                })
            }
            MacCommand::DlChannelAns { channel_freq_ok, uplink_freq_exists } => {
                // Process response about downlink channel modification
                if channel_freq_ok && uplink_freq_exists {
                    Ok(())
                } else {
                    Err(MacError::InvalidValue)
                }
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
