# kl-http

[![Build Status](https://travis-ci.com/kurtlawrence/kl-http.svg?branch=master)](https://travis-ci.com/kurtlawrence/kl-http) [![Latest Version](https://img.shields.io/crates/v/kl-http.svg)](https://crates.io/crates/kl-http) [![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/kl-http) [![codecov](https://codecov.io/gh/kurtlawrence/kl-http/branch/master/graph/badge.svg)](https://codecov.io/gh/kurtlawrence/kl-http)

A lightweight converter for taking a `TcpStream` and converting into a `http::Request` or `http::Response`.

While crates such as `tokio` or `hyper` offer great functionality and features, there is extra work in handling `Futures` and parsing into a workable HTTP request or response. This crate is focused on a simple, easy-to-use conversion of a `TcpStream` into a HTTP request. It uses `http` crate to construct the `http::Request` and `http::Response`. It uses a standard `Vec<u8>` as the body of the requests/response.

---

## Example

```rust
extern crate http;
extern crate kl_http;

use kl_http::{HttpRequest, HttpSerialise};
use std::io::BufReader;
use std::io::Write;

let incoming_request = b"GET / HTTP/1.1\r\nuser-agent: Dart/2.0 (dart:io)\r\ncontent-type: text/plain; charset=utf-8\r\naccept-encoding: gzip\r\ncontent-length: 11\r\nhost: 10.0.2.2:8080\r\n\r\nHello world";

let listener = ::std::net::TcpListener::bind("127.0.0.1:8080").unwrap();
let mut http_request = HttpRequest::from_tcp_stream(listener.accept().unwrap()).unwrap();

println!("{}", String::from_utf8_lossy(&http_request.request.to_http()));

let mut response = http::Response::builder();
response.status(http::StatusCode::OK);
let response = response.body("hello me".as_bytes().to_vec()).unwrap();
http_request.respond(response).unwrap();
```