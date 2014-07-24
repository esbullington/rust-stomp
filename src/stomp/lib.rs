#![desc = "A rust crate for Stomp protocol"]
#![license = "MIT"]

#![crate_name = "stomp"]
#![crate_type = "lib"]

#![feature(phase)]
extern crate debug;
extern crate time;
extern crate collections;
#[phase(plugin, link)]  extern crate log;

use std::io::TcpStream;
use std::collections::HashMap;
use std::fmt;
use std::io::{IoResult, BufferedReader, IoError};


#[deriving(PartialEq, Show)]
pub enum ClientCommand {
  STOMP,
  SEND,
  CONNECT,
  SUBSCRIBE,
  UNSUBSCRIBE,
  ACK,
  NACK,
  BEGIN,
  COMMIT,
  ABORT,
  DISCONNECT,
}

#[deriving(PartialEq, Show)]
pub enum ServerCommand {
  CONNECTED,
  MESSAGE,
  RECEIPT,
  ERROR,
}

#[allow(non_camel_case_types)]
pub enum StompVersion {
  STOMP_1_1,
  STOMP_1_0,
}

impl fmt::Show for StompVersion {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      STOMP_1_1 => write!(f, "STOMP/1.1"),
      STOMP_1_0 => write!(f, "STOMP/1.0"),
    }
  }
}

pub struct Response {
  pub command: ServerCommand,
  pub headers: HashMap<String, String>,
  pub stream: TcpStream,
}

impl Response {

  pub fn with_stream(s: &TcpStream) -> Response {

    let mut stream = BufferedReader::with_capacity(1, s.clone());


    let command = match stream.read_line().unwrap().as_slice().trim() {
      "RECEIPT"     => RECEIPT,
      "CONNECTED"   => CONNECTED,
      "MESSAGE"     => MESSAGE,
      "ERROR"       => fail!("Server error"),
      c             => fail!("Invalid STOMP command here: {:?}", c)
      /*_             => fail!("Invalid STOMP command")*/
    };

    /*let version = match segs[0] {*/
    /*  "1.1"          => STOMP_1_1,*/
    /*  _             => fail!("unsupported STOMP version")*/
    /*};*/

    println!("Got STOMP Response command = {:?}", command);

    let mut headers = HashMap::new();
    loop {
      let line = stream.read_line().unwrap();
      let segs = line.as_slice().splitn(':', 1).collect::<Vec<&str>>();
      if segs.len() == 2 {
        let k = segs[0].trim();
        let v = segs[1].trim();
        headers.insert(k.to_string(), v.into_string());
      } 
      // Godawful hack
      else {
        match line.as_slice() {
          "\n" => continue,
          "\0" => break,
          "\0\n" => break,
          c    => println!("matched {:?}", c)
        }
      }
    }

    Response { command: command, headers: headers, stream: stream.unwrap() }

  }

  pub fn parse_stream_to_string(&mut self) -> String {
    let mut line = std::string::String::new();
    loop {
      match self.stream.read_byte() {
        Ok(0x00)                    => {
          break;
        }
        Ok(c)                       => {
          line.push_char(c as char); 
        }
        Err(_)                      => {}
      };
    };
    line
  }

  // This is an unfortunately inefficient operation
  // But due to lifetime issues, I know of no other way
  pub fn get_header(&self, k: &str) -> String {
    let v = self.headers.get_copy(&k.to_string());
    v.as_slice().to_string()
  }

}

pub struct Request {
  command: String,
  headers: HashMap<String, String>,
  body: String,
  stream: TcpStream
}

impl Request {

  pub fn with_socket(stream: &TcpStream) -> Request {
    let s = stream.clone();
    Request{ command: String::new(), headers: HashMap::new(), body: String::new(), stream: s }
  }

  pub fn set_command(&mut self, command: &str) -> bool {
    self.command = command.to_string();
    true
  }

  pub fn set_header(&mut self, key: &str, value: &str) -> bool {
    self.headers.insert(key.to_string(), value.to_string())
  }

  pub fn set_headers(&mut self, h: HashMap<&str, &str>) -> bool {
    for (k,v) in h.iter() {
      self.set_header(*k, *v);
    } 
    true
  }

  pub fn set_body(&mut self, text: &str) -> bool {
    self.body = text.to_string();
    true
  }

  pub fn set_body_binary(&mut self, text: &[u8]) -> bool {
    self.body = text.to_string();
    true
  }

  #[allow(unused_must_use)]
  pub fn write_request(&self, w: &mut Writer) -> IoResult<Response> {

      // Command
    write!(w, "{}", self.command);

    w.write_str("\n");

    // Headers
    for (k, v) in self.headers.iter() {
        w.write_str(k.as_slice());
        w.write_str(":");
        w.write_str(v.as_slice());
        w.write_str("\n");
    }

    w.write_str("\n\n");

    // Body
    w.write_str(self.body.as_slice());

    w.write_str("\n\0");

    match w.flush() {
      Err(e) => fail!("{}", e),
      Ok(ok) => println!("Write successful: {}", ok)
    };


    let res = Response::with_stream(&self.stream);

    Ok(res)
  }

}

impl fmt::Show for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result {
        write!(f, "Command: {}\nHeaders: {}\nBody: {}\n\n", self.command, self.headers, self.body)
    }
}

/*#[deriving(PartialEq, Show)]*/
/*enum ClientError {*/
/*  ConnectionError,*/
/*}*/

pub struct Client {
  pub stream: TcpStream,
  pub username: String,
  pub password: String,
  pub host: String
}

impl Client {
  
  pub fn with_uri(uri: &str) -> Client {
    let s: String = String::from_str(uri);
    let v: Vec<&str> = s.as_slice().split(':').collect();
    let host = v[0];
    let port = from_str(v[1]).unwrap();
    let stream = TcpStream::connect(host, port).unwrap();
    Client{ stream: stream, username: String::new(), password: String::new(), host: host.to_string() }
  }

  pub fn connect(&mut self, login: &str, passcode: &str) -> Result<Response, IoError> {
    let mut request = {
      Request::with_socket(&mut self.stream)
    };
    request.set_command("CONNECT");
    request.set_header("accept-version", "1.1");
    request.set_header("host", "localhost");
    request.set_header("login", login);
    request.set_header("passcode", passcode);
    let res = {
      request.write_request(&mut self.stream)
    };
    res
  }

  pub fn send_with_receipt(&mut self, topic: &str, text: &str, receipt: &str) -> Result<Response, IoError> {
    let mut request = Request::with_socket(&mut self.stream);
    request.set_command("SEND");
    request.set_header("destination", topic);
    request.set_header("receipt", receipt);
    // Will need to allow user to set content type
    // or else determine it dynamically
    request.set_header("content-type", "text/plain");
    // We have to leave off "content-length" header for now
    // due to ActiveMQ Stomp broker implementation... umm...quirk
    /*let len = text.len();*/
    /*request.set_header("content-length", len.to_string().as_slice());*/
    request.set_body(text);
    request.write_request(&mut self.stream)
  }

  pub fn send(&mut self, topic: &str, text: &str) -> Result<Response, IoError> {
    let mut buf = [0, ..128];
    let amt = {
      let mut wr = std::io::BufWriter::new(buf);
      let t = time::get_time();
      let _ = write!(&mut wr, "default-receipt-{}", t.sec);
      wr.tell().unwrap() as uint
    };
    let s = std::str::from_utf8(buf.slice(0, amt));
    self.send_with_receipt(topic, text, s.unwrap())
  }

  pub fn subscribe_with_id(&mut self, id: &str, topic: &str, ack: &str) -> Result<Response, IoError> {
    let mut request = Request::with_socket(&mut self.stream);
    request.set_command("SUBSCRIBE");
    request.set_header("id", id);
    request.set_header("destination", topic);
    request.set_header("ack", ack);
    request.write_request(&mut self.stream)
  }

  pub fn subscribe(&mut self, destination: &str, ack: &str) -> Result<Response, IoError> {
    let id = "0";
    self.subscribe_with_id(id, destination, ack)
  }

}
