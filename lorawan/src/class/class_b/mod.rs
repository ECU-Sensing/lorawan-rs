//! LoRaWAN Class B Implementation
//!
//! This module implements LoRaWAN Class B functionality including:
//! - Beacon synchronization and tracking
//! - Ping slot timing and randomization
//! - Network time synchronization
//! - Beacon loss detection and recovery

pub mod beacon;
pub mod ping_slot;
pub mod timing;

use crate::{
    class::{DeviceClass, OperatingMode},
    config::device::{AESKey, SessionState},
    lorawan::{
        mac::{MacError, MacLayer},
        region::{DataRate, Region},
    },
    radio::traits::Radio,
};

use self::{
    beacon::{BeaconState, BeaconTracker},
    ping_slot::{PingSlotConfig, PingSlotScheduler},
    timing::NetworkTime,
};

/// Maximum number of ping slots per beacon period
const MAX_PING_SLOTS: usize = 16;

/// Class B device implementation
pub struct ClassB<R: Radio + Clone, REG: Region> {
    /// MAC layer for radio communication
    mac: MacLayer<R, REG>,
    /// Beacon tracking state
    beacon_tracker: BeaconTracker,
    /// Ping slot configuration
    ping_slot_config: PingSlotConfig,
    /// Ping slot scheduler
    ping_scheduler: PingSlotScheduler,
    /// Network time synchronization
    network_time: NetworkTime,
}

impl<R: Radio + Clone, REG: Region> ClassB<R, REG> {
    /// Create new Class B device
    pub fn new(mac: MacLayer<R, REG>) -> Self {
        Self {
            mac,
            beacon_tracker: BeaconTracker::new(),
            ping_slot_config: PingSlotConfig::default(),
            ping_scheduler: PingSlotScheduler::new(),
            network_time: NetworkTime::new(),
        }
    }

    /// Start Class B operation
    pub fn start(&mut self) -> Result<(), MacError<R::Error>> {
        // Start beacon acquisition
        self.beacon_tracker.start_acquisition(&mut self.mac)?;
        Ok(())
    }

    /// Process Class B operations
    pub fn process(&mut self) -> Result<(), MacError<R::Error>> {
        // Process beacon tracking
        self.beacon_tracker.process(&mut self.mac)?;

        // Update network time if beacon synchronized
        if self.beacon_tracker.is_synchronized() {
            self.network_time
                .update(self.beacon_tracker.last_beacon_time());
        }

        // Process ping slots if synchronized
        if let BeaconState::Synchronized = self.beacon_tracker.state() {
            self.process_ping_slots()?;
        }

        Ok(())
    }

    /// Configure ping slot parameters
    pub fn configure_ping_slots(&mut self, periodicity: u8) -> Result<(), MacError<R::Error>> {
        self.ping_slot_config.set_periodicity(periodicity);
        self.ping_scheduler
            .update_schedule(&self.ping_slot_config, self.network_time.current_time());
        Ok(())
    }

    /// Process ping slots
    fn process_ping_slots(&mut self) -> Result<(), MacError<R::Error>> {
        let current_time = self.network_time.current_time();

        // Check if we need to open a ping slot
        if let Some(slot) = self.ping_scheduler.next_slot(current_time) {
            self.open_ping_slot(slot)?;
        }

        Ok(())
    }

    /// Open a ping receive slot
    fn open_ping_slot(&mut self, _slot: u32) -> Result<(), MacError<R::Error>> {
        // Configure radio for ping slot reception
        self.mac.set_rx_config(
            self.ping_slot_config.frequency(),
            DataRate::from_index(self.ping_slot_config.data_rate()),
            30, // 30ms ping slot timeout
        )?;

        // Start reception for ping slot duration
        let mut buffer = [0u8; 256];
        self.mac.receive(&mut buffer)?;

        Ok(())
    }
}

impl<R: Radio + Clone, REG: Region> DeviceClass<R, REG> for ClassB<R, REG> {
    type Error = MacError<R::Error>;

    fn operating_mode(&self) -> OperatingMode {
        OperatingMode::ClassB
    }

    fn process(&mut self) -> Result<(), Self::Error> {
        // Call the process implementation from ClassB
        ClassB::process(self)
    }

    fn send_data(&mut self, port: u8, data: &[u8], confirmed: bool) -> Result<(), Self::Error> {
        if confirmed {
            self.mac.send_confirmed(port, data)
        } else {
            self.mac.send_unconfirmed(port, data)
        }
    }

    fn send_join_request(
        &mut self,
        dev_eui: [u8; 8],
        app_eui: [u8; 8],
        app_key: AESKey,
    ) -> Result<(), Self::Error> {
        self.mac.join_request(dev_eui, app_eui, app_key)
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        self.mac.receive(buffer)
    }

    fn get_session_state(&self) -> SessionState {
        self.mac.get_session_state().clone()
    }

    fn get_mac_layer(&self) -> &MacLayer<R, REG> {
        &self.mac
    }
}
