extern crate http;
extern crate kl_http;

use kl_http::{HttpSerialise, MyHttp};
use std::io::Read;

fn main() {
	use std::io::Write;

	let incoming_request = b"GET / HTTP/1.1\r\nuser-agent: Dart/2.0 (dart:io)\r\ncontent-type: text/plain; charset=utf-8\r\naccept-encoding: gzip\r\ncontent-length: 11\r\nhost: 10.0.2.2:8080\r\n\r\nHello world";

	let listening_thread = ::std::thread::spawn(move || {
		let listener =
			::std::net::TcpListener::bind("127.0.0.1:8080").expect("Failed listening connection.");

		for stream in listener.incoming() {
			let mut myhttp = MyHttp::from_tcp_stream(stream.expect("Failed to return stream."));

			println!("{}", String::from_utf8_lossy(&myhttp.request.to_http()));

			let mut response = http::Response::builder();
			response.status(http::StatusCode::OK);
			let response = response
				.body("hello me".as_bytes().to_vec())
				.expect("Couldn't add body");

			myhttp.respond(response);
		}
	});

	let mut s = ::std::net::TcpStream::connect("127.0.0.1:8080").unwrap();

	s.write(incoming_request);

	let mut string = String::new();
	s.read_to_string(&mut string);
	println!("{}", string);

	listening_thread.join().expect("Thread joining failed.");
}
