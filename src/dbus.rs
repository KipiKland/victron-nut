use std::error::Error;
use core::fmt;
use zbus::{Connection};
use zbus::{zvariant::OwnedValue};

pub const DBUS_INTERFACE: &str = "com.victronenergy.BusItem";
pub const INVERTER_DEST: &str = "com.victronenergy.vebus.ttyS3";
pub const PLATFORM_DEST: &str = "com.victronenergy.platform";
pub const BATTERY_DEST: &str = "com.victronenergy.battery.socketcan_can0";

#[derive(Debug)]
pub enum DbusReadError {
	ZbusProxyCreationFailed(DbusPath, Box<dyn Error>),
	GetValueCallFailed(DbusPath, Box<dyn Error>),
	ValueConversionFailed(DbusPath, String),
}
unsafe impl Send for DbusReadError {}
impl Error for DbusReadError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			DbusReadError::ZbusProxyCreationFailed(_, err) => Some(&**err),
			DbusReadError::GetValueCallFailed(_, err) => Some(&**err),
			_ => None
		}
	}
}
impl fmt::Display for DbusReadError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			DbusReadError::ZbusProxyCreationFailed(path, err) => {
				write!(f, "Failed to initialize zbus proxy to {}: {}", path, err)
			},
			DbusReadError::GetValueCallFailed(path, err) => {
				write!(f, "Failed to call dbus 'GetValue' to {}: {}", path, err)
			},
			DbusReadError::ValueConversionFailed(path, destination_type) => {
				write!(f, "Failed to cast dbus value to {} from {}", destination_type, path)
			},
		}
	}
}

#[derive(Debug, Clone)]
pub struct DbusPath {
	path: String,
	destination: String
}
impl DbusPath {
	pub fn new(dbus_path: &str, dbus_destination: &str) -> Self {
		Self {
			path: dbus_path.to_string(),
			destination: dbus_destination.to_string()
		}
	}
}
impl fmt::Display for DbusPath {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "(Destination: '{}', Path: '{}')", self.destination, self.path)
	}
}

#[derive(Debug)]
pub struct Dbus {
	pub connection: Connection
}
impl Dbus {
	pub async fn new() -> Result<Self, Box<dyn Error>> {
		let connection = Connection::system().await?;

		Ok(Self {
			connection: connection
		})
	}

	pub async fn get_value<T>(&self, path: &str, destination: &str) -> Result<T, DbusReadError> where T: TryFrom<OwnedValue> {
		self.get_value_by_path(DbusPath::new(path, destination)).await
	}

	pub async fn get_value_by_path<T>(&self, path: DbusPath) -> Result<T, DbusReadError> where T: TryFrom<OwnedValue> {
		let proxy = zbus::Proxy::new(&self.connection, path.destination.as_str(), path.path.as_str(), DBUS_INTERFACE).await.map_err(|e| DbusReadError::ZbusProxyCreationFailed(path.clone(), Box::new(e)))?;
		let value: OwnedValue = proxy.call("GetValue", &()).await.map_err(|e| DbusReadError::GetValueCallFailed(path.clone(), Box::new(e)))?;

		let casted_value = <T>::try_from(value).map_err(|_| DbusReadError::ValueConversionFailed(path.clone(), std::any::type_name::<T>().to_string()))?;
		Ok(casted_value)
	}
}
