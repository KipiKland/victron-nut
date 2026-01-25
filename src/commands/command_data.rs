pub struct CommandData {
	args: Vec<String>
}

impl CommandData {
	pub fn new(args: Vec<String>) -> Self {
		Self {
			args: args
		}
	}

	fn get_arg(&self, arg_index: usize) -> Option<String> {
		if arg_index >= self.args.len() {
			return None
		}
		Some(self.args[arg_index].clone())
	}

	pub fn get_command(&self) -> String {
		self.get_arg(0).unwrap_or(String::new())
	}

	pub fn get_str(&self, arg_index: usize) -> Option<String> {
		self.get_arg(arg_index + 1)
	}

	#[allow(dead_code)]
	pub fn get_u32(&self, arg_index: usize) -> Option<u32> {
		self.get_arg(arg_index + 1).and_then(|v| v.parse().ok())
	}

	pub fn is_args_amount_matched(&self, arg_len: usize) -> Option<()> {
		if self.args.len() == arg_len + 1 {
			return Some(());
		} else {
			return None;
		}
	}
}
