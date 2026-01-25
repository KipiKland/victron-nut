use async_trait::async_trait;
use crate::{Application, commands::{command::{Command, CommandExecutionError, validate_usv_name}, command_data::CommandData, response::{CommandResponse, CommandResponseBuilder, EscapedArg}}, connection::Connection, nut::nut_var_provider::NutVarReadScope};

pub struct ListCommand;
#[async_trait]
impl Command for ListCommand {
	async fn execute(&self, conn: &mut Connection, data: &CommandData, app: &Application) -> Result<CommandResponse, CommandExecutionError> {
		let requested_system = data.get_str(0).ok_or(CommandExecutionError::MissingArgument)?;
		
		match requested_system.as_str() {
			"UPS" => {
				Ok(CommandResponseBuilder::new()
					.add_line_array(&[&"BEGIN", &"LIST", &"UPS"])
					.add_line_array(&[&"UPS", &app.config.usv_name, &EscapedArg::new(&app.config.usv_description)])
					.add_line_array(&[&"END", &"LIST", &"UPS"])
					.build())
			},
			"VAR" => {
				validate_usv_name(&data.get_str(1).ok_or(CommandExecutionError::InvalidArgument)?, app)?;

				let read_scope = NutVarReadScope {
					client_data: Some(conn.client_data.read().await.clone()),
					app: app.clone()
				};

				let nut_var_manager = app.nut_var_manager.lock().await;
				let keys = nut_var_manager.list_keys();
				let mut values = futures::future::join_all(keys.iter().rev().map(|k| nut_var_manager.get_value(k, &read_scope))).await;

				let mut response = CommandResponseBuilder::new()
					.add_line_array(&[&"BEGIN", &"LIST", &"VAR", &app.config.usv_name]);
				for key in keys {
					let value = values.pop().unwrap().map_err(|e| e.into())?;
					response = response.add_line_array(&[&"VAR", &app.config.usv_name, &key, &EscapedArg::new(&value)]);
				}
				
				Ok(response.add_line_array(&[&"END", &"LIST", &"VAR", &app.config.usv_name]).build())
			},
			"RW" => {
				validate_usv_name(&data.get_str(1).ok_or(CommandExecutionError::InvalidArgument)?, app)?;

				let response = CommandResponseBuilder::new()
					.add_line_array(&[&"BEGIN", &"LIST", &"RW", &app.config.usv_name])
					.add_line_array(&[&"END", &"LIST", &"RW", &app.config.usv_name]);

				Ok(response.build())
			},
			"CMD" => {
				validate_usv_name(&data.get_str(1).ok_or(CommandExecutionError::InvalidArgument)?, app)?;

				let response = CommandResponseBuilder::new()
					.add_line_array(&[&"BEGIN", &"LIST", &"CMD", &app.config.usv_name])
					.add_line_array(&[&"END", &"LIST", &"CMD", &app.config.usv_name]);

				Ok(response.build())
			},
			"CLIENT" => {
				validate_usv_name(&data.get_str(1).ok_or(CommandExecutionError::InvalidArgument)?, app)?;

				let mut response = CommandResponseBuilder::new()
					.add_line_array(&[&"BEGIN", &"LIST", &"CLIENT", &app.config.usv_name]);

				let clients = app.clients.lock().await;
				for client in clients.iter() {
					let client_data = client.client_data.read().await;
					response = response.add_line_array(&[&"CLIENT", &app.config.usv_name, &client_data.addr.ip().to_string()]);
				}

				Ok(response.add_line_array(&[&"END", &"LIST", &"CLIENT", &app.config.usv_name]).build())
			},
			"ENUM" => {
				Err(CommandExecutionError::InvalidValue)
			},
			"RANGE" => {
				Err(CommandExecutionError::InvalidValue)
			},
			_ => Err(CommandExecutionError::InvalidArgument)
		}
	}
}
