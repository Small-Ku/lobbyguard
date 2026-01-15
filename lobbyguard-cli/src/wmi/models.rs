use std::net::IpAddr;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceCreationEvent")]
#[serde(rename_all = "PascalCase")]
pub struct ProcessOpenEvent {
	pub target_instance: Process,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceDeletionEvent")]
#[serde(rename_all = "PascalCase")]
pub struct ProcessCloseEvent {
	pub target_instance: Process,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_Process")]
#[serde(rename_all = "PascalCase")]
pub struct Process {
	pub name: String,
	pub process_id: u32,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "MSFT_NetTCPConnection")]
#[serde(rename_all = "PascalCase")]
pub struct NetTCPConnection {
	pub local_port: u16,
	pub remote_address: IpAddr,
	pub remote_port: u16,
	pub owning_process: u32,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "MSFT_NetUDPEndpoint")]
#[serde(rename_all = "PascalCase")]
pub struct NetUDPEndpoint {
	pub local_port: u16,
	pub owning_process: u32,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceCreationEvent")]
#[serde(rename_all = "PascalCase")]
pub struct UDPInstCreateEvent {
	pub target_instance: NetUDPEndpoint,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceModificationEvent")]
#[serde(rename_all = "PascalCase")]
pub struct UDPInstModifyEvent {
	pub target_instance: NetUDPEndpoint,
	pub previous_instance: NetUDPEndpoint,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceDeletionEvent")]
#[serde(rename_all = "PascalCase")]
pub struct UDPInstDeleteEvent {
	pub target_instance: NetUDPEndpoint,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceCreationEvent")]
#[serde(rename_all = "PascalCase")]
pub struct TCPInstCreateEvent {
	pub target_instance: NetTCPConnection,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceModificationEvent")]
#[serde(rename_all = "PascalCase")]
pub struct TCPInstModifyEvent {
	pub target_instance: NetTCPConnection,
	pub previous_instance: NetTCPConnection,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceDeletionEvent")]
#[serde(rename_all = "PascalCase")]
pub struct TCPInstDeleteEvent {
	pub target_instance: NetTCPConnection,
}
