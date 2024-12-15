#[test]
fn test_abp_activation() {
    let dev_addr = DevAddr::new([0x01; 4]);
    let nwk_skey = AESKey::new([0x01; 16]);
    let app_skey = AESKey::new([0x02; 16]);

    // ... rest of the test ...
}

#[test]
fn test_mic_computation() {
    let key = AESKey::new([0x01; 16]);
    let dev_addr = DevAddr::new([0x02; 4]);
    let data = [0x03; 32];

    // ... rest of the test ...
}

#[test]
fn test_payload_encryption() {
    let key = AESKey::new([0x01; 16]);
    let dev_addr = DevAddr::new([0x02; 4]);
    let data = [0x03; 32];

    // ... rest of the test ...
}

#[test]
fn test_join_request() {
    let app_key = AESKey::new([0x01; 16]);
    let dev_eui = [0x02; 8];
    let app_eui = [0x03; 8];

    // ... rest of the test ...
}

#[test]
fn test_session_state() {
    let session = SessionState {
        activation_state: ActivationState::ABPActivated,
        device_class: DeviceClass::A,
        dev_addr: DevAddr::new([0x01; 4]),
        nwk_skey: AESKey::new([0x01; 16]),
        app_skey: AESKey::new([0x02; 16]),
        fcnt_up: 0,
        fcnt_down: 0,
        dev_nonce: 0,
    };

    // ... rest of the test ...
} 