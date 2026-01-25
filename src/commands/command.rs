use core::fmt;
use std::error::Error;

use async_trait::async_trait;

use crate::{Application, commands::{command_data::CommandData, response::CommandResponse}, connection::Connection};

#[async_trait]
pub trait Command: Send + Sync {
	async fn execute(&self, connection: &mut Connection, data: &CommandData, app: &Application) -> Result<CommandResponse, CommandExecutionError>;
}

#[derive(Debug)]
pub enum CommandExecutionError {
	InternalError(Box<dyn Error>),
	UnknownCommand,
	MissingArgument,
	InvalidArgument,
	InvalidValue,
	AccessDenied,
	InstCmdFailed,
	SetFailed,
	Readonly,
	AlreadySetUsername,
	AlreadySetPassword,
	AlreadyLoggedIn,
	UnknownUps
}
unsafe impl Send for CommandExecutionError {}
impl Error for CommandExecutionError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			CommandExecutionError::InternalError(err) => Some(&**err),
			_ => None
		}
	}
}
impl CommandExecutionError {
	pub fn get_error_response(&self) -> String {
		match self {
			CommandExecutionError::InternalError(_) => "INTERNAL-ERROR".to_string(),
			CommandExecutionError::UnknownCommand => "UNKNOWN-COMMAND".to_string(),
			CommandExecutionError::MissingArgument => "MISSING-ARGUMENT".to_string(),
			CommandExecutionError::InvalidArgument => "INVALID-ARGUMENT".to_string(),
			CommandExecutionError::InvalidValue => "INVALID-VALUE".to_string(),
			CommandExecutionError::AccessDenied => "ACCESS-DENIED".to_string(),
			CommandExecutionError::InstCmdFailed => "INSTCMD-FAILED".to_string(),
			CommandExecutionError::SetFailed => "SET-FAILED".to_string(),
			CommandExecutionError::Readonly => "READONLY".to_string(),
			CommandExecutionError::AlreadySetUsername => "ALREADY-SET-USERNAME".to_string(),
			CommandExecutionError::AlreadySetPassword => "ALREADY-SET-PASSWORD".to_string(),
			CommandExecutionError::AlreadyLoggedIn => "ALREADY-LOGGED-IN".to_string(),
			CommandExecutionError::UnknownUps => "UNKNOWN-UPS".to_string()
		}
	}
}
impl fmt::Display for CommandExecutionError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			CommandExecutionError::InternalError(err) => {
				write!(f, "Command execution error: {}, caused by: {}", self.get_error_response(), err)
			},
			_ => write!(f, "Command execution error: {}", self.get_error_response())
		}
	}
}

pub fn validate_usv_name(usv_name: &str, app: &Application) -> Result<(), CommandExecutionError> {
	if usv_name == app.config.usv_name {
		Ok(())
	} else {
		Err(CommandExecutionError::UnknownUps)
	}
}

pub async fn ensure_authenticated(connection: &Connection, app: &Application) -> Result<(), CommandExecutionError> {
	if app.config.auth_required {
		let client_data = connection.client_data.read().await;
		if client_data.authenticated_user.is_none() {
			return Err(CommandExecutionError::AccessDenied);
		}
	}

	return Ok(());
}