//! CPU power management: governor, boost/turbo, frequency and topology
//! introspection.

pub mod boost;
pub mod cores;
pub mod frequency;
pub mod governor;
pub mod topology;

use crate::errors::Result;

/// Thin facade used by `manager::manager::PowerManager` when applying a
/// profile switch.
pub struct CpuController;

impl CpuController {
    pub fn new() -> Self {
        Self
    }

    pub fn apply_governor(&self, name: &str) -> Result<()> {
        governor::set_all(name)
    }

    pub fn set_boost(&self, enabled: bool) -> Result<()> {
        boost::set(enabled)
    }
}

impl Default for CpuController {
    fn default() -> Self {
        Self::new()
    }
}
