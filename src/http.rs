use core::fmt;
use std::error::Error;
use reqwest::{Client, RequestBuilder};

use crate::config::HttpRequestConfig;


#[derive(Debug)]
pub enum HttpError {
	ConnectionError(String, Box<dyn Error>),
	BadStatusCode(String, u16, String),
	ErrorInResponse(String, String)
}
unsafe impl Send for HttpError {}
impl Error for HttpError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			HttpError::ConnectionError(_, err) => Some(&**err),
			_ => None
		}
	}
}
impl fmt::Display for HttpError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			HttpError::ConnectionError(url, err) => {
				write!(f, "Failed to establish connection to http url '{}': {}", url, err)
			},
			HttpError::BadStatusCode(url, status_code, body) => {
				write!(f, "Received bad status code {} while sending http request to '{}', response '{}'", status_code, url, body)
			},
			HttpError::ErrorInResponse(url, body) => {
				write!(f, "Found error in http response to '{}', response '{}'", url, body)
			}
		}
	}
}

pub struct Http {
	client: Client,
}
impl Http {
	pub fn new() -> Self {
		Self {
			client: Client::new()
		}
	}

	fn init_basic_auth(request_builder: RequestBuilder, configured_basic_auth: Option<String>) -> RequestBuilder {
		match configured_basic_auth {
			Some(auth) => {
				let password_delimiter = auth.find(":");
				match password_delimiter {
					Some(delimiter_index) => {
						let username = &auth[..delimiter_index];
						let password = &auth[(delimiter_index + 1)..];

						return request_builder.basic_auth(username, Some(password));
					},
					None => ()
				}
			},
			None => ()
		}

		request_builder
	}

	pub async fn send_request(&self, request_config: &HttpRequestConfig) -> Result<(), HttpError> {
		let mut request_builder = match &request_config.post_data {
			Some(body) => {
				self.client.post(&request_config.url).body(body.clone())
			},
			None => {
				self.client.get(&request_config.url)
			}
		};
		request_builder = Self::init_basic_auth(request_builder, request_config.basic_auth.clone());

		let response = request_builder.send().await.map_err(|err| HttpError::ConnectionError(request_config.url.clone(), Box::new(err)))?;
		let response_status = response.status();
		let response_body = response.text().await.map_err(|err| HttpError::ConnectionError(request_config.url.clone(), Box::new(err)))?;
		
		if !response_status.is_success() {
			return Err(HttpError::BadStatusCode(request_config.url.clone(), response_status.as_u16(), response_body));
		}

		if response_body.contains("error") {
			return Err(HttpError::ErrorInResponse(request_config.url.clone(), response_body));
		}

		Ok(())
	}
}