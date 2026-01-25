use crate::nut::nut_var_provider::{NutVarProvider, NutVarProviderChargerStatus, NutVarProviderConfig, NutVarProviderDbus, NutVarProviderDbusF64, NutVarProviderDbusTime, NutVarProviderLogicRemainingBatteryPercentage, NutVarProviderLogicRemainingBatteryRuntime, NutVarProviderMacAddress, NutVarProviderRealPowerConsumption, NutVarProviderStatic, NutVarProviderUpsStatus, NutVarReadScope, NutvarProviderBatteryPercentageLow, NutvarProviderBatteryPercentageRestart, ProviderError};
use crate::dbus::{INVERTER_DEST, BATTERY_DEST, PLATFORM_DEST};

pub struct NutVar {
	key: String,
	provider: Box<dyn NutVarProvider>
}
impl NutVar {
	pub fn new(key: &str, provider: Box<dyn NutVarProvider>) -> Self {
		Self {
			key: key.to_string(),
			provider: provider
		}
	}
}

pub struct NutVarManager {
	variables: Vec<NutVar>
}
unsafe impl Send for NutVarManager {}
impl NutVarManager {
	pub fn new() -> Self {
		let mut manager = Self {
			variables: Vec::new()
		};
		manager.register_var("device.model", Box::new(NutVarProviderDbus::<String>::new("/ProductName", INVERTER_DEST)));
		manager.register_var("device.mfr", Box::new(NutVarProviderStatic::new("Victron")));
		manager.register_var("device.serial", Box::new(NutVarProviderDbus::<String>::new("/Devices/0/SerialNumber", INVERTER_DEST)));
		manager.register_var("device.type", Box::new(NutVarProviderStatic::new("ups")));
		manager.register_var("device.description", Box::new(NutVarProviderConfig::new(|c| c.usv_description.clone())));
		manager.register_var("device.contact", Box::new(NutVarProviderConfig::new(|c| c.usv_contact.clone())));
		manager.register_var("device.location", Box::new(NutVarProviderConfig::new(|c| c.usv_location.clone())));
		manager.register_var("device.part",Box::new(NutVarProviderDbus::<u32>::new("/ProductId", INVERTER_DEST)));
		manager.register_var("device.macaddr", Box::new(NutVarProviderMacAddress));
		manager.register_var("device.uptime", Box::new(NutVarProviderDbus::<u32>::new("/Devices/0/UpTime", INVERTER_DEST)));
		manager.register_var("device.count", Box::new(NutVarProviderStatic::new("1")));
		manager.register_var("device.usb.version", Box::new(NutVarProviderDbus::<u32>::new("/Devices/0/Version", INVERTER_DEST)));

		manager.register_var("ups.status", Box::new(NutVarProviderUpsStatus));
		manager.register_var("ups.time", Box::new(NutVarProviderDbusTime::new("/Device/Time", PLATFORM_DEST, "%H:%M")));
		manager.register_var("ups.date", Box::new(NutVarProviderDbusTime::new("/Device/Time", PLATFORM_DEST, "%Y-%m-%d")));
		manager.register_var("ups.model", Box::new(NutVarProviderDbus::<String>::new("/ProductName", INVERTER_DEST)));
		manager.register_var("ups.mfr", Box::new(NutVarProviderStatic::new("Victron")));
		manager.register_var("ups.serial", Box::new(NutVarProviderDbus::<String>::new("/Devices/0/SerialNumber", INVERTER_DEST)));
		manager.register_var("ups.firmware", Box::new(NutVarProviderDbus::<u32>::new("/Devices/0/FirmwareVersion", INVERTER_DEST)));
		manager.register_var("ups.temperature", Box::new(NutVarProviderDbusF64::new("/System/MaxCellTemperature", BATTERY_DEST)));
		manager.register_var("ups.realpower", Box::new(NutVarProviderRealPowerConsumption));
		manager.register_var("ups.realpower.nominal", Box::new(NutVarProviderDbus::<i32>::new("/Ac/Out/NominalInverterPower", INVERTER_DEST)));
		manager.register_var("ups.beeper.status", Box::new(NutVarProviderStatic::new("disabled")));
		manager.register_var("ups.mode", Box::new(NutVarProviderStatic::new("line-interactive")));
		manager.register_var("ups.start.auto", Box::new(NutVarProviderStatic::new("yes")));
		manager.register_var("ups.start.battery", Box::new(NutVarProviderStatic::new("yes")));
		manager.register_var("ups.shutdown", Box::new(NutVarProviderStatic::new("enabled")));

		manager.register_var("input.voltage", Box::new(NutVarProviderDbusF64::new("/Ac/ActiveIn/L1/V", INVERTER_DEST)));
		manager.register_var("input.current", Box::new(NutVarProviderDbusF64::new("/Ac/ActiveIn/L1/I", INVERTER_DEST)));
		manager.register_var("input.current.nominal", Box::new(NutVarProviderDbusF64::new("/Ac/In/1/CurrentLimit", INVERTER_DEST)));
		manager.register_var("input.realpower", Box::new(NutVarProviderDbus::<i32>::new("/Ac/ActiveIn/P", INVERTER_DEST)));
		manager.register_var("input.phases", Box::new(NutVarProviderDbus::<u32>::new("/Ac/NumberOfPhases", INVERTER_DEST)));

		manager.register_var("output.voltage", Box::new(NutVarProviderDbusF64::new("/Ac/Out/L1/V", INVERTER_DEST)));
		manager.register_var("output.voltage.nominal", Box::new(NutVarProviderDbusF64::new("/Settings/InverterOutputVoltage", INVERTER_DEST)));
		manager.register_var("output.current", Box::new(NutVarProviderDbusF64::new("/Ac/Out/L1/I", INVERTER_DEST)));
		manager.register_var("output.inverter.latency", Box::new(NutVarProviderStatic::new("0.02")));
		manager.register_var("output.frequency.nominal", Box::new(NutVarProviderStatic::new("50")));

		manager.register_var("battery.runtime", Box::new(NutVarProviderLogicRemainingBatteryRuntime));
		manager.register_var("battery.voltage", Box::new(NutVarProviderDbusF64::new("/Devices/0/Diagnostics/UBatVSense", INVERTER_DEST)));
		manager.register_var("battery.voltage.bms", Box::new(NutVarProviderDbusF64::new("/Dc/0/Voltage", BATTERY_DEST)));
		manager.register_var("battery.current", Box::new(NutVarProviderDbusF64::new("/Dc/0/Current", BATTERY_DEST)));
		manager.register_var("battery.power", Box::new(NutVarProviderDbus::<i32>::new("/Dc/0/Power", BATTERY_DEST)));
		manager.register_var("battery.temperature", Box::new(NutVarProviderDbusF64::new("/Dc/0/Temperature", BATTERY_DEST)));
		manager.register_var("battery.charge", Box::new(NutVarProviderLogicRemainingBatteryPercentage));
		manager.register_var("battery.charge.low", Box::new(NutvarProviderBatteryPercentageLow));
		manager.register_var("battery.charge.restart", Box::new(NutvarProviderBatteryPercentageRestart));
		manager.register_var("battery.type", Box::new(NutVarProviderStatic::new("LiFePo4")));
		manager.register_var("battery.protection", Box::new(NutVarProviderStatic::new("yes")));
		manager.register_var("battery.charger.status", Box::new(NutVarProviderChargerStatus));

		manager.register_var("driver.name", Box::new(NutVarProviderStatic::new("Victron-NUT")));
		manager.register_var("driver.version", Box::new(NutVarProviderStatic::new(env!("CARGO_PKG_VERSION"))));
		manager.register_var("driver.version.internal", Box::new(NutVarProviderStatic::new(env!("CARGO_PKG_VERSION"))));
		manager.register_var("driver.version.data", Box::new(NutVarProviderStatic::new("Franz Victron-NUT")));

		manager
	}

	fn register_var(&mut self, key: &str, provider: Box<dyn NutVarProvider>) {
		self.variables.push(NutVar::new(key, provider));
	}

	pub fn list_keys(&self) -> Vec<String> {
		self.variables.iter().map(|v| v.key.clone()).collect()
	}

	pub async fn get_value(&self, key: &str, scope: &NutVarReadScope) -> Result<String, ProviderError> {
		let variable = self.variables.iter().find(|&v| v.key == key).ok_or(ProviderError::MissingData)?;
		Ok(variable.provider.get_value(scope).await?)
	}
}
