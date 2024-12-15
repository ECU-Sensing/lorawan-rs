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
        // Calculate time since last sync
        let time_delta = beacon_time.wrapping_sub(self.last_sync);
        
        if self.last_sync != 0 {
            // Calculate timing error
            let expected_time = self.last_sync.wrapping_add(128_000); // Beacon interval
            let actual_error = beacon_time.wrapping_sub(expected_time) as i32;
            
            // Update timing error with exponential moving average
            self.timing_error = (self.timing_error * 7 + actual_error * 1000) / 8;
            
            // Update drift compensation
            self.drift_compensation = self.timing_error / time_delta as i32;
        }
        
        self.last_sync = beacon_time;
    }

    /// Get current network time
    pub fn current_time(&self) -> u32 {
        let local_time = self.get_local_time();
        let time_since_sync = local_time.wrapping_sub(self.last_sync);
        
        // Apply drift compensation
        let drift_correction = (time_since_sync as i64 * self.drift_compensation as i64 / 1_000_000) as i32;
        
        local_time.wrapping_add(self.time_offset as u32)
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

    #[test]
    fn test_drift_compensation() {
        let mut time_sync = NetworkTime::new();
        
        // Simulate beacon reception with drift
        time_sync.update(0);
        time_sync.update(128_100); // 100ms drift
        
        // Verify drift compensation
        assert!(time_sync.drift_compensation != 0);
    }
} 