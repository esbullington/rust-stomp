extern crate debug;

use std::io::TcpStream;
use std::collections::HashMap;
use std::fmt;
use std::io::IoResult;



struct Frame {
  command: String,
  headers: HashMap<String, String>,
  body: String
}

impl Frame {

  fn new(command: String) -> Frame {
    Frame{ command: command, headers: HashMap::new(), body: String::new() }
  }

  pub fn setHeader(&mut self, key: String, value: String) {
    self.headers.insert(key, value);
  }

  pub fn setBody(&mut self, text: String) {
    self.body = text;
  }

  pub fn writeRequest(&self, w: &mut Writer) -> IoResult<()> {
    
      // Command
      write!(w, "{} ", self.command);

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

      Ok(())
  }

}

impl fmt::Show for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result {
        write!(f, "Command: {}\nHeaders: {}\nBody: {}\n\n", self.command, self.headers, self.body)
    }
}


fn main() {
  let mut stream = TcpStream::connect("localhost", 61613).unwrap();
  let mut frame = Frame::new("CONNECT".to_string());
  frame.setHeader("accept-version".to_string(), "1.1".to_string());
  frame.setHeader("host".to_string(), "localhost".to_string());
  frame.setBody("Hello from Rust".to_string());
  let mut writer = stream.clone();
  frame.writeRequest(&mut writer);
  println!("Frame: {}", frame);
  let mut line = std::string::String::new();
  loop {
    match stream.read_byte() {
      Ok(0x00)                    => {
        break;
      }
      Ok(c)                       => {
        line.push_char(c as char); 
      }
      /*Ok(c)                       => {*/
      /*    fail!("malformat: reads={:?} next={:?}", line, c);*/
      /*}*/
      Err(_)                      => {}
    };
  };
  let s = line.as_slice();
  println!("{}", s);
  drop(stream); // close the connection
}
