pub trait StringExtensions {
	fn or(&self, or: &str) -> String;
}

impl StringExtensions for String {
	fn or(&self, or: &str) -> String {
		if self.is_empty() {
			String::from(or)
		} else {
			self.clone()
		}
	}
}