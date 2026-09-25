//! Aggregate AC state across every discovered adapter (some machines expose
//! more than one, e.g. USB-C PD on both sides).

use super::adapter::AcAdapter;
use super::detection;
use crate::errors::Result;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AcState {
    Online,
    Offline,
}

pub struct AcManager {
    adapters: Vec<AcAdapter>,
}

impl AcManager {
    pub fn new() -> Result<Self> {
        Ok(Self { adapters: detection::discover()? })
    }

    /// Returns `true` if online/offline state changed since the last refresh.
    pub fn refresh(&mut self) -> Result<bool> {
        let was_online = self.is_online();
        for adapter in &mut self.adapters {
            adapter.refresh()?;
        }
        Ok(was_online != self.is_online())
    }

    pub fn is_online(&self) -> bool {
        // Any adapter reporting online counts as "on AC".
        self.adapters.iter().any(|a| a.online)
    }

    pub fn state(&self) -> AcState {
        if self.is_online() { AcState::Online } else { AcState::Offline }
    }
}
