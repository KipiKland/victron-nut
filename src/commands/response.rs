pub const CHARS_TO_ESCAPE: &str = "#\\\"";

pub struct CommandResponse {
	pub lines: Vec<String>
}

pub trait CommandResponseArg {
	fn get_str(&self) -> &str;
	fn is_escape_required(&self) -> bool;
}
impl CommandResponseArg for &'static str {
	fn get_str(&self) -> &str {
		self
	}
	fn is_escape_required(&self) -> bool {
		false
	}
}
impl CommandResponseArg for String {
	fn get_str(&self) -> &str {
		self.as_str()
	}
	fn is_escape_required(&self) -> bool {
		false
	}
}
pub struct EscapedArg {
	str: String
}
impl EscapedArg {
	pub fn new(str: &str) -> Self {
		Self {
			str: str.to_string()
		}
	}
}
impl CommandResponseArg for EscapedArg {
	fn get_str(&self) -> &str {
		self.str.as_str()
	}
	fn is_escape_required(&self) -> bool {
		true
	}
}

pub struct CommandResponseBuilder {
	lines: Vec<String>
}
impl CommandResponseBuilder {
	pub fn new() -> Self {
		Self {
			lines: Vec::new()
		}
	}

	pub fn build_ok() -> CommandResponse {
		Self::new().add_line("OK").build()
	}

	pub fn add_line(mut self, line: &str) -> Self {
		self.lines.push(line.to_string());
		self
	}

	pub fn add_line_array(self, args: &[&dyn CommandResponseArg]) -> Self {
		self.add_line(&args.iter().map(|a| Self::escape_arg(*a)).collect::<Vec<String>>().join(" "))
	}

	fn escape_arg(arg: &dyn CommandResponseArg) -> String {
		let str = arg.get_str();
		let mut result: Vec<char> = Vec::with_capacity(str.len());
		let mut require_quotes = arg.is_escape_required();

		for char in str.chars() {
			if CHARS_TO_ESCAPE.contains(char) {
				result.push('\\');
				require_quotes = true;
			}
			if char.is_whitespace() {
				require_quotes = true;
			}
			result.push(char);
		}

		if require_quotes {
			result.insert(0, '\"');
			result.push('\"');
		}
		result.iter().collect()
	}

	pub fn build(self) -> CommandResponse {
		CommandResponse {
			lines: self.lines
		}
	}
}