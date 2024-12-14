# Class B and C Implementation Plan

## Class B Implementation

### 1. Beacon Synchronization
- [ ] Implement beacon frame parser
  ```rust
  struct BeaconFrame {
      time: u32,
      crc: u16,
      gwspec: u8,
      info: [u8; 7],
  }
  ```
- [ ] Add beacon timing calculation
  - Network beacon period (128 seconds)
  - Beacon reserved time (2120ms)
  - Beacon guard time (3000ms)
  - Beacon window (122880ms)
- [ ] Implement beacon acquisition state machine
  - Cold start: scan all channels
  - Warm start: use last known timing
  - Track beacon misses

### 2. Ping Slot Management
- [ ] Implement ping slot timing calculator
  ```rust
  fn calculate_ping_offset(dev_addr: u32, beacon_time: u32, ping_period: u32) -> u32
  ```
- [ ] Add ping slot scheduler
  - Support multiple ping periods (32s to 128s)
  - Handle ping slot timing drift
  - Manage colliding ping slots
- [ ] Implement ping slot state tracking
  ```rust
  struct PingSlotState {
      next_slot: u32,
      period: u32,
      frequency: u32,
      data_rate: u8,
  }
  ```

### 3. Class B MAC Commands
- [ ] PingSlotInfoReq/Ans
  - Configure ping slot parameters
  - Update ping slot timing
- [ ] BeaconTimingReq/Ans
  - Request next beacon timing
  - Handle timing responses
- [ ] BeaconFreqReq/Ans
  - Configure beacon frequency
  - Validate frequency changes

### 4. Power Management
- [ ] Optimize radio sleep between beacons
- [ ] Implement precise timing for ping slots
- [ ] Add battery level monitoring
- [ ] Handle timing drift compensation

### 5. Testing & Validation
- [ ] Beacon reception tests
- [ ] Ping slot timing accuracy tests
- [ ] Power consumption measurements
- [ ] Network compatibility tests

## Class C Implementation

### 1. Continuous Reception
- [ ] Implement RX2 window manager
  ```rust
  struct RX2Config {
      frequency: u32,
      data_rate: u8,
      continuous: bool,
  }
  ```
- [ ] Add efficient radio configuration
  - Optimize for continuous reception
  - Handle frequency/data rate changes
  - Manage power consumption
- [ ] Implement priority handling
  - RX1 vs RX2 windows
  - Uplink interruptions

### 2. Power Management
- [ ] Implement efficient sleep modes
  ```rust
  enum PowerState {
      Active,
      Sleep,
      DeepSleep,
  }
  ```
- [ ] Add battery monitoring
  - Track power consumption
  - Report battery status
  - Handle low power states
- [ ] Optimize radio settings
  - Current consumption
  - Sensitivity vs. power trade-offs
  - Temperature compensation

### 3. Multicast Support
- [ ] Add multicast group handling
  ```rust
  struct MulticastGroup {
      addr: u32,
      nwk_skey: [u8; 16],
      app_skey: [u8; 16],
  }
  ```
- [ ] Implement frame filtering
  - Multiple group addresses
  - Security key management
  - Frame counter tracking

### 4. Radio Driver Updates
- [ ] Add continuous mode support
  ```rust
  trait RadioExt: Radio {
      fn set_continuous_reception(&mut self, enabled: bool) -> Result<(), Error>;
      fn get_rssi_continuous(&self) -> Result<i32, Error>;
  }
  ```
- [ ] Implement power optimization
  - Automatic gain control
  - Dynamic sensitivity adjustment
  - Temperature compensation

### 5. Testing & Validation
- [ ] Continuous reception tests
- [ ] Power consumption analysis
- [ ] Multicast functionality tests
- [ ] Network compatibility validation

## Implementation Order

1. **Class B Phase 1**
   - Basic beacon synchronization
   - Simple ping slot timing
   - Essential MAC commands

2. **Class B Phase 2**
   - Advanced beacon handling
   - Complete ping slot management
   - Power optimization

3. **Class C Phase 1**
   - Basic continuous reception
   - RX2 window management
   - Simple power management

4. **Class C Phase 2**
   - Multicast support
   - Advanced power optimization
   - Complete radio driver updates

## Testing Strategy

1. **Unit Tests**
   - Beacon frame parsing
   - Ping slot calculations
   - MAC command handling
   - Power state management

2. **Integration Tests**
   - Network synchronization
   - End-to-end messaging
   - Power consumption patterns
   - Timing accuracy

3. **Field Testing**
   - Real network compatibility
   - Long-term stability
   - Power consumption
   - Environmental factors

## Documentation Requirements

1. **API Documentation**
   - Clear class switching methods
   - Power management interfaces
   - Configuration options
   - Error handling

2. **Example Code**
   - Basic Class B usage
   - Class C implementation
   - Power optimization
   - Multicast setup

3. **Integration Guides**
   - Network server setup
   - Gateway configuration
   - Power supply requirements
   - Antenna considerations 