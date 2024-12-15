use crate::lorawan::mac::MacError;

/// MAC command identifiers
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum CommandIdentifier {
    LinkCheckReq = 0x02,
    LinkCheckAns = 0x82,
    LinkADRReq = 0x03,
    LinkADRAns = 0x83,
    DutyCycleReq = 0x04,
    DutyCycleAns = 0x84,
    RXParamSetupReq = 0x05,
    RXParamSetupAns = 0x85,
    DevStatusReq = 0x06,
    DevStatusAns = 0x86,
    NewChannelReq = 0x07,
    NewChannelAns = 0x87,
    RXTimingSetupReq = 0x08,
    RXTimingSetupAns = 0x88,
    TxParamSetupReq = 0x09,
    TxParamSetupAns = 0x89,
    DlChannelReq = 0x0A,
    DlChannelAns = 0x8A,
}

/// MAC command
#[derive(Debug, Clone)]
pub enum MacCommand {
    /// Link check request
    LinkCheckReq,
    /// Link check answer
    LinkCheckAns {
        /// Link margin in dB
        margin: u8,
        /// Number of gateways that received the request
        gateway_count: u8,
    },
    /// Link ADR request
    LinkADRReq {
        /// Data rate
        data_rate: u8,
        /// TX power
        tx_power: u8,
        /// Channel mask
        ch_mask: u16,
        /// Channel mask control
        ch_mask_cntl: u8,
        /// Number of transmissions
        nb_trans: u8,
    },
    /// Link ADR answer
    LinkADRAns {
        /// Power ACK
        power_ack: bool,
        /// Data rate ACK
        data_rate_ack: bool,
        /// Channel mask ACK
        channel_mask_ack: bool,
    },
    /// Duty cycle request
    DutyCycleReq {
        /// Maximum duty cycle
        max_duty_cycle: u8,
    },
    /// Duty cycle answer
    DutyCycleAns,
    /// RX parameter setup request
    RXParamSetupReq {
        /// RX1 data rate offset
        rx1_dr_offset: u8,
        /// RX2 data rate
        rx2_data_rate: u8,
        /// RX2 frequency
        freq: u32,
    },
    /// RX parameter setup answer
    RXParamSetupAns {
        /// RX1 data rate offset ACK
        rx1_dr_offset_ack: bool,
        /// RX2 data rate ACK
        rx2_data_rate_ack: bool,
        /// Channel ACK
        channel_ack: bool,
    },
    /// Device status request
    DevStatusReq,
    /// Device status answer
    DevStatusAns {
        /// Battery level (0 = external power, 1-254 = level, 255 = unknown)
        battery: u8,
        /// Radio status (margin in dB)
        margin: i8,
    },
    /// New channel request
    NewChannelReq {
        /// Channel index
        ch_index: u8,
        /// Frequency
        freq: u32,
        /// Maximum data rate
        max_dr: u8,
        /// Minimum data rate
        min_dr: u8,
    },
    /// New channel answer
    NewChannelAns {
        /// Channel frequency OK
        channel_freq_ok: bool,
        /// Data rate OK
        data_rate_ok: bool,
    },
    /// RX timing setup request
    RXTimingSetupReq {
        /// Delay (0-15)
        delay: u8,
    },
    /// RX timing setup answer
    RXTimingSetupAns,
    /// TX parameter setup request (not implemented in most regions)
    TxParamSetupReq {
        /// Downlink dwell time
        downlink_dwell_time: bool,
        /// Uplink dwell time
        uplink_dwell_time: bool,
        /// Maximum EIRP
        max_eirp: u8,
    },
    /// TX parameter setup answer
    TxParamSetupAns,
    /// Downlink channel request (not implemented in most regions)
    DlChannelReq {
        /// Channel index
        ch_index: u8,
        /// Frequency
        freq: u32,
    },
    /// Downlink channel answer
    DlChannelAns {
        /// Channel frequency OK
        channel_freq_ok: bool,
        /// Uplink frequency exists
        uplink_freq_exists: bool,
    },
}

impl MacCommand {
    /// Parse MAC command from bytes
    pub fn from_bytes(cid: u8, payload: &[u8]) -> Option<Self> {
        match cid {
            0x02 => Some(MacCommand::LinkCheckReq),
            0x82 if payload.len() >= 2 => Some(MacCommand::LinkCheckAns {
                margin: payload[0],
                gateway_count: payload[1],
            }),
            0x03 if payload.len() >= 4 => Some(MacCommand::LinkADRReq {
                data_rate: payload[0] >> 4,
                tx_power: payload[0] & 0x0F,
                ch_mask: u16::from_le_bytes([payload[1], payload[2]]),
                ch_mask_cntl: payload[3] >> 4,
                nb_trans: payload[3] & 0x0F,
            }),
            0x83 if payload.len() >= 1 => Some(MacCommand::LinkADRAns {
                power_ack: (payload[0] & 0x04) != 0,
                data_rate_ack: (payload[0] & 0x02) != 0,
                channel_mask_ack: (payload[0] & 0x01) != 0,
            }),
            0x04 if payload.len() >= 1 => Some(MacCommand::DutyCycleReq {
                max_duty_cycle: payload[0],
            }),
            0x84 => Some(MacCommand::DutyCycleAns),
            0x05 if payload.len() >= 4 => Some(MacCommand::RXParamSetupReq {
                rx1_dr_offset: payload[0] >> 4,
                rx2_data_rate: payload[0] & 0x0F,
                freq: u32::from_le_bytes([payload[1], payload[2], payload[3], 0]),
            }),
            0x85 if payload.len() >= 1 => Some(MacCommand::RXParamSetupAns {
                rx1_dr_offset_ack: (payload[0] & 0x04) != 0,
                rx2_data_rate_ack: (payload[0] & 0x02) != 0,
                channel_ack: (payload[0] & 0x01) != 0,
            }),
            0x06 => Some(MacCommand::DevStatusReq),
            0x86 if payload.len() >= 2 => Some(MacCommand::DevStatusAns {
                battery: payload[0],
                margin: payload[1] as i8,
            }),
            0x07 if payload.len() >= 5 => Some(MacCommand::NewChannelReq {
                ch_index: payload[0],
                freq: u32::from_le_bytes([payload[1], payload[2], payload[3], 0]),
                max_dr: payload[4] >> 4,
                min_dr: payload[4] & 0x0F,
            }),
            0x87 if payload.len() >= 1 => Some(MacCommand::NewChannelAns {
                channel_freq_ok: (payload[0] & 0x02) != 0,
                data_rate_ok: (payload[0] & 0x01) != 0,
            }),
            0x08 if payload.len() >= 1 => Some(MacCommand::RXTimingSetupReq {
                delay: payload[0] & 0x0F,
            }),
            0x88 => Some(MacCommand::RXTimingSetupAns),
            0x09 if payload.len() >= 1 => Some(MacCommand::TxParamSetupReq {
                downlink_dwell_time: (payload[0] & 0x20) != 0,
                uplink_dwell_time: (payload[0] & 0x10) != 0,
                max_eirp: payload[0] & 0x0F,
            }),
            0x89 => Some(MacCommand::TxParamSetupAns),
            0x0A if payload.len() >= 4 => Some(MacCommand::DlChannelReq {
                ch_index: payload[0],
                freq: u32::from_le_bytes([payload[1], payload[2], payload[3], 0]),
            }),
            0x8A if payload.len() >= 1 => Some(MacCommand::DlChannelAns {
                channel_freq_ok: (payload[0] & 0x02) != 0,
                uplink_freq_exists: (payload[0] & 0x01) != 0,
            }),
            _ => None,
        }
    }

    /// Get command length in bytes
    pub fn len(&self) -> usize {
        match self {
            MacCommand::LinkCheckReq => 0,
            MacCommand::LinkCheckAns { .. } => 2,
            MacCommand::LinkADRReq { .. } => 4,
            MacCommand::LinkADRAns { .. } => 1,
            MacCommand::DutyCycleReq { .. } => 1,
            MacCommand::DutyCycleAns => 0,
            MacCommand::RXParamSetupReq { .. } => 4,
            MacCommand::RXParamSetupAns { .. } => 1,
            MacCommand::DevStatusReq => 0,
            MacCommand::DevStatusAns { .. } => 2,
            MacCommand::NewChannelReq { .. } => 5,
            MacCommand::NewChannelAns { .. } => 1,
            MacCommand::RXTimingSetupReq { .. } => 1,
            MacCommand::RXTimingSetupAns => 0,
            MacCommand::TxParamSetupReq { .. } => 1,
            MacCommand::TxParamSetupAns => 0,
            MacCommand::DlChannelReq { .. } => 4,
            MacCommand::DlChannelAns { .. } => 1,
        }
    }

    /// Process command with error handling
    pub fn process<E>(&self) -> Result<Option<MacCommand>, MacError<E>> {
        match self {
            MacCommand::LinkCheckReq => Ok(None),
            MacCommand::LinkCheckAns { margin: _, gateway_count: _ } => Ok(None),
            MacCommand::LinkADRReq { data_rate, tx_power, ch_mask: _, ch_mask_cntl: _, nb_trans: _ } => {
                // Validate parameters before processing
                if *data_rate > 15 || *tx_power > 15 {
                    return Err(MacError::InvalidValue);
                }
                
                Ok(Some(MacCommand::LinkADRAns {
                    power_ack: true,
                    data_rate_ack: true,
                    channel_mask_ack: true,
                }))
            },
            MacCommand::DutyCycleReq { max_duty_cycle: _ } => Ok(Some(MacCommand::DutyCycleAns)),
            MacCommand::RXParamSetupReq { rx1_dr_offset, rx2_data_rate, freq: _ } => {
                // Validate parameters
                if *rx1_dr_offset > 7 || *rx2_data_rate > 15 {
                    return Err(MacError::InvalidValue);
                }
                
                Ok(Some(MacCommand::RXParamSetupAns {
                    rx1_dr_offset_ack: true,
                    rx2_data_rate_ack: true,
                    channel_ack: true,
                }))
            },
            MacCommand::DevStatusReq => {
                Ok(Some(MacCommand::DevStatusAns {
                    battery: 255, // Unknown by default
                    margin: 0,   // 0 dB by default
                }))
            },
            MacCommand::NewChannelReq { ch_index: _, freq: _, max_dr, min_dr } => {
                // Validate parameters
                if *max_dr > 15 || *min_dr > 15 || *min_dr > *max_dr {
                    return Err(MacError::InvalidValue);
                }
                
                Ok(Some(MacCommand::NewChannelAns {
                    channel_freq_ok: true,
                    data_rate_ok: true,
                }))
            },
            MacCommand::RXTimingSetupReq { delay } => {
                if *delay > 15 {
                    return Err(MacError::InvalidValue);
                }
                Ok(Some(MacCommand::RXTimingSetupAns))
            },
            MacCommand::TxParamSetupReq { downlink_dwell_time: _, uplink_dwell_time: _, max_eirp: _ } => {
                // Not implemented in most regions
                Err(MacError::UnknownCommand)
            },
            MacCommand::DlChannelReq { ch_index: _, freq: _ } => {
                // Not implemented in most regions
                Err(MacError::UnknownCommand)
            },
            MacCommand::LinkADRAns { .. } |
            MacCommand::DutyCycleAns |
            MacCommand::RXParamSetupAns { .. } |
            MacCommand::DevStatusAns { .. } |
            MacCommand::NewChannelAns { .. } |
            MacCommand::RXTimingSetupAns |
            MacCommand::TxParamSetupAns |
            MacCommand::DlChannelAns { .. } => {
                // These are answers, not requests - they don't need processing
                Ok(None)
            },
        }
    }
}
