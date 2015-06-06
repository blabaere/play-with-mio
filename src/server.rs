
use mio::*;
use mio::tcp::*;
//use mio::buf::{ByteBuf, MutByteBuf, SliceBuf};
use mio::util::Slab;
//use std::io;
use std::io::{Read, Write};
use std::net::{SocketAddr};
use std::str;

const SERVER: Token = Token(0);

struct SimpleClient {
    stream: TcpStream,
    token: Option<Token>
}

impl SimpleClient {
	fn new(stream: TcpStream) -> SimpleClient {
		SimpleClient {
			stream: stream,
			token: None
		}
	}

	fn readable(&mut self, event_loop: &mut EventLoop<SimpleServer>) {
		info!("SimpleClient::readable {:?}", self.token.unwrap().as_usize());

		let mut buffer = [0u8; 2048]; 
		match self.stream.read(&mut buffer) {
			Ok(len) => {
				info!("SimpleClient {:?} read {} bytes !", self.token.unwrap().as_usize(), len);
				self.on_msg(&buffer[..len]);
			},
			Err(e)  => info!("SimpleClient {:?} failed to read err={:?}", self.token.unwrap().as_usize(), e)
		}

		let interest = Interest::writable();
		let poll_opt = PollOpt::edge();
		event_loop.reregister(&self.stream, self.token.unwrap(), interest, poll_opt).unwrap();
	}

	fn writable(&mut self, _: &mut EventLoop<SimpleServer>) -> bool {
		info!("SimpleClient::writable {:?}", self.token.unwrap().as_usize());

		match self.stream.write_all(b"baudet\r\n") {
			Ok(()) => info!("Reply sent"),
			Err(e) => error!("Failed to reply: {} !", e)
		}

		true
	}

	fn on_msg(&mut self, bytes: &[u8]) {
		match str::from_utf8(bytes) {
            Ok(text) => info!("SimpleClient::on_msg msg={:?}", text),
            Err(e) => info!("SimpleClient::on_msg err={:?}", e)
        }
	}
}

struct SimpleServer {
	listener: TcpListener,
	clients: Slab<SimpleClient>
}

impl SimpleServer {
	fn accept(&mut self, event_loop: &mut EventLoop<Self>) {
		let (client_stream, _) = self.listener.accept().unwrap();
		let client = SimpleClient::new(client_stream);
		let client_token = self.clients.insert(client).ok().unwrap();
		let client_interest = Interest::readable();
		let client_poll_opt = PollOpt::edge();

		self.get_client(client_token).token = Some(client_token);

		info!("SimpleServer::accept client {:?}", client_token.as_usize());

		event_loop.register_opt(&self.clients[client_token].stream, client_token, client_interest, client_poll_opt).unwrap();
	}

	fn readable(&mut self, event_loop: &mut EventLoop<Self>, client_token: Token) {
		info!("SimpleServer::readable");
		self.get_client(client_token).readable(event_loop)
	}

	fn writable(&mut self, event_loop: &mut EventLoop<Self>, client_token: Token) {
		info!("SimpleServer::writable");
		if self.get_client(client_token).writable(event_loop) {
			let client = self.clients.remove(client_token).unwrap();
			event_loop.deregister(&client.stream).unwrap();
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
            Token(0) => self.accept(event_loop),
            i => self.readable(event_loop, i)
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
    let socket = TcpListener::bind(&addr).unwrap();
    let interest = Interest::readable();

    event_loop.register_opt(&socket, SERVER, interest, PollOpt::edge()).unwrap();

    let clients = Slab::new_starting_at(Token(1), 128);
    let mut server = SimpleServer { listener: socket, clients: clients };
    event_loop.run(&mut server).unwrap();
}