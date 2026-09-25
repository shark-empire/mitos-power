//! Battery discovery, monitoring and derived statistics.

pub mod battery;
pub mod capacity;
pub mod charging;
pub mod current;
pub mod device;
pub mod health;
pub mod statistics;
pub mod temperature;
pub mod time_remaining;
pub mod voltage;

pub use battery::{BatteryInfo, BatteryManager, RefreshResult};
pub use charging::ChargingState;
pub use device::BatteryDevice;
