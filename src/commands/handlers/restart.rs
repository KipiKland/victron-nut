use async_trait::async_trait;
use crate::{Application, commands::{command::{Command, CommandExecutionError, ensure_authenticated}, command_data::CommandData, response::{CommandResponse, CommandResponseBuilder}}, connection::Connection, logic_manager::PendingLogicAction};

pub struct RestartCommand;
#[async_trait]
impl Command for RestartCommand {
	async fn execute(&self, connection: &mut Connection, command: &CommandData, app: &Application) -> Result<CommandResponse, CommandExecutionError> {
        ensure_authenticated(connection, app).await?;

        let shutdown_policy_name = command.get_str(0).ok_or(CommandExecutionError::MissingArgument)?;
        let shutdown_policy = app.config.shutdown_policies.iter().find(|p| p.name.eq(&shutdown_policy_name)).ok_or(CommandExecutionError::InvalidArgument)?;

        {
            let mut pending_logic_actions = app.pending_logic_actions.lock().await;
            pending_logic_actions.push_back(PendingLogicAction::Restart(shutdown_policy.clone()));
        }

		let response = CommandResponseBuilder::new().add_line_array(&[&"OK", &"QUEUED"]);
		Ok(response.build())
	}
}
