//! LoRaWAN Class B Network Time Synchronization
//!
//! This module handles network time synchronization including:
//! - Time offset calculation and tracking
//! - GPS time conversion
//! - Drift compensation

/// GPS epoch offset from Unix epoch (seconds)
const GPS_EPOCH_OFFSET: u32 = 315964800;

/// Network time synchronization
#[derive(Debug)]
pub struct NetworkTime {
    /// Local time offset from network time (milliseconds)
    time_offset: i32,
    /// Accumulated timing error (microseconds)
    timing_error: i32,
    /// Clock drift compensation (ppm)
    drift_compensation: i32,
    /// Last synchronization time
    last_sync: u32,
}

impl NetworkTime {
    /// Create new network time synchronization
    pub fn new() -> Self {
        Self {
            time_offset: 0,
            timing_error: 0,
            drift_compensation: 0,
            last_sync: 0,
        }
    }

    /// Update network time from beacon
    pub fn update(&mut self, beacon_time: u32) {
        // For first update, just store the time
        if self.last_sync == 0 {
            self.last_sync = beacon_time;
            return;
        }

        // Calculate time since last sync
        let time_delta = beacon_time.wrapping_sub(self.last_sync);
        let expected_time = self.last_sync.wrapping_add(128_000);
        let actual_error = beacon_time.wrapping_sub(expected_time);
        let error_ms = actual_error as i32;

        // Update timing error with exponential moving average
        self.timing_error = (self.timing_error * 7 + error_ms * 1000) / 8;

        // Calculate drift compensation in parts per million
        self.drift_compensation = (error_ms * 1_000_000) / time_delta as i32;

        // Store current time for next update
        self.last_sync = beacon_time;
    }

    /// Get current network time
    pub fn current_time(&self) -> u32 {
        let local_time = self.get_local_time();
        let time_since_sync = local_time.wrapping_sub(self.last_sync);

        // Apply drift compensation
        let drift_correction =
            (time_since_sync as i64 * self.drift_compensation as i64 / 1_000_000) as i32;

        local_time
            .wrapping_add(self.time_offset as u32)
            .wrapping_add(drift_correction as u32)
    }

    /// Convert GPS time to network time
    pub fn gps_to_network_time(&self, gps_time: u32) -> u32 {
        gps_time.wrapping_sub(GPS_EPOCH_OFFSET)
    }

    /// Convert network time to GPS time
    pub fn network_to_gps_time(&self, network_time: u32) -> u32 {
        network_time.wrapping_add(GPS_EPOCH_OFFSET)
    }

    /// Set time offset
    pub fn set_time_offset(&mut self, offset: i32) {
        self.time_offset = offset;
    }

    /// Get local system time
    fn get_local_time(&self) -> u32 {
        // This should be implemented to return the local system time
        // For now, we return a dummy value
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_conversion() {
        let time_sync = NetworkTime::new();

        // Test GPS time conversion
        let gps_time = 1234567890;
        let network_time = time_sync.gps_to_network_time(gps_time);
        assert_eq!(time_sync.network_to_gps_time(network_time), gps_time);
    }

    // #[test]
    // fn test_drift_compensation() {
    //     let mut time_sync = NetworkTime::new();
    //     time_sync.set_time_offset(0);
    //     time_sync.update(0);

    //     let beacon_time: u32 = 128_100;
    //     let time_delta = beacon_time.wrapping_sub(0);
    //     assert_eq!(time_delta, 128_100, "Time delta should be 128_100");

    //     let expected_time = 0u32.wrapping_add(128_000);
    //     assert_eq!(expected_time, 128_000, "Expected time should be 128_000");

    //     let actual_error = beacon_time.wrapping_sub(expected_time);
    //     assert_eq!(actual_error, 100, "Actual error should be 100ms");

    //     let error_ms = actual_error as i32;
    //     assert_eq!(error_ms, 100, "Error in ms should be 100");

    //     let expected_drift = (error_ms * 1_000_000) / time_delta as i32;
    //     assert_eq!(expected_drift, 780, "Expected drift should be 780ppm");

    //     time_sync.update(beacon_time);
    //     assert_eq!(time_sync.drift_compensation, expected_drift, "Drift compensation should match expected value");
    //     assert!(time_sync.drift_compensation > 0, "Drift compensation should be positive");
    //     assert!(time_sync.drift_compensation < 1000, "Drift compensation should be less than 1000ppm");

    //     let current = time_sync.current_time();
    //     assert!(current > 0);
    // }
}
