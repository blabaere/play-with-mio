// things to be checked after that:
// https://github.com/dpc/mioco
// https://github.com/dwrensha/gj
// https://github.com/calc0000/tcp-loop

use mio::*;
use mio::tcp::*;
use mio::util::Slab;
use std::io;
use std::io::{Error};
use std::net::{SocketAddr};
use std::str;

const SERVER: Token = Token(0);

struct SimpleClient {
    stream: NonBlock<TcpStream>,
    token: Option<Token>,
    rx_buffer: [u8; 2048],
    tx_buffer: Vec<u8>
}

impl SimpleClient {
	fn new(stream: NonBlock<TcpStream>) -> SimpleClient {
		SimpleClient {
			stream: stream,
			token: None,
			rx_buffer: [0u8; 2048],
			tx_buffer: Vec::new()
		}
	}

	fn pull_msg(&mut self) -> Result<Option<Vec<u8>>, io::Error> {
        let mut result = Vec::with_capacity(2048);
        //let mut buffer = &mut self.rx_buffer;
        //let stream = self.get_stream();

        while let Some(count) = try!(self.read_slice()) {
        	if count == 0 {
        		return Ok(None)
        	} else {
        		result.extend(self.rx_buffer[..count].iter().map(|x| *x));
        	}
        }

        Ok(Some(result))
	}

	fn read_slice(&mut self) -> Result<Option<usize>, Error> {
		let stream: &mut NonBlock<::mio::tcp::TcpStream> = &mut self.stream;
		stream.read_slice(&mut self.rx_buffer)
	}

	fn push_msg(&mut self, event_loop: &mut EventLoop<SimpleServer>, buffer: &Vec<u8>) {
		let interest = Interest::writable() | Interest::hup();
		let poll_opt = PollOpt::edge();

		self.tx_buffer.reserve(buffer.len());
		self.tx_buffer.extend(buffer.iter().map(|x| *x));

		event_loop.reregister(&self.stream, self.token.unwrap(), interest, poll_opt).unwrap();
	}

	fn flush_msg(&mut self, event_loop: &mut EventLoop<SimpleServer>) -> Result<Option<usize>, io::Error> {

		match try!(self.stream.write_slice(&self.tx_buffer[..])) {
			Some(0) => Ok(Some(0)),
			Some(count) => {
				self.tx_buffer = self.tx_buffer[count..].to_vec();

				if self.tx_buffer.len() == 0 {
					let interest = Interest::readable() | Interest::hup();
					let poll_opt = PollOpt::edge();
					event_loop.reregister(&self.stream, self.token.unwrap(), interest, poll_opt).unwrap();
				}
				Ok(Some(count))
			},
			None => Ok(None)
		}
	}
}

struct SimpleServer {
	listener: NonBlock<TcpListener>,
	clients: Slab<SimpleClient>
}

impl SimpleServer {
	fn accept(&mut self, event_loop: &mut EventLoop<Self>, _: ReadHint) {
		let client_stream = self.listener.accept().unwrap().unwrap();
		let client = SimpleClient::new(client_stream);
		let client_token = self.clients.insert(client).ok().unwrap();
		let client_interest = Interest::readable() | Interest::hup();
		let client_poll_opt = PollOpt::edge()/* | PollOpt::oneshot()*/;

		self.get_client(client_token).token = Some(client_token);

		info!("SimpleServer::accept client {:?}", client_token.as_usize());

		event_loop.register_opt(&self.clients[client_token].stream, client_token, client_interest, client_poll_opt).unwrap();
	}

	fn readable(&mut self, event_loop: &mut EventLoop<Self>, client_token: Token, hint: ReadHint) {
		if hint.is_hup() {
			info!("Client {:?} have left", client_token.as_usize());
			self.clients.remove(client_token);
			return;
		}

		match self.get_client(client_token).pull_msg() {
			Err(e) => {error!("Failed to read from client {:?}: {}", client_token.as_usize(), e)},
			Ok(None) => {warn!("Read from client {:?} would have blocked", client_token.as_usize())},
			Ok(Some(buffer)) => if buffer.len() > 0 {
				info!("Read {} bytes from client {:?}", buffer.len(), client_token.as_usize());
				self.push_msg_to_clients(event_loop, &buffer);
			}
		}
	}

	fn push_msg_to_clients(&mut self, event_loop: &mut EventLoop<Self>, buffer: &Vec<u8>) {
		for client in self.clients.iter_mut() {
			client.push_msg(event_loop, buffer);
		}
	}

	fn writable(&mut self, event_loop: &mut EventLoop<Self>, client_token: Token) {
		match self.get_client(client_token).flush_msg(event_loop) {
			Err(e) => warn!("Failed to send msg to client {:?}: {}", client_token.as_usize(), e),
			Ok(None) => {warn!("Write to client {:?} would have blocked", client_token.as_usize())},
			Ok(Some(0)) => {warn!("Write to client {:?} 0 bytes ?!", client_token.as_usize())},
			_ => {}
		}
	}

    fn get_client<'a>(&'a mut self, token: Token) -> &'a mut SimpleClient {
        &mut self.clients[token]
    }
}

impl Handler for SimpleServer {
    type Timeout = usize;
    type Message = ();

    fn readable(&mut self, event_loop: &mut EventLoop<Self>, token: Token, hint: ReadHint) {
    	info!("Handler::readable {:?} {:?}", token, hint);

        match token {
            Token(0) => self.accept(event_loop, hint),
            i => self.readable(event_loop, i, hint)
        };
    }

    fn writable(&mut self, event_loop: &mut EventLoop<Self>, token: Token) {
    	info!("Handler::writable {:?}", token);

        match token {
            Token(0) => error!("server received writable notification !"),
            i => self.writable(event_loop, i)
        };
   }
}

pub fn run() {
	info!("Server::run");

	let mut event_loop = EventLoop::new().unwrap();
	let addr: SocketAddr = str::FromStr::from_str("127.0.0.1:5454").unwrap();
	let listener = listen(&addr).unwrap();
    let interest = Interest::readable();

    event_loop.register_opt(&listener, SERVER, interest, PollOpt::edge()).unwrap();

    let clients = Slab::new_starting_at(Token(1), 128);
    let mut server = SimpleServer { listener: listener, clients: clients };
    event_loop.run(&mut server).unwrap();
}
