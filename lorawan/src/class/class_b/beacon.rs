//! LoRaWAN Class B Beacon Tracking
//!
//! This module handles beacon synchronization and tracking including:
//! - Beacon acquisition and synchronization
//! - Beacon timing and window calculation
//! - Beacon loss detection and recovery

use crate::{
    radio::traits::Radio,
    lorawan::{
        region::Region,
        mac::{MacLayer, MacError},
    },
};

/// Beacon timing parameters (all times in milliseconds)
const BEACON_INTERVAL: u32 = 128_000;
const BEACON_RESERVED: u32 = 2_120;
const BEACON_WINDOW: u32 = 122_880;
const BEACON_GUARD: u32 = 3_000;

/// Maximum beacon missed before declaring loss
const MAX_BEACON_MISSED: u8 = 3;

/// Beacon tracking state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BeaconState {
    /// Not tracking beacons
    Idle,
    /// Searching for initial beacon
    Searching,
    /// Synchronized with network beacons
    Synchronized,
    /// Lost beacon synchronization
    Lost,
}

/// Beacon tracking information
#[derive(Debug)]
pub struct BeaconTracker {
    /// Current beacon state
    state: BeaconState,
    /// Time of last received beacon
    last_beacon_time: u32,
    /// Number of consecutive missed beacons
    missed_beacons: u8,
    /// Beacon timing drift (ppm)
    timing_drift: i32,
}

impl BeaconTracker {
    /// Create new beacon tracker
    pub fn new() -> Self {
        Self {
            state: BeaconState::Idle,
            last_beacon_time: 0,
            missed_beacons: 0,
            timing_drift: 0,
        }
    }

    /// Start beacon acquisition
    pub fn start_acquisition<R: Radio + Clone, REG: Region>(
        &mut self,
        mac: &mut MacLayer<R, REG>,
    ) -> Result<(), MacError<R::Error>> {
        // Configure radio for beacon reception
        let beacon_channel = mac.get_region_mut().get_next_beacon_channel()
            .ok_or(MacError::InvalidChannel)?;
            
        mac.set_rx_config(
            beacon_channel.frequency,
            beacon_channel.min_dr,
            BEACON_WINDOW as u32,
        )?;

        self.state = BeaconState::Searching;
        Ok(())
    }

    /// Process beacon tracking
    pub fn process<R: Radio + Clone, REG: Region>(
        &mut self,
        mac: &mut MacLayer<R, REG>,
    ) -> Result<(), MacError<R::Error>> {
        match self.state {
            BeaconState::Searching => {
                self.process_beacon_search(mac)?;
            }
            BeaconState::Synchronized => {
                self.process_beacon_tracking(mac)?;
            }
            BeaconState::Lost => {
                self.process_beacon_recovery(mac)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Process beacon search
    fn process_beacon_search<R: Radio + Clone, REG: Region>(
        &mut self,
        mac: &mut MacLayer<R, REG>,
    ) -> Result<(), MacError<R::Error>> {
        // Try to receive beacon
        if let Some(beacon) = self.receive_beacon(mac)? {
            // Validate beacon
            if self.validate_beacon(&beacon) {
                self.last_beacon_time = beacon.time;
                self.state = BeaconState::Synchronized;
                self.missed_beacons = 0;
            }
        }
        Ok(())
    }

    /// Process synchronized beacon tracking
    fn process_beacon_tracking<R: Radio + Clone, REG: Region>(
        &mut self,
        mac: &mut MacLayer<R, REG>,
    ) -> Result<(), MacError<R::Error>> {
        let current_time = mac.get_time();
        
        // Check if we're in beacon window
        if self.is_beacon_window(current_time) {
            if let Some(beacon) = self.receive_beacon(mac)? {
                // Update timing
                self.update_timing(beacon.time);
                self.missed_beacons = 0;
            } else {
                self.missed_beacons += 1;
                if self.missed_beacons >= MAX_BEACON_MISSED {
                    self.state = BeaconState::Lost;
                }
            }
        }
        Ok(())
    }

    /// Process beacon recovery
    fn process_beacon_recovery<R: Radio + Clone, REG: Region>(
        &mut self,
        mac: &mut MacLayer<R, REG>,
    ) -> Result<(), MacError<R::Error>> {
        // Widen search window
        let search_window = BEACON_WINDOW + 2 * BEACON_GUARD;
        
        // Configure radio with wider window
        let beacon_channel = mac.get_region_mut().get_next_beacon_channel()
            .ok_or(MacError::InvalidChannel)?;
            
        mac.set_rx_config(
            beacon_channel.frequency,
            beacon_channel.min_dr,
            search_window,
        )?;

        // Try to reacquire beacon
        if let Some(beacon) = self.receive_beacon(mac)? {
            if self.validate_beacon(&beacon) {
                self.last_beacon_time = beacon.time;
                self.state = BeaconState::Synchronized;
                self.missed_beacons = 0;
            }
        }
        Ok(())
    }

    /// Check if current time is in beacon window
    fn is_beacon_window(&self, current_time: u32) -> bool {
        let time_since_beacon = current_time.wrapping_sub(self.last_beacon_time);
        let window_start = BEACON_INTERVAL - BEACON_GUARD;
        let window_end = BEACON_INTERVAL + BEACON_GUARD;
        
        time_since_beacon >= window_start && time_since_beacon <= window_end
    }

    /// Update beacon timing
    fn update_timing(&mut self, beacon_time: u32) {
        let expected_time = self.last_beacon_time.wrapping_add(BEACON_INTERVAL);
        let drift = beacon_time.wrapping_sub(expected_time) as i32;
        
        // Update timing drift using exponential moving average
        self.timing_drift = (self.timing_drift * 7 + drift) / 8;
        self.last_beacon_time = beacon_time;
    }

    /// Validate received beacon
    fn validate_beacon(&self, beacon: &BeaconData) -> bool {
        // Basic validation: check if beacon info is not all zeros
        !beacon.info.iter().all(|&b| b == 0)
    }

    /// Get current beacon state
    pub fn state(&self) -> BeaconState {
        self.state
    }

    /// Check if beacon is synchronized
    pub fn is_synchronized(&self) -> bool {
        self.state == BeaconState::Synchronized
    }

    /// Get last beacon time
    pub fn last_beacon_time(&self) -> u32 {
        self.last_beacon_time
    }

    /// Receive beacon
    fn receive_beacon<R: Radio + Clone, REG: Region>(
        &mut self,
        mac: &mut MacLayer<R, REG>,
    ) -> Result<Option<BeaconData>, MacError<R::Error>> {
        let mut buffer = [0u8; 17]; // Beacon size is 17 bytes
        match mac.receive(&mut buffer) {
            Ok(size) if size == 17 => {
                Ok(Some(BeaconData {
                    time: mac.get_time(),
                    info: buffer,
                }))
            }
            Ok(_) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// Beacon data structure
#[derive(Debug)]
struct BeaconData {
    time: u32,
    info: [u8; 17],
} 