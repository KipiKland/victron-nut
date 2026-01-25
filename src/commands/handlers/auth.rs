use async_trait::async_trait;
use secure_string::SecureString;
use crate::{Application, commands::{command::{Command, CommandExecutionError, validate_usv_name}, command_data::CommandData, response::{CommandResponse, CommandResponseBuilder}}, connection::Connection, logic_manager};

pub struct UsernameCommand;
#[async_trait]
impl Command for UsernameCommand {
	async fn execute(&self, connection: &mut Connection, data: &CommandData, _: &Application) -> Result<CommandResponse, CommandExecutionError> {
		let username = data.get_str(0).ok_or(CommandExecutionError::InvalidArgument)?;
		let mut client_data = connection.client_data.write().await;
		if client_data.username.is_some() || client_data.authenticated_user.is_some() {
			return Err(CommandExecutionError::AlreadySetUsername);
		}

		client_data.username = Some(username);
		Ok(CommandResponseBuilder::build_ok())
	}
}

pub struct PasswordCommand;
#[async_trait]
impl Command for PasswordCommand {
	async fn execute(&self, connection: &mut Connection, data: &CommandData, _: &Application) -> Result<CommandResponse, CommandExecutionError> {
		let password = data.get_str(0).ok_or(CommandExecutionError::InvalidArgument)?;
		let mut client_data = connection.client_data.write().await;
		if client_data.password.is_some() || client_data.authenticated_user.is_some() {
			return Err(CommandExecutionError::AlreadySetPassword);
		}

		client_data.password = Some(SecureString::from(password));
		Ok(CommandResponseBuilder::build_ok())
	}
}

pub struct LoginCommand;
#[async_trait]
impl Command for LoginCommand {
	async fn execute(&self, connection: &mut Connection, data: &CommandData, app: &Application) -> Result<CommandResponse, CommandExecutionError> {
		validate_usv_name(&data.get_str(0).ok_or(CommandExecutionError::InvalidArgument)?, app)?;

		{
			let mut client_data = connection.client_data.write().await;
			if client_data.authenticated_user.is_some() {
				return Err(CommandExecutionError::AlreadyLoggedIn);
			}

			let username = client_data.username.as_ref().ok_or(CommandExecutionError::AccessDenied)?;
			let password = client_data.password.as_ref().ok_or(CommandExecutionError::AccessDenied)?;

			match app.config.find_user(&username, password) {
				Some(user) => client_data.authenticated_user = Some(user),
				None => {
					connection.log(log::Level::Info, "Failed to authenticate").await;
					return Err(CommandExecutionError::AccessDenied);
				}
			}
			client_data.password.as_mut().map(|pw| pw.zero_out());
		}

		logic_manager::update_restart_required(app).await;
		connection.log(log::Level::Info, "Authenticated").await;
		Ok(CommandResponseBuilder::build_ok())
	}
}

pub struct LogoutCommand;
#[async_trait]
impl Command for LogoutCommand {
	async fn execute(&self, connection: &mut Connection, data: &CommandData, _: &Application) -> Result<CommandResponse, CommandExecutionError> {
		data.is_args_amount_matched(0).ok_or(CommandExecutionError::InvalidArgument)?;

		connection.send_response("OK Goodbye").await.ok();
		connection.close().await;
		Ok(CommandResponseBuilder::new().build())
	}
}
