//! Storage module for persisting captured packet data
//!
//! Uses `compio::fs` for asynchronous file operations and `epserde`
//! for efficient serialization of packet capture data.

use crate::error;
use crate::packet_data::PacketCapture;
use epserde::deser::Deserialize;
use epserde::ser::Serialize;
use snafu::ResultExt;

/// Handles asynchronous storage of captured packets
pub struct CaptureStorage {
	file_path: String,
}

impl CaptureStorage {
	/// Creates a new storage instance for the given file path
	pub fn new(file_path: impl Into<String>) -> Self {
		Self {
			file_path: file_path.into(),
		}
	}

	/// Saves a packet capture session to disk
	///
	/// # Errors
	///
	/// Returns an error if file operations or serialization fails
	pub async fn save_capture(&self, capture: &PacketCapture) -> crate::Result<()> {
		// Serialize the capture data using epserde
		let mut serialized = Vec::new();
		unsafe {
			capture
				.serialize(&mut serialized)
				.map_err(|e| error::Error::Serialize {
					source: Box::new(e) as Box<dyn std::error::Error + Send + Sync>,
				})?;
		}

		// Write to file using compio
		compio::fs::write(&self.file_path, serialized)
			.await
			.0
			.context(error::StorageWriteSnafu)?;

		Ok(())
	}

	/// Loads a packet capture session from disk
	///
	/// # Errors
	///
	/// Returns an error if file operations or deserialization fails
	pub async fn load_capture(&self) -> crate::Result<PacketCapture> {
		let data = compio::fs::read(&self.file_path)
			.await
			.context(error::StorageReadSnafu)?;

		// Fully deserialize (owned copy) from bytes
		let capture = unsafe {
			PacketCapture::deserialize_full(&mut &data[..]).map_err(|e| error::Error::Deserialize {
				source: Box::new(e) as Box<dyn std::error::Error + Send + Sync>,
			})?
		};
		Ok(capture)
	}

	/// Checks if the storage file exists
	pub async fn exists(&self) -> crate::Result<bool> {
		match compio::fs::metadata(&self.file_path).await {
			Ok(_) => Ok(true),
			Err(_) => Ok(false),
		}
	}

	/// Deletes the storage file if it exists
	pub async fn delete(&self) -> crate::Result<()> {
		if self.exists().await? {
			compio::fs::remove_file(&self.file_path)
				.await
				.context(error::StorageDeleteSnafu)?;
		}
		Ok(())
	}

	/// Gets the file path for this storage
	pub fn path(&self) -> &str {
		&self.file_path
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_storage_creation() {
		let storage = CaptureStorage::new("test.dat");
		assert_eq!(storage.path(), "test.dat");
	}
}
