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