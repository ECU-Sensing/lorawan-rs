//! Power management and monitoring for LoRaWAN devices
//!
//! This module provides power management features including:
//! - Battery level monitoring
//! - Power consumption tracking
//! - Power saving modes
//! - Duty cycle management

use core::time::Duration;

/// Power consumption states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerState {
    /// Normal operation
    Normal,
    /// Power saving mode
    PowerSaving,
    /// Critical battery level
    Critical,
}

/// Power consumption metrics
#[derive(Debug, Clone)]
pub struct PowerMetrics {
    /// Battery level (0-255, 0=external power)
    pub battery_level: u8,
    /// Estimated power consumption in mA
    pub current_consumption: u16,
    /// Time spent in TX mode
    pub tx_time: Duration,
    /// Time spent in RX mode
    pub rx_time: Duration,
    /// Time spent in sleep mode
    pub sleep_time: Duration,
}

impl PowerMetrics {
    /// Create new power metrics
    pub fn new() -> Self {
        Self {
            battery_level: 255,
            current_consumption: 0,
            tx_time: Duration::from_secs(0),
            rx_time: Duration::from_secs(0),
            sleep_time: Duration::from_secs(0),
        }
    }

    /// Update battery level
    pub fn update_battery(&mut self, level: u8) {
        self.battery_level = level;
    }

    /// Add TX time
    pub fn add_tx_time(&mut self, duration: Duration) {
        self.tx_time += duration;
        // Typical TX current: 120mA
        self.current_consumption += (duration.as_millis() as u16 * 120) / 1000;
    }

    /// Add RX time
    pub fn add_rx_time(&mut self, duration: Duration) {
        self.rx_time += duration;
        // Typical RX current: 12mA
        self.current_consumption += (duration.as_millis() as u16 * 12) / 1000;
    }

    /// Add sleep time
    pub fn add_sleep_time(&mut self, duration: Duration) {
        self.sleep_time += duration;
        // Typical sleep current: 1µA
        self.current_consumption += (duration.as_millis() as u16) / 1_000_000;
    }

    /// Get total active time
    pub fn get_active_time(&self) -> Duration {
        self.tx_time + self.rx_time
    }

    /// Get duty cycle percentage
    pub fn get_duty_cycle(&self) -> f32 {
        let total = self.get_active_time() + self.sleep_time;
        if total.as_secs() == 0 {
            return 0.0;
        }
        (self.get_active_time().as_secs_f32() / total.as_secs_f32()) * 100.0
    }
}

/// Power management configuration
#[derive(Debug, Clone)]
pub struct PowerConfig {
    /// Critical battery threshold (0-255)
    pub critical_threshold: u8,
    /// Low battery threshold (0-255)
    pub low_threshold: u8,
    /// Maximum duty cycle (%)
    pub max_duty_cycle: f32,
    /// Power saving mode enabled
    pub power_saving_enabled: bool,
}

impl Default for PowerConfig {
    fn default() -> Self {
        Self {
            critical_threshold: 10,
            low_threshold: 30,
            max_duty_cycle: 1.0,
            power_saving_enabled: false,
        }
    }
}

/// Power manager for LoRaWAN devices
pub struct PowerManager {
    /// Power configuration
    config: PowerConfig,
    /// Power metrics
    metrics: PowerMetrics,
    /// Current power state
    state: PowerState,
}

impl PowerManager {
    /// Create new power manager
    pub fn new(config: PowerConfig) -> Self {
        Self {
            config,
            metrics: PowerMetrics::new(),
            state: PowerState::Normal,
        }
    }

    /// Update battery level and check thresholds
    pub fn update_battery(&mut self, level: u8) -> PowerState {
        self.metrics.update_battery(level);
        
        self.state = if level <= self.config.critical_threshold {
            PowerState::Critical
        } else if level <= self.config.low_threshold || self.config.power_saving_enabled {
            PowerState::PowerSaving
        } else {
            PowerState::Normal
        };

        self.state
    }

    /// Record TX operation
    pub fn record_tx(&mut self, duration: Duration) {
        self.metrics.add_tx_time(duration);
    }

    /// Record RX operation
    pub fn record_rx(&mut self, duration: Duration) {
        self.metrics.add_rx_time(duration);
    }

    /// Record sleep period
    pub fn record_sleep(&mut self, duration: Duration) {
        self.metrics.add_sleep_time(duration);
    }

    /// Check if duty cycle limit is exceeded
    pub fn is_duty_cycle_exceeded(&self) -> bool {
        self.metrics.get_duty_cycle() > self.config.max_duty_cycle
    }

    /// Get current power metrics
    pub fn get_metrics(&self) -> &PowerMetrics {
        &self.metrics
    }

    /// Get current power state
    pub fn get_state(&self) -> PowerState {
        self.state
    }

    /// Enable power saving mode
    pub fn enable_power_saving(&mut self) {
        self.config.power_saving_enabled = true;
        if self.state == PowerState::Normal {
            self.state = PowerState::PowerSaving;
        }
    }

    /// Disable power saving mode
    pub fn disable_power_saving(&mut self) {
        self.config.power_saving_enabled = false;
        if self.state == PowerState::PowerSaving && 
           self.metrics.battery_level > self.config.low_threshold {
            self.state = PowerState::Normal;
        }
    }
} 