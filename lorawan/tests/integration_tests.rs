use lorawan::{
    config::device::DeviceConfig,
    lorawan::{
        commands::{CommandHandler, DownlinkCommand},
        region::US915,
    },
    class::OperatingMode,
    device::{LoRaWANDevice, DeviceState},
};

// Import mock radio from unit tests
mod mock;
use mock::MockRadio;

// Test helper to create a device
fn create_test_device() -> LoRaWANDevice<MockRadio, US915> {
    let radio = MockRadio::new();
    let dev_eui = [0x01; 8];
    let app_eui = [0x02; 8];
    let app_key = [0x03; 16];
    let config = DeviceConfig::new_otaa(dev_eui, app_eui, app_key);
    let region = US915::new();

    LoRaWANDevice::new(radio, config, region, OperatingMode::ClassA).unwrap()
}

#[test]
fn test_device_activation() {
    // Test OTAA activation
    let mut device = create_test_device();
    assert_eq!(device.state(), DeviceState::Idle);

    device.join_otaa().unwrap();
    assert_eq!(device.state(), DeviceState::Joining);

    // Test ABP activation
    let mut device = create_test_device();
    let dev_addr = [0x01; 4];
    let nwk_skey = [0x02; 16];
    let app_skey = [0x03; 16];

    device.activate_abp(dev_addr, nwk_skey, app_skey).unwrap();
    assert_eq!(device.state(), DeviceState::Ready);
}

#[test]
fn test_uplink_transmission() {
    let mut device = create_test_device();
    
    // Activate device first
    let dev_addr = [0x01; 4];
    let nwk_skey = [0x02; 16];
    let app_skey = [0x03; 16];
    device.activate_abp(dev_addr, nwk_skey, app_skey).unwrap();

    // Send unconfirmed uplink
    let data = b"Test Data";
    device.send_uplink(1, data, false).unwrap();
    assert_eq!(device.state(), DeviceState::Ready);

    // Send confirmed uplink
    device.send_uplink(1, data, true).unwrap();
    assert_eq!(device.state(), DeviceState::Ready);
}

#[test]
fn test_class_switching() {
    let mut device = create_test_device();
    
    // Activate device first
    let dev_addr = [0x01; 4];
    let nwk_skey = [0x02; 16];
    let app_skey = [0x03; 16];
    device.activate_abp(dev_addr, nwk_skey, app_skey).unwrap();

    // Test class switching
    assert_eq!(device.operating_mode(), OperatingMode::ClassA);
    
    device.set_class(OperatingMode::ClassB).unwrap();
    assert_eq!(device.operating_mode(), OperatingMode::ClassB);
    
    device.set_class(OperatingMode::ClassC).unwrap();
    assert_eq!(device.operating_mode(), OperatingMode::ClassC);
}

#[test]
fn test_downlink_commands() {
    let mut device = create_test_device();
    
    // Activate device first
    let dev_addr = [0x01; 4];
    let nwk_skey = [0x02; 16];
    let app_skey = [0x03; 16];
    device.activate_abp(dev_addr, nwk_skey, app_skey).unwrap();

    // Test SetInterval command
    let cmd = DownlinkCommand::SetInterval(60);
    device.handle_downlink_cmd(cmd).unwrap();

    // Test ShowFirmwareVersion command
    let cmd = DownlinkCommand::ShowFirmwareVersion;
    device.handle_downlink_cmd(cmd).unwrap();

    // Test Reboot command
    let cmd = DownlinkCommand::Reboot;
    device.handle_downlink_cmd(cmd).unwrap();

    // Test Custom command
    let custom_data = vec![0x01, 0x02, 0x03];
    let cmd = DownlinkCommand::Custom(10, custom_data);
    device.handle_downlink_cmd(cmd).unwrap();
}

#[test]
fn test_device_processing() {
    let mut device = create_test_device();
    
    // Activate device first
    let dev_addr = [0x01; 4];
    let nwk_skey = [0x02; 16];
    let app_skey = [0x03; 16];
    device.activate_abp(dev_addr, nwk_skey, app_skey).unwrap();

    // Process device operations
    for _ in 0..10 {
        device.process().unwrap();
    }
}

// Note: The following test would be used with real hardware
// #[test]
// #[ignore]
// fn test_hardware_in_the_loop() {
//     // This test requires actual LoRa hardware and network server
//     // It would test:
//     // 1. Physical radio communication
//     // 2. Real OTAA join procedure
//     // 3. Actual uplink/downlink with network server
//     // 4. Real timing of receive windows
// } 