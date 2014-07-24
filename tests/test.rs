#![feature(phase)]
extern crate debug;
extern crate time;
extern crate stomp;
#[phase(plugin, link)]  extern crate log;

use stomp::Client;
use stomp::{CONNECTED, RECEIPT};


// Test CONNECT
#[test]
fn test_connect() {
  let mut client = Client::with_uri("localhost:61613");
  let response = client.connect("user", "pw").unwrap();
  assert_eq!(response.command, CONNECTED);
  drop(client.stream);
}

// Test SEND
#[test]
fn test_send() {
  let mut client = Client::with_uri("localhost:61613");
  let _ = client.connect("user", "pw").unwrap();
  let mut buf = [0, ..128];
  // Temp hack to distinguish consecutive requests over discrete conns
  let amt = {
    let mut wr = std::io::BufWriter::new(buf);
    let t = time::get_time();
    let _ = write!(&mut wr, "testing 123: {}", t.sec);
    wr.tell().unwrap() as uint
  };
  let s = std::str::from_utf8(buf.slice(0, amt));
  let response = client.send("/queue/test", s.unwrap()).unwrap();
  assert_eq!(response.command, RECEIPT);
  let receipt_response = client.send_with_receipt("/queue/test", s.unwrap(), "receipt1234").unwrap();
  let id = receipt_response.get_header("receipt-id");
  assert_eq!(id.as_slice(), "receipt1234");
  assert_eq!(receipt_response.command, RECEIPT);
  drop(client.stream);
}

// Test SUBSCRIBE
#[test]
fn test_subscribe() {
  let mut client = Client::with_uri("localhost:61613");
  let _ = client.connect("user", "pw").unwrap();
  let _ = client.send("/queue/test", "auto").unwrap();
}
