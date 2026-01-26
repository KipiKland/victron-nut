use std::{cmp, error::Error, hash::{DefaultHasher, Hash, Hasher}};
use enum_display::EnumDisplay;
use log::{debug, error, info};
use crate::{Application, config::ShutdownPolicy, connection::ClientData, dbus::{BATTERY_DEST, INVERTER_DEST}, storage::shutdown_policy_states_store::{self, ShutdownPolicyStatesStore}, utility::wake_on_lan};

const SYSTEM_STATE_OFF: u32 = 0x00;
#[allow(dead_code)] const SYSTEM_STATE_LOW_POWER: u32 = 0x01;
const SYSTEM_STATE_FAULT_CONDITION: u32 = 0x02;
const SYSTEM_STATE_BULK_CHARGING: u32 = 0x03;
const SYSTEM_STATE_ABSORPTION_CHARGING: u32 = 0x04;
const SYSTEM_STATE_FLOAT_CHARGING: u32 = 0x05;
const SYSTEM_STATE_STORAGE_MODE: u32 = 0x06;
#[allow(dead_code)] const SYSTEM_STATE_EQUALIZATION_CHARGING: u32 = 0x07;
const SYSTEM_STATE_PASS_THROUGH: u32 = 0x08;
const SYSTEM_STATE_INVERTING: u32 = 0x09;
const SYSTEM_STATE_ASSISTING: u32 = 0x0A;
#[allow(dead_code)] const SYSTEM_STATE_POWER_SUPPLY_MODE: u32 = 0x0B;
const SYSTEM_STATE_SUSTAIN: u32 = 0xF4;
#[allow(dead_code)] const SYSTEM_STATE_WAKEUP: u32 = 0xF5;
#[allow(dead_code)] const SYSTEM_STATE_REPEATED_ABSORPTION: u32 = 0xF6;
#[allow(dead_code)] const SYSTEM_STATE_AUTO_EQUALIZE: u32 = 0xF7;
#[allow(dead_code)] const SYSTEM_STATE_BATTERY_SAFE: u32 = 0xF8;
#[allow(dead_code)] const SYSTEM_STATE_LOAD_DETECT: u32 = 0xF9;
#[allow(dead_code)] const SYSTEM_STATE_BLOCKED: u32 = 0xFA;
#[allow(dead_code)] const SYSTEM_STATE_TEST: u32 = 0xFB;
#[allow(dead_code)] const SYSTEM_STATE_EXTERNAL_CONTROL: u32 = 0xFC;
const SYSTEM_STATE_DISCHARGING: u32 = 0x100;
const SYSTEM_STATE_SYSTEM_SUSTAIN: u32 = 0x101;
const SYSTEM_STATE_RECHARGE: u32 = 0x102;
const SYSTEM_STATE_SCHEDULED_CHARGE: u32 = 0x103;
const SYSTEM_STATE_DYNAMIC_ESS: u32 = 0x104;

const INTERNAL_INVERTER_CONSUMPTION: f32 = 13.0;

#[derive(Debug, Hash, Clone, PartialEq, EnumDisplay)]
pub enum ChargerStatus {
	Bulk, Absorption, Sustain
}

#[derive(Debug, Hash, Clone, PartialEq, EnumDisplay)]
pub enum EssStatus {
	Discharging, Sustain, Charging, DynamicEss
}

#[derive(Debug, Hash, Clone, PartialEq, EnumDisplay)]
pub enum LogicStatus {
	HealthyAndFull,
	HealthyAndAssisting,
	#[display("HealthyAndCharging({0})")]
	HealthyAndCharging(ChargerStatus),
	#[display("HealthyAndEss({0})")]
	HealthyAndEss(EssStatus),
	Passthrough,
	OnBattery,
	Dead,
	Unknown
}
impl LogicStatus {
	pub fn has_power(&self) -> bool {
		match self {
			Self::HealthyAndFull => true,
			Self::HealthyAndAssisting => true,
			Self::HealthyAndCharging(_) => true,
			Self::HealthyAndEss(_) => true,
			Self::Passthrough => true,
			Self::Unknown => true,
			Self::Dead => false,
			Self::OnBattery => false
		}
	}
}

#[derive(Debug)]
pub struct ShutdownPolicyState {
	pub policy_config: ShutdownPolicy,
	pub triggered_shutdown: bool,
	pub restart_required: bool
}
impl ShutdownPolicyState {
	pub async fn is_client_connected(&self, app: &Application) -> bool {
		if self.policy_config.binding_nut_user.is_some() || self.policy_config.binding_nut_ip.is_some() {
			let clients = app.clients.lock().await;
			for client in clients.iter() {
				let client_data = client.client_data.read().await;
				if self.is_matching_to_client(&client_data) {
					return true;
				}
			}
		}

		false
	}
	
	pub fn is_matching_to_client(&self, client_data: &ClientData) -> bool {
		if let Some(nut_user) = &self.policy_config.binding_nut_user {
			if let Some(client_username) = &client_data.username && client_username.eq(nut_user) {
				return true;
			}
		}

		if let Some(nut_ip) = &self.policy_config.binding_nut_ip {
			if nut_ip.eq(&client_data.addr.ip()) {
				return true;
			}
		}

		if self.policy_config.is_default() {
			return true;
		}

		false
	}
}

#[derive(Debug)]
pub struct LogicState {
	pub remaining_battery_percentage: i32,
	pub remaining_battery_runtime_secs: i32,
	pub status: LogicStatus,
	pub forced_shutdown: bool,
	pub shutdown_policy_states: Vec<ShutdownPolicyState>,
	pending_shutdown_policy_states_save: bool
}
impl LogicState {
	pub fn new() -> Self {
		Self {
			remaining_battery_percentage: 100,
			remaining_battery_runtime_secs: -1,
			status: LogicStatus::Unknown,
			forced_shutdown: false,
			shutdown_policy_states: Vec::new(),
			pending_shutdown_policy_states_save: false
		}
	}

	pub fn calculate_hash_for_status(&self) -> u64 {
		let mut hasher = DefaultHasher::new();
		self.remaining_battery_percentage.hash(&mut hasher);
		self.remaining_battery_runtime_secs.hash(&mut hasher);
		self.status.hash(&mut hasher);
		self.forced_shutdown.hash(&mut hasher);
		hasher.finish()
	}
}

pub enum PendingLogicAction {
	Restart(ShutdownPolicy)
}

pub struct LogicManager {
	app: Application,
	shutdown_policy_states_store: ShutdownPolicyStatesStore
}
impl LogicManager {
	pub fn new(app: Application) -> Self {
		let shutdown_policy_states_filepath = app.config.store_folder_path.join("shutdown_policy_states.json");
		Self {
			app: app,
			shutdown_policy_states_store: ShutdownPolicyStatesStore::new(shutdown_policy_states_filepath)
		}
	}

	pub async fn init(&self) {
		let stored_data = match self.shutdown_policy_states_store.read().await {
			Ok(stored_data) => stored_data,
			Err(err) => {
				error!("Failed to read shutdown policy states from filesystem: {}", err);
				shutdown_policy_states_store::StoreData::new()
			}
		};

		let shutdown_policy_states = stored_data.get_shutdown_policy_states(&self.app.config);
		{
			let mut logic_state = self.app.logic_state.write().await;
			logic_state.shutdown_policy_states = shutdown_policy_states;
		}
	}

	pub async fn tick(&self) -> Result<(), Box<dyn Error>> {
		let battery_soc = self.app.dbus.get_value::<f64>("/Soc", INVERTER_DEST).await? as i32;
		let battery_soc_remaining = cmp::max(battery_soc - self.app.config.inverter_shutdown_soc, 0);

		let current_power_consumption_on_output = self.app.dbus.get_value::<i32>("/Ac/Out/P", INVERTER_DEST).await?;  // Ist positiv
		let current_power_consumption_on_battery = self.app.dbus.get_value::<i32>("/Dc/0/Power", BATTERY_DEST).await?;  // Ist im Minus, wenn entladen wird
		let current_power_consumption = cmp::max(current_power_consumption_on_output, current_power_consumption_on_battery * -1);

		let remaining_battery_wh = self.app.config.battery_wh as f32 * (battery_soc_remaining as f32 / 100.0);
		let remaining_battery_runtime: f32 = Self::calculate_remaining_battery_runtime_secs(remaining_battery_wh, current_power_consumption as f32);
		
		let state_changes = {
			let mut logic_state = self.app.logic_state.write().await;
			let prior_hash = logic_state.calculate_hash_for_status();
			let old_status = logic_state.status.clone();

			logic_state.remaining_battery_percentage = Self::calculate_usv_battery_percentage(battery_soc, self.app.config.inverter_shutdown_soc);
			logic_state.remaining_battery_runtime_secs = remaining_battery_runtime as i32;
			logic_state.status = self.calculate_status().await?;

			if !logic_state.status.eq(&old_status) {
				info!("Status changed from {} to {}", old_status, logic_state.status);
			}

			logic_state.calculate_hash_for_status() != prior_hash
		};

		if state_changes {
			self.tick_shutdown_policies().await;
		}

		if self.app.logic_state.read().await.pending_shutdown_policy_states_save {
			self.store_shutdown_policy_states().await;
			self.app.logic_state.write().await.pending_shutdown_policy_states_save = false;
		}

		self.execute_pending_actions().await;

		Ok(())
	}

	async fn calculate_status(&self) -> Result<LogicStatus, Box<dyn Error>> {
		let state = self.app.dbus.get_value::<u32>("/State", INVERTER_DEST).await?;
		
		match state {
			SYSTEM_STATE_OFF => Ok(LogicStatus::Dead),
			SYSTEM_STATE_FAULT_CONDITION => Ok(LogicStatus::Dead),
			SYSTEM_STATE_BULK_CHARGING => Ok(LogicStatus::HealthyAndCharging(ChargerStatus::Bulk)),
			SYSTEM_STATE_ABSORPTION_CHARGING => Ok(LogicStatus::HealthyAndCharging(ChargerStatus::Absorption)),
			SYSTEM_STATE_FLOAT_CHARGING => Ok(LogicStatus::HealthyAndFull),
			SYSTEM_STATE_STORAGE_MODE => Ok(LogicStatus::HealthyAndFull),
			SYSTEM_STATE_PASS_THROUGH => Ok(LogicStatus::Passthrough),
			SYSTEM_STATE_INVERTING => {
				Ok(LogicStatus::OnBattery)
			},
			SYSTEM_STATE_ASSISTING => Ok(LogicStatus::HealthyAndAssisting),
			SYSTEM_STATE_SUSTAIN => Ok(LogicStatus::HealthyAndCharging(ChargerStatus::Sustain)),

			SYSTEM_STATE_DISCHARGING => Ok(LogicStatus::HealthyAndEss(EssStatus::Discharging)),
			SYSTEM_STATE_SYSTEM_SUSTAIN => Ok(LogicStatus::HealthyAndEss(EssStatus::Charging)),
			SYSTEM_STATE_RECHARGE => Ok(LogicStatus::HealthyAndEss(EssStatus::Charging)),
			SYSTEM_STATE_SCHEDULED_CHARGE => Ok(LogicStatus::HealthyAndEss(EssStatus::Charging)),
			SYSTEM_STATE_DYNAMIC_ESS => Ok(LogicStatus::HealthyAndEss(EssStatus::DynamicEss)),

			_ => Ok(LogicStatus::Unknown)
		}
	}

	fn calculate_remaining_battery_runtime_secs(remaining_battery_wh: f32, current_power_consumption: f32) -> f32 {
		if remaining_battery_wh <= 0.0 {
			0.0
		} else if current_power_consumption <= INTERNAL_INVERTER_CONSUMPTION {
			remaining_battery_wh / INTERNAL_INVERTER_CONSUMPTION * 60.0 * 60.0
		} else {
			remaining_battery_wh / current_power_consumption * 60.0 * 60.0
		}
	}

	fn calculate_usv_battery_percentage(battery_soc: i32, inverter_shutdown_soc: i32) -> i32 {
		let calculated_soc = (battery_soc - inverter_shutdown_soc) as f32 / (100 - inverter_shutdown_soc) as f32 * 100.0;
		cmp::max(calculated_soc as i32, 0)
	}

	async fn tick_shutdown_policies(&self) {
		let mut pending_restarts: Vec<ShutdownPolicy> = Vec::new();

		{
			let mut logic_state = self.app.logic_state.write().await;
			let has_power = logic_state.status.has_power();
			let soc = logic_state.remaining_battery_percentage;
			let mut changes = false;

			for shutdown_policy_state in logic_state.shutdown_policy_states.iter_mut() {
				if shutdown_policy_state.triggered_shutdown {
					if has_power {
						if let Some(restart_soc) = shutdown_policy_state.policy_config.restart_soc {
							if soc >= restart_soc {
								shutdown_policy_state.triggered_shutdown = false;
								if shutdown_policy_state.restart_required {
									shutdown_policy_state.restart_required = false;

									if shutdown_policy_state.is_client_connected(&self.app).await {
										info!("Shutdown Policy {}: Client is already connected, skipping restart ...", shutdown_policy_state.policy_config.name);
									} else {
										info!("Shutdown Policy {}: Restarting ...", shutdown_policy_state.policy_config.name);
										pending_restarts.push(shutdown_policy_state.policy_config.clone());
									}
								} else {
									info!("Shutdown Policy {}: Restart soc reached, but no restart is required", shutdown_policy_state.policy_config.name);
								}
								changes = true;
							}
						} else if soc > shutdown_policy_state.policy_config.shutdown_soc {
							info!("Shutdown Policy {}: Untriggered shutdown", shutdown_policy_state.policy_config.name);
							shutdown_policy_state.triggered_shutdown = false;
							changes = true;
						}
					}
				} else {
					if !has_power && soc <= shutdown_policy_state.policy_config.shutdown_soc {
						info!("Shutdown Policy {}: Triggering shutdown", shutdown_policy_state.policy_config.name);
						shutdown_policy_state.triggered_shutdown = true;
						if shutdown_policy_state.policy_config.restart_soc.is_some() && shutdown_policy_state.is_client_connected(&self.app).await {
							shutdown_policy_state.restart_required = true;
						}
						changes = true;
					}
				}
			}

			if changes {
				logic_state.pending_shutdown_policy_states_save = true;
			}
		}

		for policy_config in pending_restarts.iter() {
			self.restart(&policy_config).await;
		}
	}

	async fn restart(&self, policy_config: &ShutdownPolicy) {
		if let Some(request_config) = policy_config.restart_http_request.clone() {
			match self.app.http.send_request(&request_config).await {
				Ok(()) => info!("Shutdown Policy {}: Successfully sent restart http request", policy_config.name),
				Err(err) => error!("Shutdown Policy {}: Failed to send restart http request: {}", policy_config.name, err)
			}
		}

		if let Some(wol_config) = &policy_config.restart_wol {
			match wake_on_lan::send_wol(wol_config.mac, wol_config.destination).await {
				Ok(()) => info!("Shutdown Policy {}: Sent wake on lan to {}", policy_config.name, wol_config.mac),
				Err(err) => error!("Shutdown Policy {}: Failed to send wake on lan to {} and {}: {}", policy_config.name, wol_config.mac, wol_config.destination, err)
			}
		}
	}

	async fn store_shutdown_policy_states(&self) {
		let snapshot = self.shutdown_policy_states_store.create_snapshot(&self.app.logic_state.read().await.shutdown_policy_states);
		match self.shutdown_policy_states_store.store(&snapshot).await {
			Ok(_) => {
				debug!("Stored shutdown policy states to filesystem.");
			},
			Err(err) => {
				error!("Failed to store shutdown policy states to filesystems: {}", err);
			}
		}
	}

	async fn execute_pending_actions(&self) {
		let mut pending_actions = self.app.pending_logic_actions.lock().await;
		while let Some(action) = pending_actions.pop_front() {
			self.execute_pending_action(&action).await;
		}
	}

	async fn execute_pending_action(&self, pending_action: &PendingLogicAction) {
		match pending_action {
			PendingLogicAction::Restart(shutdown_policy) => {
				self.restart(shutdown_policy).await;
			}
		}
	}
}

pub async fn update_restart_required(app: &Application) {
	let mut logic_state = app.logic_state.write().await;
	let mut changes = false;

	for shutdown_policy_state in logic_state.shutdown_policy_states.iter_mut() {
		if shutdown_policy_state.triggered_shutdown && !shutdown_policy_state.restart_required && shutdown_policy_state.policy_config.restart_soc.is_some() && shutdown_policy_state.is_client_connected(&app).await {
			info!("Shutdown Policy {}: Marked as restart required.", shutdown_policy_state.policy_config.name);
			shutdown_policy_state.restart_required = true;
			changes = true;
		}
	}

	if changes {
		logic_state.pending_shutdown_policy_states_save = true;
	}
}
