extern crate http;
extern crate httparse;

mod tests;

use std::net::TcpStream;
use std::io::*;
use std::fmt::{Display, Formatter};
use std::io::Read;

pub struct MyHttp {
	tcp_stream: TcpStream,
	pub request: http::Request<Vec<u8>>,
}

impl MyHttp {
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

pub trait HttpSerialise {
	fn to_http(&self) -> Vec<u8>;
}

impl HttpSerialise for http::Request<Vec<u8>> {
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

impl HttpSerialise for http::Response<Vec<u8>> {
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
