extern crate http;
extern crate httparse;

#[cfg(test)]

mod tests;

use std::net::TcpStream;
use std::io::*;
use std::fmt::{Display, Formatter};
use std::io::Read;

/// Represents a HTTP request.
///
/// The http structure which contains the parsed `http::Request`.
/// Can be used to respond with a `http::Response`.
pub struct MyHttp {
	tcp_stream: TcpStream,
	pub request: http::Request<Vec<u8>>,
}

impl MyHttp {
	/// Creates new http request from the incoming stream, consuming the stream in the process.
	///
	/// # Example
	/// ``` ignore
	/// let mut stream = ::std::net::TcpStream::connect("127.0.0.1:8080").unwrap();
	///
	/// let http_request = kl_http::MyHttp::from_tcp_stream(stream);
	/// ```
	pub fn from_tcp_stream(stream: TcpStream) -> Self {
		let request = {
			let mut reader = BufReader::new(&stream);
			parse_into_request(&mut reader)
		};

		MyHttp {
			tcp_stream: stream,
			request: request,
		}
	}

	/// Responds to the request by writing back to the owned TcpStream with a http::Response.
	///
	/// # Implementation notes
	/// If the http::Response does not contain a header "content-length", a header will be added using the Body length.
	///
	/// # Example
	/// ``` ignore
	/// extern crate http;
	/// extern crate kl_http;
	///
	/// let mut stream = ::std::net::TcpStream::connect("127.0.0.1:8080").unwrap();
	/// let mut http_request = kl_http::MyHttp::from_tcp_stream(stream);
	///
	/// let mut response = http::Response::builder();
	///	response.status(http::StatusCode::OK);
	///	let response = response
	///		.body("Hello, world".as_bytes().to_vec())
	///		.unwrap();
	///
	///	http_request.respond(response);
	/// ```
	pub fn respond(&mut self, mut response: http::Response<Vec<u8>>) {
		if !response.headers().iter().any(|x| x.0 == "content-length") {
			// i want to add in a content length if there is a body
			let body_len = response.body().len();
			response.headers_mut().insert(
				"content-length",
				http::header::HeaderValue::from_bytes(body_len.to_string().as_bytes()).unwrap(),
			);
		}
		let response_bytes: Vec<u8> = response.to_http();

		self.tcp_stream.write(&response_bytes).expect("Hello");
	}
}

/// Takes a readable item and returns a 'http::Request'.
///
/// Reading TcpStream is inefficient ([see here](https://doc.rust-lang.org/stable/std/io/struct.BufReader.html)),
/// so parsing a BufReader is used to convert a TcpStream into a Request or Response.
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
/// assert_eq!(request.method(), "GET");
/// ```
pub fn parse_into_request<R>(mut reader: &mut R) -> http::Request<Vec<u8>>
where
	R: BufRead,
{
	let request_bytes = read_head(&mut reader);

	let mut headers = [httparse::EMPTY_HEADER; 16];
	let mut req = httparse::Request::new(&mut headers);
	req.parse(&request_bytes).unwrap();
	let body_length: usize = match req.headers
		.iter()
		.find(|&&header| header.name == "content-length")
	{
		Some(header) => String::from_utf8(header.value.to_vec())
			.unwrap()
			.parse()
			.unwrap(),
		None => 0,
	};

	let mut request = http::Request::builder();
	request
		.method(req.method.unwrap())
		.uri(req.path.unwrap())
		.version(http::Version::HTTP_11);

	for kvp in req.headers {
		request.header(kvp.name, kvp.value);
	}

	let body: Vec<u8> = {
		let mut body = vec![0u8; body_length];
		reader
			.read_exact(&mut body)
			.expect("Could not read the body from the stream.");

		body
	};

	request.body(body).unwrap()
}

/// Takes a readable item and returns a 'http::Response'.
///
/// Reading TcpStream is inefficient ([see here](https://doc.rust-lang.org/stable/std/io/struct.BufReader.html)),
/// so parsing a BufReader is used to convert a TcpStream into a 'Request' or 'Response'.
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
/// let request = kl_http::parse_into_response(&mut incoming_response);
///
/// assert_eq!(request.status(), http::StatusCode::OK);
/// assert_eq!(request.body(), &b"Hello, world".iter().map(|x| *x).collect::<Vec<u8>>());
/// ```
pub fn parse_into_response<R>(mut reader: &mut R) -> http::Response<Vec<u8>>
where
	R: BufRead,
{
	let request_bytes = read_head(&mut reader);

	let mut headers = [httparse::EMPTY_HEADER; 16];
	let mut req = httparse::Response::new(&mut headers);
	req.parse(&request_bytes).unwrap();
	let body_length: usize = match req.headers
		.iter()
		.find(|&&header| header.name == "content-length")
	{
		Some(header) => String::from_utf8(header.value.to_vec())
			.unwrap()
			.parse()
			.unwrap(),
		None => 0,
	};

	let mut request = http::Response::builder();
	request.version(http::Version::HTTP_11);

	for kvp in req.headers {
		request.header(kvp.name, kvp.value);
	}

	let body: Vec<u8> = {
		let mut body = vec![0u8; body_length];
		reader
			.read_exact(&mut body)
			.expect("Could not read the body from the stream.");

		body
	};

	request.body(body).unwrap()
}

fn read_head<R>(reader: &mut R) -> Vec<u8>
where
	R: BufRead,
{
	let mut buff = Vec::new();
	let mut read_bytes = reader.read_until(b'\n', &mut buff).unwrap();
	while read_bytes > 0 {
		read_bytes = reader.read_until(b'\n', &mut buff).unwrap();
		if read_bytes == 2 && &buff[(buff.len() - 2)..] == b"\r\n" {
			break;
		}
	}
	return buff;
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
	/// ``` ignore
	/// extern crate http;
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
	/// ``` ignore
	/// extern crate http;
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
