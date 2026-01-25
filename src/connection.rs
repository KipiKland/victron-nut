use std::{error::Error, net::SocketAddr, sync::Arc, time::Instant};
use secure_string::SecureString;
use tokio::{io::{AsyncReadExt, AsyncWriteExt, BufReader}, net::{TcpStream, tcp::{OwnedReadHalf, OwnedWriteHalf}}, sync::{RwLock}};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use log::log;
use crate::{Application, commands::{command::CommandExecutionError, command_data::CommandData, command_reader::{CommandReader, CommandReaderState}, command_registry, response::CommandResponse}, config::User};

#[derive(Debug, Clone)]
pub struct ClientData {
	pub addr: SocketAddr,
	pub username: Option<String>,
	pub password: Option<SecureString>,
	pub authenticated_user: Option<User>
}

pub struct Connection {
	reader: BufReader<OwnedReadHalf>,
	writer: OwnedWriteHalf,
	pub uuid: Uuid,
	pub command_reader: CommandReader,
	pub shutdown_token: CancellationToken,
	pub last_heard: Instant,
	pub client_data: Arc<RwLock<ClientData>>,
}

impl Connection {
	pub fn new(stream: TcpStream, addr: SocketAddr) -> Self {
		let (reader, writer) = stream.into_split();
		let uuid = Uuid::new_v4();
		Self {
			reader: BufReader::new(reader),
			writer: writer,
			uuid: uuid,
			command_reader: CommandReader::new(),
			shutdown_token: CancellationToken::new(),
			last_heard: Instant::now(),
			client_data: Arc::new(RwLock::new(ClientData {
				addr: addr,
				username: None,
				password: None,
				authenticated_user: None
			}))
		}
	}

	pub async fn handle(&mut self, app: &Application) -> Result<(), Box<dyn Error>> {
		let shutdown_childtoken = self.shutdown_token.child_token();
		let mut buffer = [0u8; 512];

		self.log(log::Level::Info, "Opened connection").await;
		app.add_client(self.uuid, self.client_data.clone()).await;

		loop {
			tokio::select! {
				bytes_read_result = self.reader.read(&mut buffer) => {
					let bytes_read = bytes_read_result?;
					if bytes_read == 0 {
						// Connection closed
						break;
					}

					for index in 0..bytes_read {
						self.command_reader.read_char(buffer[index] as char);

						match self.command_reader.state {
							CommandReaderState::EndOfLine => {
								self.command_received(app).await?;
							},
							CommandReaderState::EndOfTransmission => {
								return Ok(());
							},
							CommandReaderState::ParseError => {
								self.log(log::Level::Warn, format!("Parse error occurred: {}", self.command_reader.error.as_ref().map(|s| s.as_str()).unwrap_or("<ERR>")).as_str()).await;
								return Ok(());
							}
							_ => ()
						}
					}
				}
				_ = shutdown_childtoken.cancelled() => {
					break;
				}
			}
		}

		self.log(log::Level::Info, "Closed connection").await;
		self.cleanup(app).await;
		Ok(())
	}

	async fn command_received(&mut self, app: &Application) -> Result<(), Box<dyn Error>> {
		self.log(log::Level::Debug, format!("Received command \"{}\"", self.command_reader.get_current_line()).as_str()).await;
		self.last_heard = Instant::now();

		let command_data = CommandData::new(self.command_reader.args.clone());
		match self.try_to_execute_command(command_data, app).await {
			Ok(response) => {
				for line in response.lines {
					self.send_response(&line).await?;
				}
			},
			Err(err) => {
				self.send_error(&err.get_error_response()).await?;
				self.log(log::Level::Warn, format!("Command \"{}\" produced an error {}", self.command_reader.get_current_line(), err).as_str()).await;
			}
		}

		Ok(())
	}

	async fn try_to_execute_command(&mut self, command_data: CommandData, app: &Application) -> Result<CommandResponse, CommandExecutionError> {
		let command = command_registry::resolve_command(&command_data.get_command()).ok_or(CommandExecutionError::UnknownCommand)?;
		command.execute(self, &command_data, app).await
	}

	pub async fn send_response(&mut self, message: &str) -> Result<(), Box<dyn Error>> {
		self.log(log::Level::Debug, format!("Sent response: \"{}\"", message).as_str()).await;

		let final_message = message.to_string() + "\n";
		self.writer.write_all(final_message.as_bytes()).await?;
		Ok(())
	}

	async fn send_response_args(&mut self, args: &[&str]) -> Result<(), Box<dyn Error>> {
		self.send_response(&args.join(" ")).await
	}

	async fn send_error(&mut self, error: &str) -> Result<(), Box<dyn Error>> {
		self.send_response_args(&["ERR", error]).await
	}

	async fn cleanup(&mut self, app: &Application) {
		app.remove_client(&self.uuid).await;
	}

	pub async fn close(&mut self) {
		match self.writer.shutdown().await {
			Ok(()) => (),
			Err(err) => self.log(log::Level::Error, format!("Failed to close connection: {}", err).as_str()).await
		}
		self.shutdown_token.cancel();
	}

	pub async fn get_log_identifier(&self) -> String {
		let client_data = self.client_data.read().await;
		match &client_data.authenticated_user {
			Some(user) => format!("{} - {}", client_data.addr, user.username),
			None => client_data.addr.to_string()
		}
	}

	pub async fn log(&self, level: log::Level, msg: &str) {
		let log_identifier = self.get_log_identifier().await;
		log!(target: "Connection", level, "Connection {}: {}", log_identifier, msg);
	}
}
