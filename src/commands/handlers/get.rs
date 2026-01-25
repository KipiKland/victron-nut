use async_trait::async_trait;
use crate::{Application, commands::{command::{Command, CommandExecutionError, validate_usv_name}, command_data::CommandData, response::{CommandResponse, CommandResponseBuilder, EscapedArg}}, connection::Connection, extensions::StringExtensions, nut::nut_var_provider::NutVarReadScope};

pub struct GetCommand;
#[async_trait]
impl Command for GetCommand {
	async fn execute(&self, conn: &mut Connection, data: &CommandData, app: &Application) -> Result<CommandResponse, CommandExecutionError> {
		let requested_system = data.get_str(0).ok_or(CommandExecutionError::MissingArgument)?;
		
		match requested_system.as_str() {
			"NUMLOGINS" => {
				validate_usv_name(&data.get_str(1).ok_or(CommandExecutionError::MissingArgument)?, app)?;

				let clients = app.clients.lock().await;
				Ok(CommandResponseBuilder::new().add_line_array(&[&"NUMLOGINS", &app.config.usv_name, &clients.len().to_string()]).build())
			},
			"UPSDESC" => {
				validate_usv_name(&data.get_str(1).ok_or(CommandExecutionError::MissingArgument)?, app)?;

				Ok(CommandResponseBuilder::new().add_line_array(&[&"UPSDESC", &app.config.usv_name, &app.config.usv_description.or("Unavailable")]).build())
			},
			"DESC" => {
				validate_usv_name(&data.get_str(1).ok_or(CommandExecutionError::MissingArgument)?, app)?;

				let var = data.get_str(2).ok_or(CommandExecutionError::MissingArgument)?;
				Ok(CommandResponseBuilder::new().add_line_array(&[&"DESC", &app.config.usv_name, &var, &"Description unavailable"]).build())
			},
			"CMDDESC" => {
				validate_usv_name(&data.get_str(1).ok_or(CommandExecutionError::MissingArgument)?, app)?;

				let cmd = data.get_str(2).ok_or(CommandExecutionError::MissingArgument)?;
				Ok(CommandResponseBuilder::new().add_line_array(&[&"CMDDESC", &app.config.usv_name, &cmd, &"Description unavailable"]).build())
			},
			"TYPE" => {
				// TODO
				Err(CommandExecutionError::InvalidArgument)
			},
			"VAR" => {
				// TODO: Add server.info and server.version variables

				validate_usv_name(&data.get_str(1).ok_or(CommandExecutionError::MissingArgument)?, app)?;
				let var_key = data.get_str(2).ok_or(CommandExecutionError::MissingArgument)?;

				let read_scope = NutVarReadScope {
					client_data: Some(conn.client_data.read().await.clone()),
					app: app.clone()
				};

				let nut_var_manager = app.nut_var_manager.lock().await;
				let var_value = nut_var_manager.get_value(&var_key, &read_scope).await.map_err(|e| e.into())?;

				Ok(CommandResponseBuilder::new().add_line_array(&[&"VAR", &app.config.usv_name, &var_key, &EscapedArg::new(&var_value)]).build())
			}
			_ => Err(CommandExecutionError::InvalidArgument)
		}
	}
}