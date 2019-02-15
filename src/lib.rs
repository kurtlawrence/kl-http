//! # kl-http
//!
//! [![Build Status](https://travis-ci.com/kurtlawrence/kl-http.svg?branch=master)](https://travis-ci.com/kurtlawrence/kl-http) [![Latest Version](https://img.shields.io/crates/v/kl-http.svg)](https://crates.io/crates/kl-http) [![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/kl-http) [![codecov](https://codecov.io/gh/kurtlawrence/kl-http/branch/master/graph/badge.svg)](https://codecov.io/gh/kurtlawrence/kl-http)
//!
//! A lightweight converter for taking a `TcpStream` and converting into a `http::Request` or `http::Response`.
//!
//! While crates such as `tokio` or `hyper` offer great functionality and features, there is extra work in handling `Futures` and parsing into a workable HTTP request or response. This crate is focused on a simple, easy-to-use conversion of a `TcpStream` into a HTTP request. It uses `http` crate to construct the `http::Request` and `http::Response`. It uses a standard `Vec<u8>` as the body of the requests/response.
//!
//! ---
//!
//! ## Example
//!
//! ```ignore
//! extern crate http;
//! extern crate kl_http;
//!
//! use kl_http::{HttpRequest, HttpSerialise};
//! use std::io::BufReader;
//! use std::io::Write;
//!
//! let incoming_request = b"GET / HTTP/1.1\r\nuser-agent: Dart/2.0 (dart:io)\r\ncontent-type: text/plain; charset=utf-8\r\naccept-encoding: gzip\r\ncontent-length: 11\r\nhost: 10.0.2.2:8080\r\n\r\nHello world";
//!
//! let listener = ::std::net::TcpListener::bind("127.0.0.1:8080").unwrap();
//! let mut http_request = HttpRequest::from_tcp_stream(listener.accept().unwrap().0).unwrap();
//!
//! println!("{}", String::from_utf8_lossy(&http_request.request.to_http()));
//!
//! let mut response = http::Response::builder();
//! response.status(http::StatusCode::OK);
//! let response = response.body("hello me".as_bytes().to_vec()).unwrap();
//! http_request.respond(response).unwrap();
//! ```
extern crate http;
extern crate httparse;

#[cfg(test)]
mod tests;

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

pub use http::Response;

/// Represents a HTTP request.
///
/// The http structure which contains the parsed `http::Request`.
/// Can be used to respond with a `http::Response`.
pub struct HttpRequest {
	tcp_stream: TcpStream,
	pub request: http::Request<Vec<u8>>,
}

impl HttpRequest {
	/// Creates new `HttpRequest` from the incoming stream, taking ownership of the `TcpStream` in the process.
	///
	/// # Example
	/// ```ignore
	/// let mut stream = ::std::net::TcpStream::connect("127.0.0.1:8080").unwrap();
	///
	/// let http_request = kl_http::HttpRequest::from_tcp_stream(stream);
	/// ```
	pub fn from_tcp_stream(stream: TcpStream) -> Result<Self, HttpRequestError> {
		let request = {
			let mut reader = BufReader::new(&stream);
			parse_into_request(&mut reader)
		};

		let request = request?;

		Ok(HttpRequest {
			tcp_stream: stream,
			request: request,
		})
	}

	/// Responds to the request by writing back to the owned `TcpStream` with a `http::Response`.
	///
	/// # Implementation notes
	/// If the `http::Response` does not contain a header `"content-length"`, a header will be added using the Body(`Vec<u8>`) length.
	///
	/// # Example
	/// ```ignore
	/// let mut stream = ::std::net::TcpStream::connect("127.0.0.1:8080").unwrap();
	/// let mut http_request = kl_http::HttpRequest::from_tcp_stream(stream).unwrap();
	///
	/// let mut response = http::Response::builder();
	///	response.status(http::StatusCode::OK);
	///	let response = response
	///		.body("Hello, world".as_bytes().to_vec())
	///		.unwrap();
	///
	///	http_request.respond(response);
	/// ```
	pub fn respond(
		&mut self,
		mut response: http::Response<Vec<u8>>,
	) -> Result<(), HttpRequestError> {
		if !response.headers().iter().any(|x| x.0 == "content-length") {
			// i want to add in a content length if there is a body
			let body_len = response.body().len();
			response.headers_mut().insert(
				"content-length",
				http::header::HeaderValue::from_bytes(body_len.to_string().as_bytes())
					.expect("Failed reading a usize into string? This shouldn't happen."),
			);
		}
		let response_bytes: Vec<u8> = response.to_http();

		self.tcp_stream.write(&response_bytes)?;

		Ok(())
	}
}

/// Takes a readable item and returns a `http::Request`.
///
/// Reading `TcpStream` is inefficient ([see here](https://doc.rust-lang.org/stable/std/io/struct.BufReader.html)),
/// so parsing a `BufRead` is used to convert a `TcpStream` into a `Request` or `Response`.
///
/// The body of the request is read into `Vec<u8>` using the number of bytes that is contained in
/// the `"content-length"` header item. If this item does not exist there will be no body.
///
/// # Example
/// ``` rust
/// let incoming_request = b"GET / HTTP/1.1\r\nuser-agent: Dart/2.0 (dart:io)\r\ncontent-type: text/plain; charset=utf-8\r\naccept-encoding: gzip\r\ncontent-length: 11\r\nhost: 10.0.2.2:8080\r\n\r\nHello, world";
/// let mut incoming_request = &incoming_request[..];
/// let request = kl_http::parse_into_request(&mut incoming_request);
///
/// assert_eq!(request.unwrap().method(), "GET");
/// ```
pub fn parse_into_request<R>(mut reader: &mut R) -> Result<http::Request<Vec<u8>>, HttpRequestError>
where
	R: BufRead,
{
	let request_bytes = read_head(&mut reader)?;

	let mut headers = [httparse::EMPTY_HEADER; 16];
	let mut http_parse_request = httparse::Request::new(&mut headers);
	http_parse_request.parse(&request_bytes)?;
	let body_length: usize = match http_parse_request
		.headers
		.iter()
		.find(|&&header| header.name == "content-length")
	{
		Some(header) => String::from_utf8_lossy(header.value).parse()?,
		None => 0,
	};

	let mut request = http::Request::builder();
	if let Some(method) = http_parse_request.method {
		request.method(method);
	}
	if let Some(path) = http_parse_request.path {
		request.uri(path);
	}
	request.version(http::Version::HTTP_11);

	for kvp in http_parse_request.headers {
		request.header(kvp.name, kvp.value);
	}

	let body: Vec<u8> = {
		let mut body = vec![0u8; body_length];
		reader.read_exact(&mut body)?;

		body
	};

	let request = request.body(body)?;

	Ok(request)
}

/// Takes a readable item and returns a `http::Response`.
///
/// Reading `TcpStream` is inefficient ([see here](https://doc.rust-lang.org/stable/std/io/struct.BufReader.html)),
/// so parsing a `BufRead` is used to convert a `TcpStream` into a `Request` or `Response`.
///
/// The body of the request is read into `Vec<u8>` using the number of bytes that is contained in
/// the `"content-length"` header item. If this item does not exist there will be no body.
///
/// # Example
/// ``` rust
/// extern crate http;
/// extern crate kl_http;
///
/// let incoming_response = b"HTTP/1.1 200 OK\r\ncontent-length: 12\r\n\r\nHello, world";
/// let mut incoming_response = &incoming_response[..];
/// let request = kl_http::parse_into_response(&mut incoming_response).unwrap();
///
/// assert_eq!(request.status(), http::StatusCode::OK);
/// assert_eq!(request.body(), &b"Hello, world".iter().map(|x| *x).collect::<Vec<u8>>());
/// ```
pub fn parse_into_response<R>(
	mut reader: &mut R,
) -> Result<http::Response<Vec<u8>>, HttpRequestError>
where
	R: BufRead,
{
	let response_bytes = read_head(&mut reader)?;
	let mut headers = [httparse::EMPTY_HEADER; 16];
	let mut http_parse_response = httparse::Response::new(&mut headers);
	http_parse_response.parse(&response_bytes)?;
	let body_length: usize = match http_parse_response
		.headers
		.iter()
		.find(|&&header| header.name == "content-length")
	{
		Some(header) => String::from_utf8_lossy(header.value).parse()?,
		None => 0,
	};

	let mut response = http::Response::builder();
	response.version(http::Version::HTTP_11);

	for kvp in http_parse_response.headers {
		response.header(kvp.name, kvp.value);
	}

	let body: Vec<u8> = {
		let mut body = vec![0u8; body_length];
		reader.read_exact(&mut body)?;

		body
	};

	let response = response.body(body)?;

	Ok(response)
}

fn read_head<R>(reader: &mut R) -> Result<Vec<u8>, HttpRequestError>
where
	R: BufRead,
{
	let mut buff = Vec::new();
	let mut read_bytes = reader.read_until(b'\n', &mut buff)?;
	while read_bytes > 0 {
		read_bytes = reader.read_until(b'\n', &mut buff)?;
		if read_bytes == 2 && &buff[(buff.len() - 2)..] == b"\r\n" {
			break;
		}
	}

	Ok(buff)
}

/// A trait that can serialise into a HTTP request or response ready for transfer.
pub trait HttpSerialise {
	/// Serialise into a byte vector HTTP request or response.
	fn to_http(&self) -> Vec<u8>;
}

impl HttpSerialise for http::Request<Vec<u8>> {
	/// Serialise a `http::Request<Vec<u8>>` into a HTTP request.
	///
	/// # Example
	/// ```rust
	/// extern crate http;
	/// extern crate kl_http;
	///
	/// use kl_http::HttpSerialise;
	///
	/// let mut request = http::Request::builder();
	///	request.method(http::Method::GET);
	///	request.header("content-length", "12");
	/// let request: http::Request<Vec<u8>> = request.body(b"Hello, world".to_vec()).unwrap();
	///
	/// let http = request.to_http();
	/// assert_eq!(
	///		http,
	/// 	b"GET / HTTP/1.1\r\ncontent-length: 12\r\n\r\nHello, world".to_vec()
	/// );
	/// ```
	fn to_http(&self) -> Vec<u8> {
		let first_line = format!("{} {} {:?}\r\n", self.method(), self.uri(), self.version());
		let iter = first_line.as_bytes().iter();

		let mut headers = Vec::new();

		for header in self.headers() {
			headers.extend_from_slice(header.0.as_str().as_bytes());
			headers.push(b':');
			headers.push(b' ');
			headers.extend_from_slice(header.1.as_bytes());
			headers.push(b'\r');
			headers.push(b'\n');
		}

		let iter = iter.chain(&headers);

		let iter = iter.chain(b"\r\n").chain(self.body());

		let ret: Vec<u8> = iter.map(|x| *x).collect();
		ret
	}
}

#[test]
fn test_http_request() {
	let mut request = http::Request::builder();
	request.method(http::Method::GET);
	request.header("content-length", "12");
	let request: http::Request<Vec<u8>> = request.body(b"Hello, world".to_vec()).unwrap();

	let http = request.to_http();
	println!("{}", String::from_utf8_lossy(&http));
	assert_eq!(
		http,
		b"GET / HTTP/1.1\r\ncontent-length: 12\r\n\r\nHello, world".to_vec()
	);
}

impl HttpSerialise for http::Response<Vec<u8>> {
	/// Serialise a `http::Response<Vec<u8>>` into a HTTP response.
	///
	/// # Example
	/// ```rust
	/// extern crate http;
	/// extern crate kl_http;
	///
	/// use kl_http::HttpSerialise;
	///
	/// let mut response = http::Response::builder();
	/// response.status(http::StatusCode::OK);
	/// response.header("content-length", "12");
	/// let response: http::Response<Vec<u8>> = response.body(b"Hello, world".to_vec()).unwrap();
	///
	/// let http = response.to_http();
	/// assert_eq!(
	/// 	http,
	/// 	b"HTTP/1.1 200 OK\r\ncontent-length: 12\r\n\r\nHello, world".to_vec()
	/// );
	/// ```
	fn to_http(&self) -> Vec<u8> {
		let first_line = format!("{:?} {}\r\n", self.version(), self.status());
		let iter = first_line.as_bytes().iter();

		let mut headers = Vec::new();

		for header in self.headers() {
			headers.extend_from_slice(header.0.as_str().as_bytes());
			headers.push(b':');
			headers.push(b' ');
			headers.extend_from_slice(header.1.as_bytes());
			headers.push(b'\r');
			headers.push(b'\n');
		}

		let iter = iter.chain(&headers);

		let iter = iter.chain(b"\r\n").chain(self.body());

		let ret: Vec<u8> = iter.map(|x| *x).collect();
		ret
	}
}

#[test]
fn test_http_response() {
	let mut response = http::Response::builder();
	response.status(http::StatusCode::OK);
	response.header("content-length", "12");
	let response: http::Response<Vec<u8>> = response.body(b"Hello, world".to_vec()).unwrap();

	let http = response.to_http();
	println!("{}", String::from_utf8_lossy(&http));
	assert_eq!(
		http,
		b"HTTP/1.1 200 OK\r\ncontent-length: 12\r\n\r\nHello, world".to_vec()
	);
}

/// Represents any errors regarding `HttpRequest`
#[derive(Debug)]
pub enum HttpRequestError {
	ParsingError(String),
	ContentLengthParsingError(String),
	IOError(String),
	BodyWritingError(String),
}

impl Display for HttpRequestError {
	fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
		write!(f, "{:?}", self)
	}
}

impl Error for HttpRequestError {
	fn description(&self) -> &str {
		match self {
			&HttpRequestError::ParsingError(_) => "A stream parsing error occurred.",
			&HttpRequestError::ContentLengthParsingError(_) => {
				"The content length failed parsing into an integer."
			}
			&HttpRequestError::IOError(_) => "A IO error occurred.",
			&HttpRequestError::BodyWritingError(_) => "Failed to write http body.",
		}
	}
}

impl From<httparse::Error> for HttpRequestError {
	fn from(other: httparse::Error) -> Self {
		HttpRequestError::ParsingError(other.description().to_string())
	}
}

impl From<std::num::ParseIntError> for HttpRequestError {
	fn from(other: std::num::ParseIntError) -> Self {
		HttpRequestError::ContentLengthParsingError(format!("{}", other))
	}
}

impl From<std::io::Error> for HttpRequestError {
	fn from(other: std::io::Error) -> Self {
		HttpRequestError::IOError(other.description().to_string())
	}
}

impl From<http::Error> for HttpRequestError {
	fn from(other: http::Error) -> Self {
		HttpRequestError::BodyWritingError(other.description().to_string())
	}
}
