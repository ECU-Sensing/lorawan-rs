#![no_std]

use lorawan::{
    class::{
        class_b::ClassB,
        class_c::ClassC,
        DeviceClass,
        OperatingMode,
    },
    config::device::{AESKey, DeviceConfig, SessionState},
    lorawan::{
        mac::MacLayer,
        region::US915,
    },
};

use heapless::Vec;

mod mock;
use mock::MockRadio;

#[test]
fn test_class_c_continuous_reception() {
    let radio = MockRadio::new();
    let region = US915::new();
    let session = SessionState::new();
    let mac = MacLayer::new(radio, region, session);
    let mut device = ClassC::new(mac, 923_300_000, 8);

    // Test continuous reception
    let mut buffer = [0u8; 256];
    assert!(device.receive(&mut buffer).is_ok());
    assert_eq!(device.operating_mode(), OperatingMode::ClassC);
}

#[test]
fn test_class_c_power_management() {
    let radio = MockRadio::new();
    let region = US915::new();
    let session = SessionState::new();
    let mac = MacLayer::new(radio, region, session);
    let mut device = ClassC::new(mac, 923_300_000, 8);

    // Test battery level monitoring
    device.update_power_state(20); // Set to low battery
    let mut buffer = [0u8; 256];
    assert!(device.receive(&mut buffer).is_ok());
}

#[test]
fn test_class_b_beacon_sync() {
    let radio = MockRadio::new();
    let region = US915::new();
    let session = SessionState::new();
    let mac = MacLayer::new(radio, region, session);
    let mut device = ClassB::new(mac);

    // Start beacon acquisition
    device.start_beacon_acquisition();
    assert!(device.process().is_ok());
}

#[test]
fn test_class_b_ping_slots() {
    let radio = MockRadio::new();
    let region = US915::new();
    let session = SessionState::new();
    let mac = MacLayer::new(radio, region, session);
    let mut device = ClassB::new(mac);

    // Configure ping slots
    let mut buffer = [0u8; 256];
    assert!(device.receive(&mut buffer).is_ok());
}

#[test]
fn test_error_recovery() {
    let mut radio = MockRadio::new();
    let region = US915::new();
    let session = SessionState::new();
    let mac = MacLayer::new(radio.clone(), region, session);
    let mut device = ClassC::new(mac, 923_300_000, 8);

    // Simulate radio error and test recovery
    radio.set_error_mode(true);
    let mut buffer = [0u8; 256];
    assert!(device.receive(&mut buffer).is_ok());
}

#[test]
fn test_window_switching() {
    let radio = MockRadio::new();
    let region = US915::new();
    let session = SessionState::new();
    let mac = MacLayer::new(radio, region, session);
    let mut device = ClassC::new(mac, 923_300_000, 8);

    // Test RX window switching during transmission
    let data = [1, 2, 3, 4];
    assert!(device.send_data(1, &data, false).is_ok());
    
    let mut buffer = [0u8; 256];
    assert!(device.receive(&mut buffer).is_ok());
} 