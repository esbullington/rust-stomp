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

pub struct Response<'a> {
  pub command: ServerCommand,
  pub headers: HashMap<String, String>,
  pub stream: TcpStream,
}

impl<'a> Response<'a> {


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

    /*let version = match segs.get(0) {*/
    /*  "1.1"          => STOMP_1_1,*/
    /*  _             => fail!("unsupported STOMP version")*/
    /*};*/

    println!("Got STOMP Response command = {:?}", command);

    let mut headers = HashMap::new();
    loop {
      let line = stream.read_line().unwrap();
      let segs = line.as_slice().splitn(':', 1).collect::<Vec<&str>>();
      if segs.len() == 2 {
        let k = segs.get(0).trim();
        let v = segs.get(1).trim();
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

  // This is an unfortunately inefficient operation
  // But due to lifetime issues, I know of no other way
  pub fn get_header(&self, k: &str) -> String {
    let v = self.headers.get_copy(&k.to_string());
    v.as_slice().to_string()
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

/*#[deriving(PartialEq, Show)]*/
/*enum ClientError {*/
/*  ConnectionError,*/
/*}*/

struct Client {
  stream: TcpStream,
  username: String,
  password: String,
  host: String
}

impl Client {
  
  fn with_uri(uri: &str) -> Client {
    let s: String = String::from_str(uri);
    let v: Vec<&str> = s.as_slice().split(':').collect();
    let host = *v.get(0);
    let port = from_str(*v.get(1)).unwrap();
    let stream = TcpStream::connect(host, port).unwrap();
    Client{ stream: stream, username: String::new(), password: String::new(), host: host.to_string() }
  }

  fn connect(&mut self, login: &str, passcode: &str) -> Result<Response, IoError> {
    let mut request = Request::with_socket(&self.stream);
    request.set_command("CONNECT");
    request.set_header("accept-version", "1.1");
    request.set_header("host", "localhost");
    request.set_header("login", login);
    request.set_header("passcode", passcode);
    request.write_request(&mut self.stream)
  }

  fn send(&mut self, topic: &str, text: &str) -> Result<Response, IoError> {
    let mut request = Request::with_socket(&self.stream);
    request.set_command("SEND");
    request.set_header("destination", topic);
    request.set_header("receipt", "receipt123334");
    // Will need to allow user to set content type
    // or else determine it dynamically
    request.set_header("content-type", "text/plain");
    // We have to leave off "content-length" header
    // due to ActiveMQ Stomp broker implementation... umm...quirk
    /*let len = text.len() + 10;*/
    /*request.set_header("content-length", len.to_string().as_slice());*/
    request.set_body(text);
    request.write_request(&mut self.stream)
  }

}


fn main() {
  /*let mut stream = TcpStream::connect("localhost", 61613).unwrap();*/
  let mut client = Client::with_uri("localhost:61613");
  // Test CONNECT
  let response = client.connect("user", "pw").unwrap();
  /*let server = response.get_header("version");*/
  /*println!("Server: {}", server);*/
  println!("Success! Command: {}", response.command);
  println!("Success! Headers: {}", response.headers);
  // Test SEND
  /*std::io::timer::sleep(2000);*/
  let mut buf = [0, ..128];
  // Temp hack to distinguish consecutive requests over discrete conns
  let amt = {
    let mut wr = std::io::BufWriter::new(buf);
    let t = time::get_time();
    write!(&mut wr, "testing 123: {}", t.sec);
    wr.tell().unwrap() as uint
  };
  let s = std::str::from_utf8(buf.slice(0, amt));
  let r2 = client.send("/queue/test", s.unwrap()).unwrap();
  println!("Success! Command: {}", r2.command);
  println!("Success! Headers: {}", r2.headers);
  drop(client.stream);
}
