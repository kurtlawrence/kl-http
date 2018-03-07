extern crate http;
extern crate kl_http;

use kl_http::{HttpRequest, HttpSerialise};
use std::io::BufReader;

fn main() {
	use std::io::Write;

	let incoming_request = b"GET / HTTP/1.1\r\nuser-agent: Dart/2.0 (dart:io)\r\ncontent-type: text/plain; charset=utf-8\r\naccept-encoding: gzip\r\ncontent-length: 11\r\nhost: 10.0.2.2:8080\r\n\r\nHello world";

	let listening_thread = ::std::thread::spawn(move || {
		let listener = ::std::net::TcpListener::bind("127.0.0.1:8080").unwrap();

		for stream in listener.incoming() {
			let mut http_request = HttpRequest::from_tcp_stream(stream.unwrap()).unwrap();

			println!(
				"{}",
				String::from_utf8_lossy(&http_request.request.to_http())
			);

			let mut response = http::Response::builder();
			response.status(http::StatusCode::OK);
			let response = response.body("hello me".as_bytes().to_vec()).unwrap();

			http_request.respond(response).unwrap();
		}
	});

	let mut s = ::std::net::TcpStream::connect("127.0.0.1:8080").unwrap();

	s.write(incoming_request).unwrap();

	let response = {
		let mut reader = BufReader::new(&mut s);
		kl_http::parse_into_response(&mut reader)
	};

	println!("{}", String::from_utf8_lossy(&response.unwrap().to_http()));

	listening_thread.join().expect("Thread joining failed.");
}
