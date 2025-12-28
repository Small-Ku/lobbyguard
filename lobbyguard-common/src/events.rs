use crate::flow::FlowKey;

#[derive(Debug, Clone)]
pub enum MonitorEvent {
	ProcessFound(u32),
	ProcessLost(u32),
	FlowEstablished(FlowKey),
	FlowDeleted(FlowKey),
}

#[derive(Debug, Clone)]
pub enum GuardEvent {
	PacketBlocked,
	PacketAllowed,
	EngineStopped,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
	Monitor(MonitorEvent),
	Guard(GuardEvent),
	Error(String),
}
