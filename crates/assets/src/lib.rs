#![no_std]

extern crate alloc;
use alloc::vec::Vec;

include!("../assets_struct.rs");

impl Assets {
	pub fn parse_from_data(data: &[u8]) -> Result<Self, &'static str> {
		wincode::deserialize(data)
			.map_err(|e| "Error deserializing assets file")
	}
}

