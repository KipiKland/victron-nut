use async_trait::async_trait;
use crate::{Application, commands::{command::{Command, CommandExecutionError}, command_data::CommandData, response::{CommandResponse, CommandResponseBuilder}}, connection::Connection};

pub struct VerCommand;
#[async_trait]
impl Command for VerCommand {
	async fn execute(&self, _: &mut Connection, _: &CommandData, _: &Application) -> Result<CommandResponse, CommandExecutionError> {
		let response = CommandResponseBuilder::new().add_line(&format!("Victron NUT server {} - https://www.howaner.de/", env!("CARGO_PKG_VERSION")));
		Ok(response.build())
	}
}
