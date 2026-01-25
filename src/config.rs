use std::{fs, net::{IpAddr, SocketAddr}, path::PathBuf};
use mac_address::MacAddress;
use secure_string::SecureString;
use serde::{Serialize, Deserialize};

const DEFAULT_SHUTDOWN_POLICY_NAME: &str = "default";

#[derive(Debug, Deserialize)]
pub struct Configuration {
	pub store_folder_path: PathBuf,
	pub lan_interface_name: String,
	pub usv_name: String,
	pub usv_description: String,
	pub usv_contact: String,
	pub usv_location: String,
	pub inverter_shutdown_soc: i32,
	pub battery_wh: i32,
	pub auth_required: bool,
	#[serde(default = "Vec::new")]
	pub users: Vec<User>,
	#[serde(default = "Vec::new")]
	pub shutdown_policies: Vec<ShutdownPolicy>
}

#[derive(Debug, Deserialize, Clone)]
pub struct User {
	pub username: String,
	#[serde(with = "securestring_format")]
	pub password: SecureString
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct ShutdownPolicy {
	pub name: String,
	pub shutdown_soc: i32,
	pub restart_soc: Option<i32>,
	pub binding_nut_user: Option<String>,
	#[serde(default, with = "ip_address_format")]
	pub binding_nut_ip: Option<IpAddr>,
	pub restart_wol: Option<WakeOnLanConfig>,
	pub restart_http_request: Option<HttpRequestConfig>
}
impl ShutdownPolicy {
	pub fn is_default(&self) -> bool {
		self.name.eq(DEFAULT_SHUTDOWN_POLICY_NAME)
	}
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct WakeOnLanConfig {
	pub mac: MacAddress,
	#[serde(with = "socket_addr_format")]
	pub destination: SocketAddr,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct HttpRequestConfig {
	pub url: String,
	pub basic_auth: Option<String>,
	pub post_data: Option<String>
}

mod securestring_format {
	use secure_string::SecureString;
	use serde::{self, Deserialize, Deserializer};

	pub fn deserialize<'de, D>(deserializer: D) -> Result<SecureString, D::Error> where D: Deserializer<'de> {
		let str = String::deserialize(deserializer)?;
		Ok(SecureString::from(str))
	}
}

impl Configuration {
	pub fn find_user(&self, username: &str, password: &SecureString) -> Option<User> {
		self.users.iter().find(|user| user.username == username && &user.password == password).cloned()
	}
}

pub fn read_configuration(file: &str) -> Configuration {
	let config_text = fs::read_to_string(file).expect("Can't read config file");
	let mut config: Configuration = toml::from_str(&config_text).expect("Can't parse config file");

	if let Some(default_shutdown_policy_index) = config.shutdown_policies.iter().position(|x| x.is_default()) && default_shutdown_policy_index != config.shutdown_policies.len() - 1 {
		let default_policy = config.shutdown_policies.remove(default_shutdown_policy_index);
		config.shutdown_policies.push(default_policy);
	}

	config
}

mod ip_address_format {
	use std::net::IpAddr;
	use serde::{self, Deserialize, Deserializer};

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<IpAddr>, D::Error> where D: Deserializer<'de> {
		let str = Option::<String>::deserialize(deserializer)?;
		match str {
			Some(ip_str) => Ok(Some(ip_str.parse().map_err(serde::de::Error::custom)?)),
			None => Ok(None)
		}
	}
}

mod socket_addr_format {
	use std::net::{IpAddr, SocketAddr};
	use serde::{self, Deserialize, Deserializer};

	pub fn deserialize<'de, D>(deserializer: D) -> Result<SocketAddr, D::Error> where D: Deserializer<'de> {
		let str = String::deserialize(deserializer)?;
		let port_delimiter = str.find(':').ok_or(serde::de::Error::custom("Port number is required. Use <ip>:<port>"))?;
		
		let ip_addr: IpAddr = str[..port_delimiter].parse().map_err(serde::de::Error::custom)?;
		let port: u16 = str[(port_delimiter + 1)..].parse().map_err(serde::de::Error::custom)?;
		Ok(SocketAddr::new(ip_addr, port))
	}
}