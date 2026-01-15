use dashmap::{DashMap, DashSet};
use log::debug;

/// Manages tracking of game processes and their network connections
pub struct ConnectionTracker {
	/// Set of tracked process IDs (e.g., GTA5_Enhanced.exe)
	pub pid_set: DashSet<u32>,
	/// Map of PID -> Set<(local_port, remote_port)> for TCP connections
	pub tcp_map: DashMap<u32, DashSet<(u16, u16)>>,
	/// Map of PID -> Set<local_port> for UDP endpoints
	pub udp_map: DashMap<u32, DashSet<u16>>,
}

impl ConnectionTracker {
	/// Create a new ConnectionTracker
	pub fn new() -> Self {
		Self {
			pid_set: DashSet::new(),
			tcp_map: DashMap::new(),
			udp_map: DashMap::new(),
		}
	}

	/// Add a process ID to track
	pub fn add_process(&self, pid: u32) { self.pid_set.insert(pid); }

	/// Remove a process ID and its connections
	pub fn remove_process(&self, pid: u32) {
		self.pid_set.remove(&pid);
		self.tcp_map.remove(&pid);
		self.udp_map.remove(&pid);
	}

	/// Check if a process is being tracked
	pub fn contains_process(&self, pid: u32) -> bool { self.pid_set.contains(&pid) }

	/// Add a TCP connection for a process
	pub fn add_tcp_connection(&self, pid: u32, local_port: u16, remote_port: u16) {
		if local_port == 0 || remote_port == 0 || pid == 0 {
			return;
		}
		debug!(
			"TCP connection added for PID {}: local:{} <=> remote:{}",
			pid, local_port, remote_port
		);
		let entry = self.tcp_map.entry(pid).or_default();
		entry.value().insert((local_port, remote_port));
	}

	/// Remove a TCP connection for a process
	pub fn remove_tcp_connection(&self, pid: u32, local_port: u16, remote_port: u16) {
		if local_port == 0 || remote_port == 0 || pid == 0 {
			return;
		}
		if let Some(entry) = self.tcp_map.get(&pid) {
			debug!(
				"TCP connection removed for PID {}: local:{} <=> remote:{}",
				pid, local_port, remote_port
			);
			entry.value().remove(&(local_port, remote_port));
		}
	}

	/// Add a UDP endpoint for a process
	pub fn add_udp_endpoint(&self, pid: u32, local_port: u16) {
		if local_port == 0 || pid == 0 {
			return;
		}
		debug!("UDP endpoint added for PID {}: local:{}", pid, local_port);
		let entry = self.udp_map.entry(pid).or_default();
		entry.value().insert(local_port);
	}

	/// Remove a UDP endpoint for a process
	pub fn remove_udp_endpoint(&self, pid: u32, local_port: u16) {
		if local_port == 0 || pid == 0 {
			return;
		}
		if let Some(entry) = self.udp_map.get(&pid) {
			debug!("UDP endpoint removed for PID {}: local:{}", pid, local_port);
			entry.value().remove(&local_port);
		}
	}

	/// Check if a UDP packet with the given local port belongs to a tracked process
	pub fn is_tracked_udp(&self, local_port: u16) -> bool {
		if local_port == 0 {
			return false;
		}
		self.pid_set.iter().any(|pid| {
			self
				.udp_map
				.view(pid.key(), |_, ports| ports.contains(&local_port))
				.unwrap_or(false)
		})
	}

	/// Check if a TCP packet belongs to a tracked process
	pub fn is_tracked_tcp(&self, src_port: u16, dst_port: u16) -> bool {
		if src_port == 0 || dst_port == 0 {
			return false;
		}
		self.pid_set.iter().any(|pid| {
			self
				.tcp_map
				.view(pid.key(), |_, ports| {
					ports.contains(&(src_port, dst_port)) || ports.contains(&(dst_port, src_port))
				})
				.unwrap_or(false)
		})
	}
}

impl Default for ConnectionTracker {
	fn default() -> Self { Self::new() }
}
