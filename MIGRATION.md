# Migration Guide for LoRaWAN-rs Class B and C Support

This guide helps you migrate your existing LoRaWAN-rs applications to use the new Class B and C features.

## Overview of Changes

The latest version introduces comprehensive support for LoRaWAN Class B and C devices, including:
- Continuous reception for Class C
- Beacon synchronization for Class B
- Power management improvements
- Enhanced error handling
- New testing utilities

## Step-by-Step Migration

### 1. Update Dependencies

Update your `Cargo.toml`:
```toml
[dependencies]
lorawan-rs = "0.2.0"  # Or latest version with Class B/C support
```

### 2. Class A to Class C Migration

If you're upgrading from Class A to Class C:

```rust
// Old Class A code
let mut device = LoRaWANDevice::new(
    radio,
    config,
    region,
    OperatingMode::ClassA,
)?;

// New Class C code
let mut device = LoRaWANDevice::new(
    radio,
    config,
    region,
    OperatingMode::ClassC,
)?;

// Optional: Configure power management
device.update_power_state(battery_level);
```

### 3. Class A to Class B Migration

For Class B support:

```rust
// Initialize as Class B device
let mut device = LoRaWANDevice::new(
    radio,
    config,
    region,
    OperatingMode::ClassB,
)?;

// Start beacon synchronization
if let Some(class_b) = device.as_class_b_mut() {
    class_b.start_beacon_acquisition();
}
```

### 4. Power Management Integration

The new power management system provides better battery monitoring:

```rust
use lorawan::device::power::{PowerConfig, PowerManager};

// Create power manager
let config = PowerConfig::default();
let mut power_manager = PowerManager::new(config);

// Update battery status
let power_state = power_manager.update_battery(battery_level);
if power_state.is_battery_critical() {
    // Handle critical battery
}

// Record operations
power_manager.record_tx(duration);
power_manager.record_rx(duration);
```

### 5. Error Handling Updates

The error handling system now includes automatic recovery:

```rust
match device.process() {
    Ok(_) => {
        // Normal operation
    }
    Err(DeviceError::Radio(e)) => {
        // Radio errors are now handled automatically
        // but you can still implement custom recovery
    }
    Err(e) => {
        // Handle other errors
    }
}
```

### 6. Testing Updates

Update your tests to use the new testing utilities:

```rust
use lorawan::tests::mock::MockRadio;

#[test]
fn test_class_c_reception() {
    let radio = MockRadio::new();
    let mut device = ClassC::new(/* ... */);
    
    // Test continuous reception
    assert!(device.process().is_ok());
}
```

## Breaking Changes

1. The `Radio` trait now requires `Clone`
2. Power management is now mandatory for Class C devices
3. Session state handling has been updated
4. Error types have been expanded

## Best Practices

1. **Power Management**
   - Always monitor battery levels for Class C devices
   - Use power saving mode when battery is low
   - Track duty cycle to comply with regulations

2. **Class B Operation**
   - Maintain beacon synchronization
   - Handle ping slot timing carefully
   - Monitor beacon loss events

3. **Class C Operation**
   - Implement proper error recovery
   - Monitor power consumption
   - Handle RX window switching correctly

## Common Issues

1. **High Power Consumption**
   - Enable power saving mode when battery is low
   - Use duty cycle management
   - Monitor RX windows carefully

2. **Beacon Synchronization**
   - Ensure proper timing configuration
   - Handle beacon loss gracefully
   - Use proper region settings

3. **Memory Usage**
   - Class C devices use more memory for continuous reception
   - Monitor stack usage
   - Use appropriate buffer sizes

## Need Help?

- Check the [examples](examples/) directory
- Read the [documentation](https://docs.rs/lorawan-rs)
- Open an [issue](https://github.com/user/lorawan-rs/issues)
- Join our [Discord](https://discord.gg/your-invite-here) 