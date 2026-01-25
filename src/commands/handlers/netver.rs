use async_trait::async_trait;
use crate::{Application, commands::{command::{Command, CommandExecutionError}, command_data::CommandData, response::{CommandResponse, CommandResponseBuilder}}, connection::Connection};

pub struct NetVerCommand;
#[async_trait]
impl Command for NetVerCommand {
	async fn execute(&self, _: &mut Connection,_: &CommandData, _: &Application) -> Result<CommandResponse, CommandExecutionError> {
		let response = CommandResponseBuilder::new().add_line_array(&[&"1.2"]);
		Ok(response.build())
	}
}
