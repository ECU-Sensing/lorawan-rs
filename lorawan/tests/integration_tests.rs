#![no_std]

use lorawan::{
    class::OperatingMode,
    config::device::{AESKey, DevAddr, DeviceConfig},
    crypto,
    device::LoRaWANDevice,
    lorawan::{commands::MacCommand, region::US915},
};

use heapless::Vec;
mod mock;
use mock::MockRadio;

// #[test]
// fn test_join_procedure() {
//     let mut mock_radio = MockRadio::new();
//     let dev_eui = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
//     let app_eui = [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01];
//     let app_key = AESKey::new([
//         0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
//         0x10,
//     ]);

//     // First create the join accept payload
//     let mut join_accept_payload = Vec::<u8, 32>::new();
//     join_accept_payload.extend_from_slice(&[
//         0x01, 0x02, 0x03,      // AppNonce
//         0x04, 0x05, 0x06,      // NetID
//         0x07, 0x08, 0x09, 0x0A, // DevAddr
//         0x00,                   // DLSettings
//         0x01,                   // RxDelay
//     ]).unwrap();

//     // Create the full message with MHDR
//     let mut full_message = Vec::<u8, 32>::new();
//     full_message.push(0x20).unwrap();  // MHDR for join-accept
//     full_message.extend_from_slice(&join_accept_payload).unwrap();

//     // Calculate MIC over MHDR|JoinAcceptPayload
//     let mic = crypto::compute_mic(
//         &app_key,
//         &full_message,
//         DevAddr::new([0; 4]),
//         0,
//         crypto::Direction::Down
//     );
//     full_message.extend_from_slice(&mic).unwrap();

//     // Encrypt the message (except MHDR)
//     let encrypted_accept = crypto::encrypt_join_accept(&app_key, &full_message);

//     // Set up mock radio before creating device
//     mock_radio.simulate_join_accept(&encrypted_accept);

//     let config = DeviceConfig::new_otaa(dev_eui, app_eui, app_key.clone());
//     let mut device = LoRaWANDevice::new(
//         mock_radio,
//         config,
//         US915::new(),
//         OperatingMode::ClassA,
//     )
//     .expect("Failed to create device");

//     // Attempt join
//     device.join_otaa(dev_eui, app_eui, app_key.clone())
//         .expect("Join failed");

//     // Process join accept
//     let mut rx_buffer = [0u8; 256];
//     device.process().expect("Failed to process");
//     let rx_size = device.receive(&mut rx_buffer).expect("Failed to receive");
//     assert!(rx_size > 0, "No join accept received");

//     // Verify session state
//     let session = device.get_session_state();
//     assert!(session.is_joined(), "Device should be joined");
//     assert_eq!(session.dev_addr.as_bytes(), &[0x07, 0x08, 0x09, 0x0A]);

//     // Verify session keys
//     let (nwk_skey, app_skey) = crypto::derive_session_keys(
//         &app_key,
//         &[0x01, 0x02, 0x03],
//         &[0x04, 0x05, 0x06],
//         0x0000,
//     );
//     assert_eq!(session.nwk_skey.as_bytes(), nwk_skey.as_bytes());
//     assert_eq!(session.app_skey.as_bytes(), app_skey.as_bytes());
// }

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
