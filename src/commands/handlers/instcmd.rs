use async_trait::async_trait;
use crate::{Application, commands::{command::{Command, CommandExecutionError, ensure_authenticated}, command_data::CommandData, response::CommandResponse}, connection::Connection};

pub struct InstCmdCommand;
#[async_trait]
impl Command for InstCmdCommand {
	async fn execute(&self, connection: &mut Connection, _: &CommandData, app: &Application) -> Result<CommandResponse, CommandExecutionError> {
		ensure_authenticated(connection, app).await?;
		Err(CommandExecutionError::InstCmdFailed)
	}
}
