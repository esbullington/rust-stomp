#![feature(phase)]
extern crate debug;
extern crate collections;
#[phase(plugin, link)]  extern crate log;

use std::io::TcpStream;
use std::collections::HashMap;
use std::fmt;
use std::io::{IoResult, BufferedReader};


#[deriving(PartialEq, Show)]
pub enum ClientCommand {
  STOMP,
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

pub struct Response<'a> {
  pub command: ServerCommand,
  pub headers: HashMap<String, Vec<String>>,
  pub stream: TcpStream,
}

impl<'a> Response<'a> {

  pub fn with_stream(s: &TcpStream) -> Response {
    let mut stream = BufferedReader::with_capacity(1, s.clone());

    let command = match stream.read_line().unwrap().as_slice().trim() {
      "CONNECTED"   => CONNECTED,
      "MESSAGE"     => MESSAGE,
      "RECEIPT"     => RECEIPT,
      "ERROR"       => fail!("Server error"),
      _             => fail!("Invalid STOMP command")
    };

    /*let version = match segs.get(0) {*/
    /*  "1.1"          => STOMP_1_1,*/
    /*  _             => fail!("unsupported STOMP version")*/
    /*};*/

    println!("Got STOMP Response command = {:?}", command);

    let mut headers = HashMap::new();
    loop {
      let line = stream.read_line().unwrap();
      let segs = line.as_slice().splitn(':', 1).collect::<Vec<&str>>();
      /*println!("Segs is: {}", segs);*/
      if segs.len() == 2 {
        let k = segs.get(0).trim();
        let v = segs.get(1).trim();
        headers.insert_or_update_with(k.to_string(), vec!(v.into_string()),
                        |_k, ov| ov.push(v.into_string()));
      } 
      else {
        if ["\n".to_string(), "\0".to_string()].contains(&line) {
          break;
        }
        /*println!("Fail: {}", segs);*/
        fail!("malformatted line");
      }

    }

    Response { command: command, headers: headers, stream: stream.unwrap() }

  }

  fn parse_stream_to_string(&mut self) -> String {
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

  pub fn get_header<'a>(&'a self, k: &str) ->&'a str {
    let v = self.headers.get(&k.to_string()).as_slice().clone();
    let st = v.get(0);
    let s = st.unwrap();
    s.as_slice()
  }

}

struct Request<'a> {
  command: String,
  headers: HashMap<String, String>,
  body: String,
  stream: TcpStream
}

impl<'a> Request<'a> {

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

  pub fn set_body(&mut self, text: &str) -> bool {
    self.body = text.to_string();
    true
  }

  #[allow(unused_must_use)]
  pub fn write_request(&self, w: &mut Writer) -> IoResult<Response<'a>> {
    
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

impl<'a> fmt::Show for Request<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result {
        write!(f, "Command: {}\nHeaders: {}\nBody: {}\n\n", self.command, self.headers, self.body)
    }
}

struct Client {
  stream: TcpStream,
  username: String,
  password: String
}

impl Client {
  
  fn with_uri(s: &str) {
  }

}


fn main() {
  let mut stream = TcpStream::connect("localhost", 61613).unwrap();
  // REQUEST
  /*let response_reader = writer.clone();*/
  let mut request = Request::with_socket(&stream);
  request.set_command("CONNECT");
  request.set_header("accept-version", "1.1");
  request.set_header("host", "localhost");
  request.set_body("Hello from Rust");
  /*let mut writer = stream.clone();*/
  let response = request.write_request(&mut stream).unwrap();
  let server = response.get_header("server");
  println!("Server: {}", server);
  // Drop request_writer socket
  drop(stream);
  // RESPONSE
  println!("Success! Command: {}", response.command);
  println!("Success! Headers: {}", response.headers);
  /*drop(response.stream); // close the response reader stream*/
}

