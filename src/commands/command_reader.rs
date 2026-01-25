#[derive(Copy, Clone, PartialEq)]
pub enum CommandReaderState {
	FindWordStart,
	FindEol,
	QuoteCollect,
	QcLiteral,
	Collect,
	CollectLiteral,
	EndOfLine,
	EndOfTransmission,
	ParseError,
}

const ARG_LIMIT: usize = 32;
const WORD_LEN_LIMIT: usize = 512;
const END_OF_LINE_CHAR: char = 0xA as char;
const END_OF_TRANSMISSION_CHAR: char = 0x4 as char;

pub struct CommandReader {
	pub args: Vec<String>,
	pub state: CommandReaderState,
	pub error: Option<String>,
	current_word: Vec<char>
}

impl CommandReader {
	pub fn new() -> Self {
		Self {
			args: Vec::new(),
			state: CommandReaderState::FindWordStart,
			error: None,
			current_word: Vec::new()
		}
	}

	#[allow(dead_code)]
	pub fn read_line(&mut self, line: &String) {
		for char in line.chars() {
			self.parse_char(char);

			if self.state == CommandReaderState::EndOfLine {
				return;
			}
		}

		if !self.current_word.is_empty() {
			self.end_of_word();
		}
	}

	pub fn read_char(&mut self, char: char) {
		if self.state == CommandReaderState::EndOfLine || self.state == CommandReaderState::ParseError {
			self.args.clear();
			self.state = CommandReaderState::FindWordStart;
		}

		self.parse_char(char);
	}

	fn parse_char(&mut self, char: char) {
		if char == END_OF_TRANSMISSION_CHAR {
			self.state = CommandReaderState::EndOfTransmission;
			return;
		}

		let new_state = match self.state {
			CommandReaderState::FindWordStart => {
				self.find_word_start(char)
			},
			CommandReaderState::FindEol => {
				self.find_eol(char)
			},
			CommandReaderState::QuoteCollect => {
				self.quote_collect(char)
			},
			CommandReaderState::QcLiteral => {
				self.qc_literal(char)
			},
			CommandReaderState::Collect => {
				self.collect(char)
			},
			CommandReaderState::CollectLiteral => {
				self.collect_literal(char)
			},
			_ => self.state
		};

		if self.state != CommandReaderState::ParseError {
			self.state = new_state;
		}
	}

	fn find_word_start(&mut self, char: char) -> CommandReaderState {
		if char == END_OF_LINE_CHAR {
			return CommandReaderState::EndOfLine;
		}

		if char == '#' {
			return CommandReaderState::FindEol;
		}

		if char.is_whitespace() {
			return CommandReaderState::FindWordStart;
		}

		if char == '\\' {
			return CommandReaderState::CollectLiteral;
		}

		if char == '"' {
			return CommandReaderState::QuoteCollect;
		}

		self.add_char(char);

		if char == '#' {
			self.end_of_word();
			return CommandReaderState::FindWordStart;
		}

		CommandReaderState::Collect
	}

	fn add_char(&mut self, char: char) {
		if !Self::is_allowed_char(char) {
			self.trigger_error(format!("Received invalid character (0x{:02x})", char as u8).as_str());
			return;
		}

		if self.current_word.len() >= WORD_LEN_LIMIT {
			self.trigger_error("Word limit reached");
			return;
		}

		self.current_word.push(char);
	}

	fn end_of_word(&mut self) {
		if self.args.len() >= ARG_LIMIT {
			self.trigger_error("Argument limit reached");
			return;
		}

		self.add_arg();
		self.current_word.clear();
	}

	fn find_eol(&mut self, char: char) -> CommandReaderState {
		if char == END_OF_LINE_CHAR {
			return CommandReaderState::EndOfLine;
		}

		return CommandReaderState::FindEol;
	}

	fn quote_collect(&mut self, char: char) -> CommandReaderState {
		if char == '#' {
			self.trigger_error("Unbalanced word due to unescaped # in quotes");
			return CommandReaderState::ParseError;
		}

		if char == '"' {
			self.end_of_word();
			return CommandReaderState::FindWordStart;
		}

		if char == '\\' {
			return CommandReaderState::QcLiteral;
		}

		self.add_char(char);
		return CommandReaderState::QuoteCollect;
	}

	fn qc_literal(&mut self, char: char) -> CommandReaderState {
		if char == 0xA as char {
			return CommandReaderState::QuoteCollect;
		}

		self.add_char(char);
		return CommandReaderState::QuoteCollect;
	}

	fn collect(&mut self, char: char) -> CommandReaderState {
		if char == '#' {
			self.end_of_word();
			return CommandReaderState::FindEol;
		}

		if char == END_OF_LINE_CHAR {
			self.end_of_word();
			return CommandReaderState::EndOfLine;
		}

		if char.is_whitespace() {
			self.end_of_word();
			return CommandReaderState::FindWordStart;
		}

		if char == '=' {
			self.end_of_word();
			self.find_word_start(char);

			return CommandReaderState::FindWordStart;
		}

		if char == '\\' {
			return CommandReaderState::CollectLiteral;
		}

		self.add_char(char);
		return CommandReaderState::Collect;
	}

	fn collect_literal(&mut self, char: char) -> CommandReaderState {
		if char == END_OF_LINE_CHAR {
			return CommandReaderState::Collect;
		}

		self.add_char(char);
		return CommandReaderState::Collect;
	}

	fn add_arg(&mut self) {
		let str: String = self.current_word.iter().collect();
		self.args.push(str);
	}

	fn trigger_error(&mut self, error: &str) {
		self.error = Some(error.to_string());
		self.state = CommandReaderState::ParseError;
	}

	fn is_allowed_char(char: char) -> bool {
		char >= 0x20 as char && char <= 0x7f as char
	}

	pub fn get_current_line(&self) -> String {
		self.args.join(" ")
	}
}
