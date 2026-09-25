//! Generic TOML parsing wrapper -- kept as its own tiny module so
//! `loader.rs` stays focused on *which* file goes *where*, not on parsing
//! mechanics.

use crate::errors::Result;
use serde::de::DeserializeOwned;

pub fn parse<T: DeserializeOwned>(text: &str) -> Result<T> {
    Ok(toml::from_str(text)?)
}
