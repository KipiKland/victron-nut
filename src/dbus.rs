# Dynamic VE.Bus Detection Patch

Replace the contents of `src/dbus.rs` with the following:

```rust
use std::error::Error;
use core::fmt;
use zbus::Connection;
use zbus::zvariant::OwnedValue;

pub const DBUS_INTERFACE: &str = "com.victronenergy.BusItem";
pub const PLATFORM_DEST: &str = "com.victronenergy.platform";
pub const BATTERY_DEST: &str = "com.victronenergy.battery.socketcan_can0";

const VEBUS_PREFIX: &str = "com.victronenergy.vebus.";

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
	pub connection: Connection,
	pub inverter_dest: String,
}

impl Dbus {
	pub async fn new() -> Result<Self, Box<dyn Error>> {
		let connection = Connection::system().await?;

		let inverter_dest = Self::detect_vebus_service(&connection).await?;

		println!("Detected VE.Bus service: {}", inverter_dest);

		Ok(Self {
			connection,
			inverter_dest,
		})
	}

	async fn detect_vebus_service(connection: &Connection) -> Result<String, Box<dyn Error>> {
		let proxy = zbus::fdo::DBusProxy::new(connection).await?;
		let names = proxy.list_names().await?;

		let service = names
			.into_iter()
			.find(|name| name.starts_with(VEBUS_PREFIX))
			.ok_or("No VE.Bus DBus service found")?;

		Ok(service.to_string())
	}

	pub async fn get_value<T>(&self, path: &str, destination: &str) -> Result<T, DbusReadError>
	where
		T: TryFrom<OwnedValue>
	{
		self.get_value_by_path(DbusPath::new(path, destination)).await
	}

	pub async fn get_inverter_value<T>(&self, path: &str) -> Result<T, DbusReadError>
	where
		T: TryFrom<OwnedValue>
	{
		self.get_value(path, self.inverter_dest.as_str()).await
	}

	pub async fn get_value_by_path<T>(&self, path: DbusPath) -> Result<T, DbusReadError>
	where
		T: TryFrom<OwnedValue>
	{
		let proxy = zbus::Proxy::new(
			&self.connection,
			path.destination.as_str(),
			path.path.as_str(),
			DBUS_INTERFACE,
		)
		.await
		.map_err(|e| DbusReadError::ZbusProxyCreationFailed(path.clone(), Box::new(e)))?;

		let value: OwnedValue = proxy
			.call("GetValue", &())
			.await
			.map_err(|e| DbusReadError::GetValueCallFailed(path.clone(), Box::new(e)))?;

		let casted_value = <T>::try_from(value)
			.map_err(|_| DbusReadError::ValueConversionFailed(path.clone(), std::any::type_name::<T>().to_string()))?;

		Ok(casted_value)
	}
}
```

Then replace usages of:

```rust
INVERTER_DEST
```

with:

```rust
self.dbus.inverter_dest.as_str()
```

or preferably:

```rust
self.dbus.get_inverter_value(...)
```

Example conversion:

Before:

```rust
self.dbus.get_value::<i32>("/Mode", INVERTER_DEST).await
```

After:

```rust
self.dbus.get_inverter_value::<i32>("/Mode").await
```

This now automatically detects:

```text
com.victronenergy.vebus.ttyS3
com.victronenergy.vebus.ttyS4
com.victronenergy.vebus.ttyUSB0
```

without hardcoding the device name.
