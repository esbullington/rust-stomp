
pub struct Response<'a> {
  pub command: String,
  pub version: StompVersion,
  pub session: int,
  pub server: String,

  chunked: bool,
  chunked_left: Option<uint>,
  pub length: Option<uint>,
  length_left: uint,
  // make sock a owned TcpStream
  // FIXME: maybe a rust bug here
  // when using Buffer/Reader traits here, program will hangs at main() ends
  // gdb shows that epoll_wait with timeout=-1, and pthread_cond_wait()
  sock: TcpStream,
  eof: bool,
}

impl<'a> Response<'a> {
  pub fn with_stream(s: &TcpStream) -> Response {
    let mut stream = BufferedReader::with_capacity(1, s.clone());

    let command = stream.read_line().unwrap();
    let line = stream.read_line().unwrap(); // status line
    let segs = line.as_slice().splitn(' ', 2).collect::<Vec<&str>>();

    let version = match *segs.get(0) {
      "1.1"          => STOMP_1_1,
      "1.0"          => STOMP_1_0,
      _             => fail!("unsupported HTTP version")
    };
    let session = from_str::<int>(*segs.get(1)).unwrap();
    let server = segs.get(2).trim_right();

    debug!("Got HTTP Response version = {:?} status = {:?} reason = {:?}",
         version, status, reason);

    let mut headers = HashMap::new();
    loop {
      let line = stream.read_line().unwrap();
      let segs = line.as_slice().splitn(':', 1).collect::<Vec<&str>>();
      if segs.len() == 2 {
        let k = segs.get(0).trim();
        let v = segs.get(1).trim();
        headers.insert_or_update_with(k.to_ascii_lower(), vec!(v.into_string()),
                        |_k, ov| ov.push(v.into_string()));
      } else {
        if ["\r\n".to_string(), "\n".to_string(), "".to_string()].contains(&line) {
          break;
        }
        fail!("malformatted line");
      }
    }

    let mut chunked = false;
    for (k, v) in headers.iter() {
      if k.as_slice().eq_ignore_ascii_case("transfer-encoding") {
        if v.get(0).as_slice().eq_ignore_ascii_case("chunked") {
          chunked = true;
        }
        break;
      }
    }

    let mut length = None;
    if !chunked {
      length = match headers.find(&"Content-Length".to_ascii_lower()) {
        None => None,
        Some(v) => from_str::<uint>(v.get(0).as_slice()),
      }
    }

    debug!("HTTP Response chunked={} length={}", chunked, length);

    Response { version: version, status: status, reason: reason.into_string(),
           headers: headers,
           chunked: chunked, chunked_left: None,
           length: length, length_left: length.unwrap_or(0),
           sock: s.clone(), eof: false }
  }

  pub fn get_headers(&self, header_name: &str) -> Vec<String> {
    let mut ret = Vec::new();
    match self.headers.find(&header_name.to_ascii_lower()) {
      Some(hdrs) => for hdr in hdrs.iter() {
        ret.push(hdr.clone())
      },
      _ => ()
    }
    ret
  }

  fn read_next_chunk_size(&mut self) -> Option<uint> {
    let mut line = String::new();
    static MAXNUM_SIZE : uint = 16; // 16 hex digits
    static HEX_CHARS : &'static [u8] = bytes!("0123456789abcdefABCDEF");
    let mut is_in_chunk_extension = false;
    loop {
      match self.sock.read_byte() {
        Ok(0x0du8)              => {    // \r\n ends chunk size line
          let lf = self.sock.read_byte().unwrap() as char;
          assert_eq!(lf, '\n');
          break;
        }
        Ok(0x0au8)              => {    // \n ends is dangerous
          warn!("http chunk transfer encoding format: LF without CR.");
          break;
        }
        Ok(_) if is_in_chunk_extension    => { continue; }
        Ok(c) if HEX_CHARS.contains(&c) => { line.push_char(c as char); }
        // `;`
        Ok(0x3bu8)              => { is_in_chunk_extension = true; }
        Ok(c)               => {
          fail!("malformat: reads={:?} next={:?}", line, c);
        }
        Err(_)                => return None,
      }
    }

    if line.len() > MAXNUM_SIZE {
      fail!("http chunk transfer encoding format: size line too long: {:?}", line);
    }
    debug!("read_next_chunk_size, line={:?} value={:?}", line, from_str_radix::<uint>(line.as_slice(), 16));

    match from_str_radix(line.as_slice(), 16) {
      Some(v) => Some(v),
      None => fail!("wrong chunk size value: {:?}", line),
    }
  }
}

impl<'a> Reader for Response<'a> {
  fn read(&mut self, buf: &mut [u8]) -> IoResult<uint> {
    if self.eof {
      return Err(io::standard_error(io::EndOfFile));
    }
    if !self.chunked {
      if self.length.is_none() {
        warn!("No content length header set!");
        return self.sock.read(buf);
      }
      if self.length_left == 0 {
        self.eof = true;
        return Err(io::standard_error(io::EndOfFile));
      } else if self.length_left > 0 {
        match self.sock.read(buf) {
          Ok(n)  => {
            self.length_left -= n;
            return Ok(n);
          }
          Err(e) => {
            warn!("error while reading {} bytes: {:}", self.length_left, e);
            return Err(io::standard_error(io::InvalidInput));
          }
        }
      } else {
        unreachable!()
      }
    }

    // read one chunk or less
    match self.chunked_left {
      Some(left) => {
        let tbuf_len = min(buf.len(), left);
        let mut tbuf = Vec::from_elem(tbuf_len, 0u8);
        match self.sock.read(tbuf.as_mut_slice()) {
          Ok(nread) => {
            buf.move_from(tbuf.as_slice().into_owned(), 0, nread);
            if left == nread {
              // this chunk ends
              // toss the CRLF at the end of the chunk
              assert!(self.sock.read_exact(2).is_ok());
              self.chunked_left = None;
            } else {
              self.chunked_left = Some(left - nread);
            }
            Ok(nread)
          }
          Err(e) => {
            error!("error read from sock: {}", e);
            Err(e)
          }
        }
      }
      None => {
        let chunked_left = self.read_next_chunk_size();
        match chunked_left {
          Some(0) => {
            assert!(self.sock.read_exact(2).is_ok());
            self.eof = true;
            Err(io::standard_error(io::EndOfFile))
          }
          Some(_) => {
            self.chunked_left = chunked_left;
            self.read(buf) // recursive call once, istead of Ok(0)
          }
          None => {
            self.eof = true;
            Err(io::standard_error(io::EndOfFile))
          }
        }
      }
    }
  }
}
