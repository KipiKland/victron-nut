use std::{sync::Arc};
use lazy_static::lazy_static;
use crate::commands::{command::Command, handlers::{auth::{LoginCommand, LogoutCommand, PasswordCommand, UsernameCommand}, get::GetCommand, help::HelpCommand, netver::NetVerCommand, restart::RestartCommand, set::SetCommand, starttls::StartTlsCommand, ver::VerCommand}};
use crate::commands::handlers::{instcmd::InstCmdCommand, list::ListCommand};

type CommandFactory = Box<dyn Fn() -> Box<dyn Command> + Send + Sync>;

struct RegisteredCommand {
	prefix: String,
	factory: CommandFactory
}

impl RegisteredCommand {
	fn new<F>(prefix: &str, factory: F) -> Self where F: Fn() -> Box<dyn Command> + Send + Sync + 'static {
		Self {
			prefix: prefix.to_string(),
			factory: Box::new(factory)
		}
	}
}

lazy_static! {
	static ref COMMAND_REGISTRY: Arc<Vec<RegisteredCommand>> = {
		let mut registry: Vec<RegisteredCommand> = Vec::new();
		registry.push(RegisteredCommand::new("LIST", || Box::new(ListCommand)));
		registry.push(RegisteredCommand::new("GET", || Box::new(GetCommand)));
		registry.push(RegisteredCommand::new("INSTCMD", || Box::new(InstCmdCommand)));
		registry.push(RegisteredCommand::new("SET", || Box::new(SetCommand)));
		registry.push(RegisteredCommand::new("NETVER", || Box::new(NetVerCommand)));
		registry.push(RegisteredCommand::new("VER", || Box::new(VerCommand)));
		registry.push(RegisteredCommand::new("HELP", || Box::new(HelpCommand)));
		registry.push(RegisteredCommand::new("USERNAME", || Box::new(UsernameCommand)));
		registry.push(RegisteredCommand::new("PASSWORD", || Box::new(PasswordCommand)));
		registry.push(RegisteredCommand::new("LOGIN", || Box::new(LoginCommand)));
		registry.push(RegisteredCommand::new("LOGOUT", || Box::new(LogoutCommand)));
		registry.push(RegisteredCommand::new("RESTART", || Box::new(RestartCommand)));
		registry.push(RegisteredCommand::new("STARTTLS", || Box::new(StartTlsCommand)));

		Arc::new(registry)
	};
}

fn find_command_registration(cmd: &str) -> Option<&'static RegisteredCommand> {
	COMMAND_REGISTRY.iter().find(|command| cmd.starts_with(&command.prefix))
}

pub fn get_command_names() -> Vec<String> {
	COMMAND_REGISTRY.iter().map(|command| command.prefix.clone()).collect()
}

pub fn resolve_command(cmd: &str) -> Option<Box<dyn Command>> {
	match find_command_registration(cmd) {
		Some(command) => {
			let handler = (&*command.factory)();
			Some(handler)
		},
		None => {
			None
		}
	}
}