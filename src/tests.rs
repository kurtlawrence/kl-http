use super::*;
extern crate http;

#[cfg(test)]
#[test]
fn main() {
	use std::io::{BufReader, Write};
	use MyHttp;
	use HttpSerialise;
	use http::{Request, Response, StatusCode};

	let incoming_request = b"GET / HTTP/1.1\r\nuser-agent: Dart/2.0 (dart:io)\r\ncontent-type: text/plain; charset=utf-8\r\naccept-encoding: gzip\r\ncontent-length: 11\r\nhost: 10.0.2.2:8080\r\n\r\nHello world";

	let listening_thread = ::std::thread::spawn(move || {
		let listener =
			::std::net::TcpListener::bind("127.0.0.1:8080").expect("Failed listening connection.");

		for stream in listener.incoming() {
			let mut myhttp = MyHttp::from_tcp_stream(stream.expect("Failed to return stream."));

			assert_eq!(
				myhttp.request.to_http(),
				incoming_request.iter().map(|x| *x).collect::<Vec<u8>>()
			);

			let mut response = Response::builder();
			response.status(StatusCode::OK);
			let response = response
				.body("hello me".as_bytes().to_vec())
				.expect("Couldn't add body");

			myhttp.respond(response);
		}
	});

	let mut s = ::std::net::TcpStream::connect("127.0.0.1:8080").unwrap();

	s.write(incoming_request).unwrap();

	let response = {
		let mut reader = BufReader::new(&mut s);
		::parse_into_response(&mut reader)
	};

	assert_eq!(
		response.body(),
		&"hello me"
			.as_bytes()
			.iter()
			.map(|x| *x)
			.collect::<Vec<u8>>()
	);

	//listening_thread.join().expect("Thread joining failed.");
}
