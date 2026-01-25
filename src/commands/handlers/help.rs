use async_trait::async_trait;
use crate::{Application, commands::{command::{Command, CommandExecutionError}, command_data::CommandData, command_registry, response::{CommandResponse, CommandResponseBuilder}}, connection::Connection};

pub struct HelpCommand;
#[async_trait]
impl Command for HelpCommand {
	async fn execute(&self, _: &mut Connection, _: &CommandData, _: &Application) -> Result<CommandResponse, CommandExecutionError> {
		let commands = command_registry::get_command_names().join(" ");
		let response = CommandResponseBuilder::new().add_line(&format!("Commands: {}", commands));
		Ok(response.build())
	}
}
