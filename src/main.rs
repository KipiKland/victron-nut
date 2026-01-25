mod config;
mod utility;
mod extensions;
mod commands;
mod connection;
mod dbus;
mod nut;
mod http;
mod storage;
mod logic_manager;

use std::{collections::VecDeque, error::Error, io::Write, sync::Arc, time::Duration};
use env_logger::Builder;
use log::{error, info};
use tokio::{net::TcpListener, sync::{Mutex, RwLock}, time::sleep};
use uuid::Uuid;

use crate::{config::Configuration, connection::{ClientData, Connection}, dbus::Dbus, http::Http, logic_manager::{LogicManager, LogicState, PendingLogicAction}, nut::nut_var_manager::NutVarManager};

const CONFIG_FILEPATH: &str = "victron-nut.conf";

#[derive(Clone)]
pub struct Application {
	pub nut_var_manager: Arc<Mutex<NutVarManager>>,
	pub dbus: Arc<Dbus>,
	pub http: Arc<Http>,
	pub config: Arc<Configuration>,
	pub clients: Arc<Mutex<Vec<ConnectedClient>>>,
	pub logic_state: Arc<RwLock<LogicState>>,
	pub pending_logic_actions: Arc<Mutex<VecDeque<PendingLogicAction>>>
}

#[derive(Clone)]
pub struct ConnectedClient {
	uuid: Uuid,
	client_data: Arc<RwLock<ClientData>>
}

impl Application {
	pub async fn add_client(&self, uuid: Uuid, client_data: Arc<RwLock<ClientData>>) {
		{
			let mut items = self.clients.lock().await;
			items.push(ConnectedClient { uuid, client_data });
		}
		logic_manager::update_restart_required(self).await;
	}

	pub async fn remove_client(&self, uuid: &Uuid) {
		let mut items = self.clients.lock().await;
		match items.iter().position(|c| &c.uuid == uuid) {
			Some(client_idx) => {
				items.remove(client_idx);
			}
			None => ()
		}
	}
}

fn init_logging() {
	let mut builder = Builder::from_default_env();
	builder
		.target(env_logger::Target::Stdout)
		.format(|buf, record| writeln!(buf, "[{}] {} - {}", buf.timestamp(), record.level(), record.args()))
    	.filter(None, log::LevelFilter::Info)
		.init();
}

async fn run_updatetick(app: Application) -> Result<(), Box<dyn Error>> {
	let logic_manager = LogicManager::new(app);
	logic_manager.init().await;

	loop {
		match logic_manager.tick().await {
			Err(err) => {
				error!("Error while logic tick: {}", err);
			},
			_ => (),
		};
		sleep(Duration::from_secs(5)).await;
	}
}

async fn run_nut_server(app: Application) -> Result<(), Box<dyn Error>> {
	let listener = TcpListener::bind("0.0.0.0:3493").await?;
	info!("Started tcp server.");

	loop {
		let (socket, addr) = listener.accept().await?;

		let app_instance = app.clone();
		tokio::spawn(async move {
			let mut connection = Connection::new(socket, addr);
			match connection.handle(&app_instance).await.map_err(|err| format!("An error occurred: {}", err)) {
				Ok(()) => {},
				Err(err_msg) => {
					connection.log(log::Level::Error, &err_msg).await;
				}
			}
		});
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	init_logging();

	let config = config::read_configuration(CONFIG_FILEPATH);
	let nut_var_manager = Arc::new(Mutex::new(NutVarManager::new()));
	let dbus = Dbus::new().await?;
	let logic_state = LogicState::new();

	let app = Application {
		nut_var_manager: nut_var_manager,
		dbus: Arc::new(dbus),
		http: Arc::new(Http::new()),
		logic_state: Arc::new(RwLock::new(logic_state)),
		pending_logic_actions: Arc::new(Mutex::new(VecDeque::new())),
		config: Arc::new(config),
		clients: Arc::new(Mutex::new(Vec::new()))
	};

	futures::future::try_join(run_nut_server(app.clone()), run_updatetick(app.clone())).await.map(|_| ())
}
