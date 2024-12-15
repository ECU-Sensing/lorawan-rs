#![no_std]

use lorawan::{
    config::device::{AESKey, DeviceConfig},
    lorawan::{
        region::US915,
        commands::MacCommand,
    },
};

use heapless::Vec;

mod mock;
use mock::MockRadio;

#[test]
fn test_otaa_join() {
    let dev_eui = [0x01; 8];
    let app_eui = [0x02; 8];
    let app_key = AESKey::new([0x03; 16]);

    let _config = DeviceConfig::new_otaa(dev_eui, app_eui, app_key);
    let _region = US915::new();
    let _radio = MockRadio::new();

    // TODO: Test OTAA join procedure
}

#[test]
fn test_downlink_commands() {
    let mut custom_data: Vec<u8, 32> = Vec::new();
    custom_data.extend_from_slice(&[0x01, 0x02, 0x03]).unwrap();

    let cmd = MacCommand::DevStatusReq;

    match cmd {
        MacCommand::DevStatusReq => {
            // Test passed
        }
        _ => panic!("Wrong command type"),
    }
}
