use core::fmt;
use std::{cmp, error::Error};
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use mac_address::mac_address_by_name;
use zbus::zvariant::OwnedValue;
use std::marker::PhantomData;
use crate::{Application, commands::command::CommandExecutionError, config::Configuration, connection::ClientData, dbus::{DbusPath, DbusReadError, INVERTER_DEST}, logic_manager::{ChargerStatus, EssStatus, LogicStatus}};

pub struct NutVarReadScope {
	pub client_data: Option<ClientData>,
	pub app: Application
}

#[derive(Debug)]
pub enum ProviderError {
	InternalError(String, Box<dyn Error>),
	MissingData
}
unsafe impl Send for ProviderError {}
impl Error for ProviderError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			ProviderError::InternalError(_, err) => Some(&**err),
			_ => None
		}
	}
}
impl Into<ProviderError> for DbusReadError {
	fn into(self) -> ProviderError {
		ProviderError::InternalError(format!("Failed to query dbus data"), Box::new(self))
	}
}
impl Into<CommandExecutionError> for ProviderError {
	fn into(self) -> CommandExecutionError {
		match &self {
			ProviderError::MissingData => CommandExecutionError::InvalidValue,
			ProviderError::InternalError(_, _) => CommandExecutionError::InternalError(Box::new(self))
		}
	}
}
impl fmt::Display for ProviderError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			ProviderError::InternalError(msg, err) => {
				write!(f, "Provider caused an internal server error '{}' caused by: {}", msg, err)
			},
			ProviderError::MissingData => write!(f, "Provider is missing data")
		}
	}
}

#[async_trait]
pub trait NutVarProvider: Send + Sync {
	async fn get_value(&self, scope: &NutVarReadScope) -> Result<String, ProviderError>;
}

pub struct NutVarProviderStatic {
	value: String
}
impl NutVarProviderStatic {
	pub fn new(value: &str) -> Self {
		Self {
			value: value.to_string()
		}
	}
}
#[async_trait]
impl NutVarProvider for NutVarProviderStatic {
	async fn get_value(&self, _: &NutVarReadScope) -> Result<String, ProviderError> {
		Ok(self.value.clone())
	}
}

pub struct NutVarProviderConfig {
	value_func: Box<dyn Fn(&Configuration) -> String + Send + Sync>
}
impl NutVarProviderConfig {
	pub fn new<F>(value_func: F) -> Self where F: Fn(&Configuration) -> String + Send + Sync + 'static {
		Self {
			value_func: Box::new(value_func)
		}
	}
}
#[async_trait]
impl NutVarProvider for NutVarProviderConfig {
	async fn get_value(&self, scope: &NutVarReadScope) -> Result<String, ProviderError> {
		let value = (&*self.value_func)(&scope.app.config);
		Ok(value)
	}
}

pub struct NutVarProviderDbus<T> where T: fmt::Display + Sync + Send + TryFrom<OwnedValue> {
	dbus_path: DbusPath,
	_phantom: PhantomData<T>,
}
impl<T> NutVarProviderDbus<T> where T: fmt::Display + Sync + Send + TryFrom<OwnedValue> {
	pub fn new(dbus_path: &str, dbus_destination: &str) -> Self {
		Self {
			dbus_path: DbusPath::new(dbus_path, dbus_destination),
			_phantom: PhantomData {},
		}
	}
}
#[async_trait]
impl<T> NutVarProvider for NutVarProviderDbus<T> where T: fmt::Display + Sync + Send + TryFrom<OwnedValue> {
	async fn get_value(&self, scope: &NutVarReadScope) -> Result<String, ProviderError> {
		let value: T = scope.app.dbus.get_value_by_path(self.dbus_path.clone()).await.map_err(|e| e.into())?;
		Ok(value.to_string())
	}
}

pub struct NutVarProviderDbusF64 {
	dbus_path: DbusPath,
}
impl NutVarProviderDbusF64 {
	pub fn new(dbus_path: &str, dbus_destination: &str) -> Self {
		Self {
			dbus_path: DbusPath::new(dbus_path, dbus_destination)
		}
	}
}
#[async_trait]
impl NutVarProvider for NutVarProviderDbusF64 {
	async fn get_value(&self, scope: &NutVarReadScope) -> Result<String, ProviderError> {
		let value: f64 = scope.app.dbus.get_value_by_path(self.dbus_path.clone()).await.map_err(|e| e.into())?;
		Ok(format!("{:.2}", value))
	}
}

pub struct NutVarProviderDbusTime {
	dbus_path: DbusPath,
	format: String,
}
impl NutVarProviderDbusTime {
	pub fn new(dbus_path: &str, dbus_destination: &str, format: &str) -> Self {
		Self {
			dbus_path: DbusPath::new(dbus_path, dbus_destination),
			format: format.to_string()
		}
	}
}
#[async_trait]
impl NutVarProvider for NutVarProviderDbusTime {
	async fn get_value(&self, scope: &NutVarReadScope) -> Result<String, ProviderError> {
		let value: u64 = scope.app.dbus.get_value_by_path(self.dbus_path.clone()).await.map_err(|e| e.into())?;

		let datetime: DateTime<Utc> = Utc.timestamp_opt(value as i64, 0).unwrap();
		let result = datetime.format(&self.format).to_string();
		Ok(result)
	}
}

pub struct NutVarProviderRealPowerConsumption;
#[async_trait]
impl NutVarProvider for NutVarProviderRealPowerConsumption {
	async fn get_value(&self, scope: &NutVarReadScope) -> Result<String, ProviderError> {
		let current_power_consumption_on_output = scope.app.dbus.get_value::<i32>("/Ac/Out/P", INVERTER_DEST).await.map_err(|e| e.into())?;
		let current_power_consumption = cmp::max(current_power_consumption_on_output, 0);
		Ok(current_power_consumption.to_string())
	}
}

pub struct NutVarProviderMacAddress;
#[async_trait]
impl NutVarProvider for NutVarProviderMacAddress {
	async fn get_value(&self, scope: &NutVarReadScope) -> Result<String, ProviderError> {
		let addr = mac_address_by_name(&scope.app.config.lan_interface_name).map_err(|e| ProviderError::InternalError(format!("Failed to get mac address from interface '{}'", &scope.app.config.lan_interface_name), Box::new(e)))?.ok_or(ProviderError::MissingData)?;
		Ok(addr.to_string())
	}
}

pub struct NutVarProviderLogicRemainingBatteryPercentage;
#[async_trait]
impl NutVarProvider for NutVarProviderLogicRemainingBatteryPercentage {
	async fn get_value(&self, scope: &NutVarReadScope) -> Result<String, ProviderError> {
		let logic_state = scope.app.logic_state.read().await;
		Ok(logic_state.remaining_battery_percentage.to_string())
	}
}

pub struct NutVarProviderRemainingBatteryRuntime;
#[async_trait]
impl NutVarProvider for NutVarProviderRemainingBatteryRuntime {
	async fn get_value(&self, scope: &NutVarReadScope) -> Result<String, ProviderError> {
		let logic_state = scope.app.logic_state.read().await;
		Ok(logic_state.remaining_battery_runtime_secs.to_string())
	}
}

pub struct NutvarProviderBatteryPercentageLow;
#[async_trait]
impl NutVarProvider for NutvarProviderBatteryPercentageLow {
	async fn get_value(&self, scope: &NutVarReadScope) -> Result<String, ProviderError> {
		if let Some(client_data) = &scope.client_data {
			let logic_state = scope.app.logic_state.read().await;
			if let Some(shutdown_policy_state) = logic_state.shutdown_policy_states.iter().find(|s| s.is_matching_to_client(client_data)) {
				return Ok(shutdown_policy_state.policy_config.shutdown_soc.to_string());
			}
		}

		Ok("0".to_string())
	}
}

pub struct NutvarProviderBatteryPercentageRestart;
#[async_trait]
impl NutVarProvider for NutvarProviderBatteryPercentageRestart {
	async fn get_value(&self, scope: &NutVarReadScope) -> Result<String, ProviderError> {
		if let Some(client_data) = &scope.client_data {
			let logic_state = scope.app.logic_state.read().await;
			if let Some(shutdown_policy_state) = logic_state.shutdown_policy_states.iter().find(|s| s.is_matching_to_client(client_data)) {
				let restart_soc = shutdown_policy_state.policy_config.restart_soc.unwrap_or(shutdown_policy_state.policy_config.shutdown_soc);
				return Ok(restart_soc.to_string());
			}
		}

		Ok("0".to_string())
	}
}

pub struct NutVarProviderUpsStatus;
#[async_trait]
impl NutVarProvider for NutVarProviderUpsStatus {
	async fn get_value(&self, scope: &NutVarReadScope) -> Result<String, ProviderError> {
		let logic_state = scope.app.logic_state.read().await;
		if logic_state.forced_shutdown {
			return Ok("FSD".to_string());
		}

		if !logic_state.status.has_power() && let Some(client_data) = &scope.client_data {
			if let Some(shutdown_policy_state) = logic_state.shutdown_policy_states.iter().find(|s| s.is_matching_to_client(client_data)) {
				if shutdown_policy_state.triggered_shutdown {
					return Ok("OB LB".to_string());
				}
			}
		}

		match logic_state.status {
			LogicStatus::HealthyAndFull => Ok("OL".to_string()),
			LogicStatus::HealthyAndCharging(_) => Ok("OL".to_string()),
			LogicStatus::HealthyAndEss(_) => Ok("OL".to_string()),
			LogicStatus::HealthyAndAssisting => Ok("BOOST".to_string()),
			LogicStatus::Passthrough => Ok("BYPASS".to_string()),
			LogicStatus::OnBattery => Ok("OB".to_string()),
			LogicStatus::Dead => Ok("OFF".to_string()),
			LogicStatus::Unknown => Err(ProviderError::MissingData)
		}
	}
}

pub struct NutVarProviderChargerStatus;
#[async_trait]
impl NutVarProvider for NutVarProviderChargerStatus {
	async fn get_value(&self, scope: &NutVarReadScope) -> Result<String, ProviderError> {
		let logic_state = scope.app.logic_state.read().await;
		match logic_state.status {
			LogicStatus::HealthyAndFull => Ok("resting".to_string()),
			LogicStatus::HealthyAndCharging(ChargerStatus::Absorption) => Ok("floating".to_string()),
			LogicStatus::HealthyAndCharging(ChargerStatus::Bulk) => Ok("charging".to_string()),
			LogicStatus::HealthyAndCharging(ChargerStatus::Sustain) => Ok("charging".to_string()),
			LogicStatus::HealthyAndAssisting => Ok("discharging".to_string()),
			LogicStatus::HealthyAndEss(EssStatus::Charging) => Ok("charging".to_string()),
			LogicStatus::HealthyAndEss(EssStatus::Discharging) => Ok("discharging".to_string()),
			LogicStatus::HealthyAndEss(EssStatus::DynamicEss) => Ok("discharging".to_string()),
			LogicStatus::HealthyAndEss(EssStatus::Sustain) => Ok("charging".to_string()),
			LogicStatus::OnBattery => Ok("discharging".to_string()),
			LogicStatus::Passthrough | LogicStatus::Unknown | LogicStatus::Dead => Err(ProviderError::MissingData)
		}
	}
}