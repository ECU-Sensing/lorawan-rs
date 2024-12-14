use lorawan::{
    config::device::{DeviceConfig, SessionState},
    crypto,
    lorawan::{
        commands::{CommandHandler, DownlinkCommand},
        region::{DataRate, Region, US915},
    },
};

// Mock radio for testing
mod mock {
    use lorawan::radio::traits::{ModulationParams, Radio, RxConfig, TxConfig};

    pub struct MockRadio {
        pub last_tx_data: Vec<u8>,
        pub next_rx_data: Vec<u8>,
    }

    impl MockRadio {
        pub fn new() -> Self {
            Self {
                last_tx_data: Vec::new(),
                next_rx_data: Vec::new(),
            }
        }
    }

    impl Radio for MockRadio {
        type Error = ();

        fn init(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }

        fn set_frequency(&mut self, _freq: u32) -> Result<(), Self::Error> {
            Ok(())
        }

        fn set_tx_power(&mut self, _power: i8) -> Result<(), Self::Error> {
            Ok(())
        }

        fn transmit(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
            self.last_tx_data = buffer.to_vec();
            Ok(())
        }

        fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
            let len = self.next_rx_data.len().min(buffer.len());
            buffer[..len].copy_from_slice(&self.next_rx_data[..len]);
            Ok(len)
        }

        fn configure_tx(&mut self, _config: TxConfig) -> Result<(), Self::Error> {
            Ok(())
        }

        fn configure_rx(&mut self, _config: RxConfig) -> Result<(), Self::Error> {
            Ok(())
        }

        fn is_receiving(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }

        fn get_rssi(&mut self) -> Result<i16, Self::Error> {
            Ok(-60)
        }

        fn get_snr(&mut self) -> Result<i8, Self::Error> {
            Ok(10)
        }

        fn sleep(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }

        fn standby(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }

        fn is_transmitting(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
    }
}

#[test]
fn test_us915_channel_plan() {
    let region = US915::new();

    // Test default channels
    let enabled_channels: Vec<_> = region.enabled_channels().collect();
    assert_eq!(enabled_channels.len(), 72); // 64 125kHz + 8 500kHz channels

    // Test sub-band selection
    let mut region = US915::new();
    region.set_sub_band(2);
    let enabled_channels: Vec<_> = region.enabled_channels().collect();
    assert_eq!(enabled_channels.len(), 8); // 8 channels per sub-band
    assert!(enabled_channels.iter().all(|c| c.enabled));

    // Test data rates
    assert_eq!(region.data_rate(), DataRate::SF10BW125); // Default DR
    region.set_data_rate(DataRate::SF7BW125);
    assert_eq!(region.data_rate(), DataRate::SF7BW125);
}

#[test]
fn test_aes_encryption() {
    let key = [0x2B; 16];
    let dev_addr = [0x01, 0x02, 0x03, 0x04];
    let fcnt = 1;
    let payload = b"Hello LoRaWAN";

    // Test encryption
    let encrypted = crypto::encrypt_payload(
        &key,
        dev_addr,
        fcnt,
        crypto::Direction::Up,
        payload,
    );

    // Test decryption
    let decrypted = crypto::encrypt_payload(
        &key,
        dev_addr,
        fcnt,
        crypto::Direction::Up,
        &encrypted,
    );

    assert_eq!(&decrypted[..], payload);
}

#[test]
fn test_mic_calculation() {
    let key = [0x2B; 16];
    let dev_addr = [0x01, 0x02, 0x03, 0x04];
    let fcnt = 1;
    let data = b"Test Data";

    let mic = crypto::compute_mic(
        &key,
        data,
        dev_addr,
        fcnt,
        crypto::Direction::Up,
    );

    assert_eq!(mic.len(), 4);
}

#[test]
fn test_join_request_mic() {
    let app_key = [0x2B; 16];
    let data = b"Join Request Data";

    let mic = crypto::compute_join_request_mic(&app_key, data);
    assert_eq!(mic.len(), 4);
}

#[test]
fn test_session_key_generation() {
    let app_key = [0x2B; 16];
    let app_nonce = [0x01, 0x02, 0x03];
    let net_id = [0x04, 0x05, 0x06];
    let dev_nonce = 0x0708;

    let (nwk_skey, app_skey) = crypto::generate_session_keys(
        &app_key,
        &app_nonce,
        &net_id,
        dev_nonce,
    );

    assert_eq!(nwk_skey.len(), 16);
    assert_eq!(app_skey.len(), 16);
}

#[test]
fn test_downlink_commands() {
    // Test command parsing
    let data = [0x01, 0x00, 0x00, 0x00, 0x3C]; // SetInterval(60)
    let cmd = DownlinkCommand::from_bytes(224, &data).unwrap();
    match cmd {
        DownlinkCommand::SetInterval(interval) => assert_eq!(interval, 60),
        _ => panic!("Wrong command type"),
    }

    // Test command serialization
    let cmd = DownlinkCommand::SetInterval(60);
    let (port, data) = cmd.to_bytes().unwrap();
    assert_eq!(port, 224);
    assert_eq!(&data[..], &[0x01, 0x00, 0x00, 0x00, 0x3C]);
}

#[test]
fn test_device_config() {
    let dev_eui = [0x01; 8];
    let app_eui = [0x02; 8];
    let app_key = [0x03; 16];

    // Test OTAA config
    let config = DeviceConfig::new_otaa(dev_eui, app_eui, app_key);
    assert_eq!(config.dev_eui, dev_eui);
    assert_eq!(config.app_eui, app_eui);
    assert_eq!(config.app_key, app_key);
    assert!(config.dev_addr.is_none());

    // Test ABP config
    let dev_addr = [0x04; 4];
    let nwk_skey = [0x05; 16];
    let app_skey = [0x06; 16];
    let config = DeviceConfig::new_abp(dev_eui, app_eui, dev_addr, nwk_skey, app_skey);
    assert_eq!(config.dev_eui, dev_eui);
    assert_eq!(config.app_eui, app_eui);
    assert_eq!(config.dev_addr, Some(dev_addr));
    assert_eq!(config.nwk_skey, Some(nwk_skey));
    assert_eq!(config.app_skey, Some(app_skey));
}

#[test]
fn test_session_state() {
    let dev_addr = [0x01; 4];
    let nwk_skey = [0x02; 16];
    let app_skey = [0x03; 16];

    // Test ABP session
    let mut session = SessionState::new_abp(dev_addr, nwk_skey, app_skey);
    assert_eq!(session.dev_addr, dev_addr);
    assert_eq!(session.nwk_skey, nwk_skey);
    assert_eq!(session.app_skey, app_skey);
    assert_eq!(session.fcnt_up, 0);
    assert_eq!(session.fcnt_down, 0);

    // Test frame counter increment
    session.increment_fcnt_up();
    session.increment_fcnt_down();
    assert_eq!(session.fcnt_up, 1);
    assert_eq!(session.fcnt_down, 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        class::{class_b::*, class_c::*, DeviceClass, OperatingMode},
        lorawan::mac::{MacCommand, MacError},
        radio::*,
    };

    // Mock radio implementation for testing
    struct MockRadio {
        last_frequency: u32,
        last_power: i8,
        continuous_mode: bool,
    }

    impl MockRadio {
        fn new() -> Self {
            Self {
                last_frequency: 0,
                last_power: 0,
                continuous_mode: false,
            }
        }
    }

    impl Radio for MockRadio {
        type Error = ();

        fn init(&mut self) -> Result<(), Self::Error> { Ok(()) }
        fn set_frequency(&mut self, freq: u32) -> Result<(), Self::Error> {
            self.last_frequency = freq;
            Ok(())
        }
        fn set_tx_power(&mut self, power: i8) -> Result<(), Self::Error> {
            self.last_power = power;
            Ok(())
        }
        // Implement other required methods...
    }

    impl RadioExt for MockRadio {
        fn set_continuous_reception(&mut self, enabled: bool) -> Result<(), Self::Error> {
            self.continuous_mode = enabled;
            Ok(())
        }
        fn get_rssi_continuous(&self) -> Result<i32, Self::Error> {
            Ok(-60)
        }
        // Implement other required methods...
    }

    #[test]
    fn test_beacon_frame_parsing() {
        let beacon_data = [
            0x00, 0x00, 0x00, 0x01, // time
            0xAA, 0xBB,             // CRC
            0x01,                   // gwspec
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, // info
        ];

        let frame = BeaconFrame::parse(&beacon_data).unwrap();
        assert_eq!(frame.time, 1);
        assert_eq!(frame.crc, 0xAABB);
        assert_eq!(frame.gwspec, 0x01);
        assert_eq!(frame.info, [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]);
    }

    #[test]
    fn test_ping_slot_calculation() {
        let calculator = PingSlotCalculator::new(0x12345678, 100, 32);
        let offset = calculator.calculate_ping_offset();
        assert!(offset < 32 * BEACON_SLOT_LEN);
    }

    #[test]
    fn test_class_b_state_machine() {
        let mock_radio = MockRadio::new();
        let mac = MacLayer::new(mock_radio, Region::new(), SessionState::new());
        let mut class_b = ClassB::new(mac);

        // Test cold start
        class_b.start_beacon_acquisition();
        assert_eq!(class_b.beacon_state, BeaconState::ColdStart);

        // Test warm start
        class_b.update_time(Duration::from_secs(1));
        class_b.last_beacon = Some(Duration::from_secs(0));
        class_b.start_beacon_acquisition();
        assert_eq!(class_b.beacon_state, BeaconState::WarmStart);
    }

    #[test]
    fn test_class_c_continuous_reception() {
        let mock_radio = MockRadio::new();
        let mac = MacLayer::new(mock_radio, Region::new(), SessionState::new());
        let mut class_c = ClassC::new(mac, 915_000_000, 8);

        // Test continuous reception
        assert_eq!(class_c.power_state, PowerState::Active);
        assert!(class_c.rx2_config.continuous);

        // Test power state changes
        class_c.set_power_state(PowerState::Sleep).unwrap();
        assert_eq!(class_c.power_state, PowerState::Sleep);
        assert!(!class_c.rx2_config.continuous);
    }

    #[test]
    fn test_class_b_mac_commands() {
        // Test PingSlotInfoReq
        let cmd = MacCommand::PingSlotInfoReq { periodicity: 3 };
        let (cid, payload) = cmd.to_bytes();
        assert_eq!(cid, 0x10);
        assert_eq!(payload[0], 3);

        // Test BeaconTimingReq
        let cmd = MacCommand::BeaconTimingReq;
        let (cid, payload) = cmd.to_bytes();
        assert_eq!(cid, 0x12);
        assert!(payload.is_empty());

        // Test BeaconFreqReq
        let cmd = MacCommand::BeaconFreqReq { frequency: 915_000_000 };
        let (cid, payload) = cmd.to_bytes();
        assert_eq!(cid, 0x14);
        assert_eq!(payload.len(), 3);
    }

    #[test]
    fn test_class_c_power_management() {
        let mock_radio = MockRadio::new();
        let mac = MacLayer::new(mock_radio, Region::new(), SessionState::new());
        let mut class_c = ClassC::new(mac, 915_000_000, 8);

        // Test battery level reporting
        class_c.update_battery_level(128);
        assert_eq!(class_c.get_battery_level(), 128);

        // Test power state transitions
        class_c.set_power_state(PowerState::DeepSleep).unwrap();
        assert_eq!(class_c.power_state, PowerState::DeepSleep);
        assert!(!class_c.rx2_config.continuous);

        class_c.set_power_state(PowerState::Active).unwrap();
        assert_eq!(class_c.power_state, PowerState::Active);
        assert!(class_c.rx2_config.continuous);
    }

    #[test]
    fn test_class_b_beacon_acquisition() {
        let mock_radio = MockRadio::new();
        let mac = MacLayer::new(mock_radio, Region::new(), SessionState::new());
        let mut class_b = ClassB::new(mac);

        // Test cold start beacon acquisition
        class_b.start_beacon_acquisition();
        assert_eq!(class_b.beacon_state, BeaconState::ColdStart);

        // Simulate beacon reception
        let beacon_data = [
            0x00, 0x00, 0x00, 0x01, // time
            0xAA, 0xBB,             // CRC
            0x01,                   // gwspec
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, // info
        ];
        let beacon = BeaconFrame::parse(&beacon_data).unwrap();
        class_b.handle_beacon(beacon).unwrap();
        assert_eq!(class_b.beacon_state, BeaconState::Synchronized);

        // Test beacon loss detection
        for _ in 0..3 {
            class_b.ping_slot.record_missed_slot();
        }
        class_b.process().unwrap();
        assert_eq!(class_b.beacon_state, BeaconState::Lost);
    }

    #[test]
    fn test_class_b_ping_slot_timing() {
        let mock_radio = MockRadio::new();
        let mac = MacLayer::new(mock_radio, Region::new(), SessionState::new());
        let mut class_b = ClassB::new(mac);

        // Configure ping slot
        let ping_slot = PingSlotState::new(32, 915_000_000, 8);
        assert_eq!(ping_slot.period, 32);
        assert_eq!(ping_slot.frequency, 915_000_000);
        assert_eq!(ping_slot.data_rate, 8);

        // Test ping slot calculation
        let calculator = PingSlotCalculator::new(0x12345678, 100, 32);
        let offset = calculator.calculate_ping_offset();
        assert!(offset < 32 * BEACON_SLOT_LEN);
        assert_eq!(offset % BEACON_SLOT_LEN, 0);
    }

    #[test]
    fn test_class_c_power_transitions() {
        let mock_radio = MockRadio::new();
        let mac = MacLayer::new(mock_radio, Region::new(), SessionState::new());
        let mut class_c = ClassC::new(mac, 915_000_000, 8);

        // Test initial state
        assert_eq!(class_c.power_state, PowerState::Active);
        assert!(class_c.rx2_config.continuous);

        // Test sleep transition
        class_c.set_power_state(PowerState::Sleep).unwrap();
        assert_eq!(class_c.power_state, PowerState::Sleep);
        assert!(!class_c.rx2_config.continuous);

        // Test deep sleep transition
        class_c.set_power_state(PowerState::DeepSleep).unwrap();
        assert_eq!(class_c.power_state, PowerState::DeepSleep);
        assert!(!class_c.rx2_config.continuous);

        // Test wake up
        class_c.set_power_state(PowerState::Active).unwrap();
        assert_eq!(class_c.power_state, PowerState::Active);
        assert!(class_c.rx2_config.continuous);
    }

    #[test]
    fn test_class_c_rx_windows() {
        let mock_radio = MockRadio::new();
        let mac = MacLayer::new(mock_radio, Region::new(), SessionState::new());
        let mut class_c = ClassC::new(mac, 915_000_000, 8);

        // Test RX1 window timing
        class_c.last_uplink = Some(Duration::from_secs(0));
        class_c.current_time = Duration::from_millis(100);
        class_c.process().unwrap();
        // Should be in RX1 window (within 500ms)
        assert!(class_c.rx2_config.continuous == false);

        // Test RX2 continuous mode
        class_c.current_time = Duration::from_millis(1000);
        class_c.process().unwrap();
        // Should be back in continuous RX2
        assert!(class_c.rx2_config.continuous == true);
    }

    #[test]
    fn test_class_b_mac_command_handling() {
        // Test PingSlotInfoReq
        let cmd = MacCommand::PingSlotInfoReq { periodicity: 3 };
        let (cid, payload) = cmd.to_bytes();
        assert_eq!(cid, 0x10);
        assert_eq!(payload[0], 3);

        // Test BeaconFreqReq
        let cmd = MacCommand::BeaconFreqReq { frequency: 915_000_000 };
        let (cid, payload) = cmd.to_bytes();
        assert_eq!(cid, 0x14);
        let freq = u32::from_le_bytes([payload[0], payload[1], payload[2], 0]) * 100;
        assert_eq!(freq, 915_000_000);

        // Test BeaconFreqAns
        let cmd = MacCommand::BeaconFreqAns { status: 0 };
        let (cid, payload) = cmd.to_bytes();
        assert_eq!(cid, 0x15);
        assert_eq!(payload[0], 0);
    }
} 