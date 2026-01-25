use async_trait::async_trait;
use crate::{Application, commands::{command::{Command, CommandExecutionError}, command_data::CommandData, response::CommandResponse}, connection::Connection};

pub struct StartTlsCommand;
#[async_trait]
impl Command for StartTlsCommand {
	async fn execute(&self, _: &mut Connection, _: &CommandData, _: &Application) -> Result<CommandResponse, CommandExecutionError> {
		Err(CommandExecutionError::UnknownCommand)
	}
}
