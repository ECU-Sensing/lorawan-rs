//! LoRaWAN Class B Ping Slot Management
//!
//! This module handles ping slot timing and randomization including:
//! - Ping slot scheduling and calculation
//! - Randomization for collision avoidance
//! - Timing synchronization with beacons

use core::cmp::min;
use heapless::Vec;

/// Maximum number of ping slots per beacon period
const MAX_PING_SLOTS: usize = 16;

/// Ping slot configuration
#[derive(Debug, Clone)]
pub struct PingSlotConfig {
    /// Ping slot periodicity (0-7)
    periodicity: u8,
    /// Data rate for ping slots
    data_rate: u8,
    /// Frequency for ping slots
    frequency: u32,
}

impl PingSlotConfig {
    /// Create new ping slot configuration
    pub fn new(periodicity: u8, data_rate: u8, frequency: u32) -> Self {
        Self {
            periodicity: min(periodicity, 7),
            data_rate,
            frequency,
        }
    }

    /// Set ping slot periodicity
    pub fn set_periodicity(&mut self, periodicity: u8) {
        self.periodicity = min(periodicity, 7);
    }

    /// Get ping slot data rate
    pub fn data_rate(&self) -> u8 {
        self.data_rate
    }

    /// Get ping slot frequency
    pub fn frequency(&self) -> u32 {
        self.frequency
    }

    /// Get number of ping slots per beacon period
    pub fn slots_per_beacon(&self) -> u32 {
        128 >> self.periodicity
    }
}

impl Default for PingSlotConfig {
    fn default() -> Self {
        Self {
            periodicity: 0,
            data_rate: 0,
            frequency: 0,
        }
    }
}

/// Ping slot scheduler
#[derive(Debug)]
pub struct PingSlotScheduler {
    /// Scheduled ping slots
    slots: Vec<u32, MAX_PING_SLOTS>,
    /// Random seed for slot calculation
    rand_seed: u32,
}

impl PingSlotScheduler {
    /// Create new ping slot scheduler
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            rand_seed: 0,
        }
    }

    /// Update ping slot schedule
    pub fn update_schedule(&mut self, config: &PingSlotConfig, _beacon_time: u32) {
        self.slots.clear();

        let num_slots = config.slots_per_beacon();
        let beacon_reserved = 2_120; // ms

        // Calculate ping slots using device address as randomization seed
        for i in 0..num_slots {
            let slot_time = beacon_reserved + self.calculate_slot_offset(i);
            if self.slots.push(slot_time).is_err() {
                break;
            }
        }
    }

    /// Calculate randomized slot offset
    fn calculate_slot_offset(&self, slot_index: u32) -> u32 {
        // Base offset ensures minimum spacing (40ms)
        let base_offset = slot_index * 40;

        // Add random offset that won't violate minimum spacing
        let hash = self.rand_seed.wrapping_mul(slot_index.wrapping_add(1));
        let random_offset = hash % 5;

        base_offset.saturating_add(random_offset)
    }

    /// Get next ping slot time
    pub fn next_slot(&self, current_time: u32) -> Option<u32> {
        self.slots
            .iter()
            .find(|&&slot| slot > current_time)
            .copied()
    }

    /// Set random seed for slot calculation
    pub fn set_random_seed(&mut self, seed: u32) {
        self.rand_seed = seed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_slot_calculation() {
        let mut config = PingSlotConfig::default();
        config.set_periodicity(1); // 64 slots

        let mut scheduler = PingSlotScheduler::new();
        scheduler.set_random_seed(0x12345678);
        scheduler.update_schedule(&config, 0);

        // Verify number of slots
        assert_eq!(scheduler.slots.len(), 16); // Limited by MAX_PING_SLOTS

        // Verify slot spacing
        let mut last_slot = 0;
        for (i, &slot) in scheduler.slots.iter().enumerate() {
            let spacing = slot.saturating_sub(last_slot);
            assert!(
                slot >= last_slot + 30,
                "Slot {} has insufficient spacing: {} ms (slot time: {}, last slot: {})",
                i,
                spacing,
                slot,
                last_slot
            );
            last_slot = slot;
        }
    }

    #[test]
    fn test_next_slot() {
        let mut config = PingSlotConfig::default();
        config.set_periodicity(2); // 32 slots

        let mut scheduler = PingSlotScheduler::new();
        scheduler.update_schedule(&config, 0);

        // Test next slot finding
        if let Some(first_slot) = scheduler.next_slot(0) {
            assert!(first_slot >= 2_120); // After beacon reserved
            assert!(scheduler.next_slot(first_slot).unwrap() > first_slot);
        }
    }
}
