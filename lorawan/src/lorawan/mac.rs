use heapless::Vec;

use crate::config::device::{AESKey, DevAddr, EUI64, SessionState};
use crate::crypto::{self, Direction, MIC_SIZE};
use super::region::{Channel, DataRate, Region};
use super::phy::{PhyLayer, PhyConfig};
use crate::radio::Radio;

/// Maximum MAC payload size
pub const MAX_MAC_PAYLOAD_SIZE: usize = 242;

/// MAC header types
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum MType {
    JoinRequest = 0x00,
    JoinAccept = 0x20,
    UnconfirmedDataUp = 0x40,
    UnconfirmedDataDown = 0x60,
    ConfirmedDataUp = 0x80,
    ConfirmedDataDown = 0xA0,
    RejoinRequest = 0xC0,
    Proprietary = 0xE0,
}

/// Frame header flags
#[derive(Debug, Clone, Copy)]
pub struct FCtrl {
    pub adr: bool,
    pub adr_ack_req: bool,
    pub ack: bool,
    pub f_pending: bool,
    pub f_opts_len: u8,
}

impl FCtrl {
    fn to_byte(&self) -> u8 {
        let mut byte = self.f_opts_len & 0x0F;
        if self.adr {
            byte |= 0x80;
        }
        if self.adr_ack_req {
            byte |= 0x40;
        }
        if self.ack {
            byte |= 0x20;
        }
        if self.f_pending {
            byte |= 0x10;
        }
        byte
    }

    fn from_byte(byte: u8) -> Self {
        Self {
            adr: (byte & 0x80) != 0,
            adr_ack_req: (byte & 0x40) != 0,
            ack: (byte & 0x20) != 0,
            f_pending: (byte & 0x10) != 0,
            f_opts_len: byte & 0x0F,
        }
    }
}

/// Frame header
#[derive(Debug)]
pub struct FHDR {
    pub dev_addr: DevAddr,
    pub f_ctrl: FCtrl,
    pub f_cnt: u16,
    pub f_opts: Vec<u8, 15>,
}

impl FHDR {
    fn serialize(&self) -> Vec<u8, 64> {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&self.dev_addr).unwrap();
        buffer.push(self.f_ctrl.to_byte()).unwrap();
        buffer.extend_from_slice(&self.f_cnt.to_le_bytes()).unwrap();
        buffer.extend_from_slice(&self.f_opts).unwrap();
        buffer
    }
}

/// MAC layer error
#[derive(Debug)]
pub enum MacError<E> {
    /// Radio error
    Radio(E),
    /// Invalid MIC
    InvalidMic,
    /// Buffer too small
    BufferTooSmall,
    /// Invalid frame
    InvalidFrame,
}

/// MAC layer state
pub struct MacLayer<R: Radio, REG: Region> {
    /// PHY layer
    phy: PhyLayer<R>,
    /// Region configuration
    region: REG,
    /// Session state
    session: SessionState,
}

impl<R: Radio, REG: Region> MacLayer<R, REG> {
    /// Create a new MAC layer
    pub fn new(radio: R, region: REG, session: SessionState) -> Self {
        Self {
            phy: PhyLayer::new(radio, PhyConfig::default()),
            region,
            session,
        }
    }

    /// Initialize the MAC layer
    pub fn init(&mut self) -> Result<(), MacError<R::Error>> {
        self.phy.init().map_err(MacError::Radio)
    }

    /// Send unconfirmed data
    pub fn send_unconfirmed(
        &mut self,
        f_port: u8,
        data: &[u8],
    ) -> Result<(), MacError<R::Error>> {
        self.send_data(MType::UnconfirmedDataUp, f_port, data)
    }

    /// Send confirmed data
    pub fn send_confirmed(
        &mut self,
        f_port: u8,
        data: &[u8],
    ) -> Result<(), MacError<R::Error>> {
        self.send_data(MType::ConfirmedDataUp, f_port, data)
    }

    /// Send data frame
    fn send_data(
        &mut self,
        mtype: MType,
        f_port: u8,
        data: &[u8],
    ) -> Result<(), MacError<R::Error>> {
        // Select next channel using frequency hopping
        let channel = self.region
            .get_next_channel()
            .ok_or(MacError::InvalidFrame)?;

        // Configure radio for selected channel
        self.phy
            .configure_tx::<REG>(channel, self.region.data_rate())
            .map_err(MacError::Radio)?;

        // Prepare frame
        let mut buffer = Vec::new();

        // Add MAC header
        buffer.push(mtype as u8).map_err(|_| MacError::BufferTooSmall)?;

        // Add frame header
        let fhdr = FHDR {
            dev_addr: self.session.dev_addr,
            f_ctrl: FCtrl {
                adr: false,
                adr_ack_req: false,
                ack: false,
                f_pending: false,
                f_opts_len: 0,
            },
            f_cnt: self.session.fcnt_up as u16,
            f_opts: Vec::new(),
        };
        buffer.extend_from_slice(&fhdr.serialize()).map_err(|_| MacError::BufferTooSmall)?;

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
        buffer.extend_from_slice(&encrypted).map_err(|_| MacError::BufferTooSmall)?;

        // Add MIC
        let mic = crypto::compute_mic(
            &self.session.nwk_skey,
            &buffer,
            self.session.dev_addr,
            self.session.fcnt_up,
            Direction::Up,
        );
        buffer.extend_from_slice(&mic).map_err(|_| MacError::BufferTooSmall)?;

        // Transmit
        self.phy.transmit(&buffer).map_err(MacError::Radio)?;

        // Increment frame counter
        self.session.increment_fcnt_up();

        Ok(())
    }

    /// Send join request
    pub fn join_request(
        &mut self,
        dev_eui: EUI64,
        app_eui: EUI64,
        app_key: AESKey,
    ) -> Result<(), MacError<R::Error>> {
        // Select next join channel using frequency hopping
        let channel = self.region
            .get_next_join_channel()
            .ok_or(MacError::InvalidFrame)?;

        // Configure radio for selected channel
        self.phy
            .configure_tx::<REG>(channel, self.region.data_rate())
            .map_err(MacError::Radio)?;

        // Prepare frame
        let mut buffer = Vec::new();

        // Add MAC header
        buffer.push(MType::JoinRequest as u8).map_err(|_| MacError::BufferTooSmall)?;

        // Add AppEUI
        buffer.extend_from_slice(&app_eui).map_err(|_| MacError::BufferTooSmall)?;

        // Add DevEUI
        buffer.extend_from_slice(&dev_eui).map_err(|_| MacError::BufferTooSmall)?;

        // Add DevNonce
        buffer.extend_from_slice(&self.session.dev_nonce.to_le_bytes()).map_err(|_| MacError::BufferTooSmall)?;

        // Add MIC
        let mic = crypto::compute_join_request_mic(&app_key, &buffer);
        buffer.extend_from_slice(&mic).map_err(|_| MacError::BufferTooSmall)?;

        // Transmit
        self.phy.transmit(&buffer).map_err(MacError::Radio)?;

        Ok(())
    }

    /// Receive data
    pub fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, MacError<R::Error>> {
        let len = self.phy.receive(buffer).map_err(MacError::Radio)?;

        if len < MIC_SIZE + 1 {
            return Err(MacError::InvalidFrame);
        }

        // Verify MIC
        let payload = &buffer[..len - MIC_SIZE];
        let mic = &buffer[len - MIC_SIZE..len];
        
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

        // Parse frame
        let mtype = buffer[0] & 0xE0;
        match mtype {
            x if x == MType::UnconfirmedDataDown as u8 => {
                self.handle_data_down(buffer, len - MIC_SIZE)?;
            }
            x if x == MType::ConfirmedDataDown as u8 => {
                self.handle_data_down(buffer, len - MIC_SIZE)?;
                // TODO: Send ACK
            }
            x if x == MType::JoinAccept as u8 => {
                self.handle_join_accept(buffer, len - MIC_SIZE)?;
            }
            _ => return Err(MacError::InvalidFrame),
        }

        Ok(len)
    }

    /// Handle downlink data frame
    fn handle_data_down(&mut self, buffer: &[u8], len: usize) -> Result<(), MacError<R::Error>> {
        // TODO: Parse FHDR, decrypt payload, handle MAC commands
        self.session.increment_fcnt_down();
        Ok(())
    }

    /// Handle join accept
    fn handle_join_accept(&mut self, buffer: &[u8], len: usize) -> Result<(), MacError<R::Error>> {
        // TODO: Decrypt join accept, extract parameters, derive session keys
        Ok(())
    }

    /// Configure for TTN US915
    pub fn configure_for_ttn(&mut self) -> Result<(), MacError<R::Error>> {
        if let Some(us915) = (&mut self.region as *mut REG).cast::<US915>() {
            unsafe {
                (*us915).configure_ttn_us915();
            }
        }
        Ok(())
    }
}

// Add to the existing MacCommand enum
#[derive(Debug, Clone)]
pub enum MacCommand {
    // ... existing commands ...

    /// PingSlotInfoReq - Device requests ping slot parameters
    PingSlotInfoReq {
        periodicity: u8,
    },
    /// PingSlotInfoAns - Network confirms ping slot parameters
    PingSlotInfoAns,
    
    /// BeaconTimingReq - Device requests next beacon timing
    BeaconTimingReq,
    /// BeaconTimingAns - Network provides next beacon timing
    BeaconTimingAns {
        delay: u16,
        channel: u8,
    },
    
    /// BeaconFreqReq - Network configures beacon frequency
    BeaconFreqReq {
        frequency: u32,
    },
    /// BeaconFreqAns - Device confirms beacon frequency
    BeaconFreqAns {
        status: u8,
    },
}

impl MacCommand {
    // Add to the existing from_bytes function
    pub fn from_bytes(cid: u8, payload: &[u8]) -> Result<Self, MacError> {
        match cid {
            // ... existing command parsing ...

            // Class B MAC Commands
            0x10 => {
                // PingSlotInfoReq
                if payload.len() != 1 {
                    return Err(MacError::InvalidLength);
                }
                Ok(MacCommand::PingSlotInfoReq {
                    periodicity: payload[0] & 0x07,
                })
            }
            0x11 => {
                // PingSlotInfoAns
                Ok(MacCommand::PingSlotInfoAns)
            }
            0x12 => {
                // BeaconTimingReq
                Ok(MacCommand::BeaconTimingReq)
            }
            0x13 => {
                // BeaconTimingAns
                if payload.len() != 3 {
                    return Err(MacError::InvalidLength);
                }
                let delay = u16::from_le_bytes([payload[0], payload[1]]);
                Ok(MacCommand::BeaconTimingAns {
                    delay,
                    channel: payload[2],
                })
            }
            0x14 => {
                // BeaconFreqReq
                if payload.len() != 3 {
                    return Err(MacError::InvalidLength);
                }
                let freq = u32::from_le_bytes([payload[0], payload[1], payload[2], 0]);
                Ok(MacCommand::BeaconFreqReq {
                    frequency: freq * 100,
                })
            }
            0x15 => {
                // BeaconFreqAns
                if payload.len() != 1 {
                    return Err(MacError::InvalidLength);
                }
                Ok(MacCommand::BeaconFreqAns {
                    status: payload[0],
                })
            }
            _ => Err(MacError::UnknownCommand),
        }
    }

    // Add to the existing to_bytes function
    pub fn to_bytes(&self) -> (u8, Vec<u8, 16>) {
        match self {
            // ... existing command serialization ...

            // Class B MAC Commands
            MacCommand::PingSlotInfoReq { periodicity } => {
                let mut payload = Vec::new();
                payload.extend_from_slice(&[periodicity & 0x07]);
                (0x10, payload)
            }
            MacCommand::PingSlotInfoAns => (0x11, Vec::new()),
            MacCommand::BeaconTimingReq => (0x12, Vec::new()),
            MacCommand::BeaconTimingAns { delay, channel } => {
                let mut payload = Vec::new();
                payload.extend_from_slice(&delay.to_le_bytes());
                payload.extend_from_slice(&[*channel]);
                (0x13, payload)
            }
            MacCommand::BeaconFreqReq { frequency } => {
                let mut payload = Vec::new();
                let freq_bytes = (*frequency / 100).to_le_bytes();
                payload.extend_from_slice(&freq_bytes[0..3]);
                (0x14, payload)
            }
            MacCommand::BeaconFreqAns { status } => {
                let mut payload = Vec::new();
                payload.extend_from_slice(&[*status]);
                (0x15, payload)
            }
        }
    }
}

impl<R: Radio, REG: Region> MacLayer<R, REG> {
    // Add Class B command handling methods
    
    /// Handle PingSlotInfoReq command
    pub fn handle_ping_slot_info_req(&mut self, periodicity: u8) -> Result<(), MacError<R::Error>> {
        // Validate periodicity (0-7)
        if periodicity > 7 {
            return Err(MacError::InvalidValue);
        }

        // Store ping slot parameters
        // TODO: Update device ping slot configuration

        // Send answer
        self.queue_mac_command(MacCommand::PingSlotInfoAns);
        
        Ok(())
    }

    /// Handle BeaconTimingReq command
    pub fn handle_beacon_timing_req(&mut self) -> Result<(), MacError<R::Error>> {
        // Calculate time to next beacon
        // TODO: Calculate actual beacon timing

        // Send answer with next beacon timing
        self.queue_mac_command(MacCommand::BeaconTimingAns {
            delay: 0, // TODO: Calculate actual delay
            channel: 0, // TODO: Use actual beacon channel
        });
        
        Ok(())
    }

    /// Handle BeaconFreqReq command
    pub fn handle_beacon_freq_req(&mut self, frequency: u32) -> Result<(), MacError<R::Error>> {
        // Validate frequency
        if !self.region.is_valid_frequency(frequency) {
            // Reject invalid frequency
            self.queue_mac_command(MacCommand::BeaconFreqAns { status: 1 });
            return Ok(());
        }

        // Update beacon frequency
        // TODO: Store and apply new beacon frequency

        // Accept new frequency
        self.queue_mac_command(MacCommand::BeaconFreqAns { status: 0 });
        
        Ok(())
    }

    /// Process received MAC command
    pub fn process_mac_command(&mut self, command: MacCommand) -> Result<(), MacError<R::Error>> {
        match command {
            // ... existing command handling ...

            // Class B MAC Commands
            MacCommand::PingSlotInfoReq { periodicity } => {
                self.handle_ping_slot_info_req(periodicity)?;
            }
            MacCommand::BeaconTimingReq => {
                self.handle_beacon_timing_req()?;
            }
            MacCommand::BeaconFreqReq { frequency } => {
                self.handle_beacon_freq_req(frequency)?;
            }
            MacCommand::PingSlotInfoAns |
            MacCommand::BeaconTimingAns { .. } |
            MacCommand::BeaconFreqAns { .. } => {
                // These are responses to our requests, handle accordingly
                // TODO: Update device state based on responses
            }
        }
        Ok(())
    }
} 