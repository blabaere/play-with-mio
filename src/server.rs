
use mio::*;
use mio::tcp::*;
use mio::buf::{ByteBuf, MutByteBuf};
use mio::util::Slab;
use std::io::{Read, Write};
use std::net::{SocketAddr};
use std::str;

const SERVER: Token = Token(0);

struct SimpleClient {
    stream: NonBlock<TcpStream>,
    token: Option<Token>,
    rx_buf: MutByteBuf
}

impl SimpleClient {
	fn new(stream: NonBlock<TcpStream>, rx_buf: MutByteBuf) -> SimpleClient {
		SimpleClient {
			stream: stream,
			token: None,
			rx_buf: rx_buf 
		}
	}

	fn readable(&mut self, _: &mut EventLoop<SimpleServer>) -> bool {
		info!("SimpleClient::readable {:?}", self.token.unwrap().as_usize());

		match self.stream.read(&mut self.rx_buf) {
			Err(e) => {error!("Failed to read: {} !", e); false},
			Ok(None) => {info!("Read nothing, would block ?"); false},
			Ok(Some(0)) => {info!("Read 0 bytes, end of stream ?"); false},
			Ok(Some(count)) => {info!("Read {} bytes", count); true} // could there be some more to read ?
		}
		// now we need to create a message for the other clients and have the server send it

		/*let mut buffer = [0u8; 2048]; 
		match self.stream.read(&mut buffer) {
			Ok(len) => {
				info!("SimpleClient {:?} read {} bytes !", self.token.unwrap().as_usize(), len);
				self.on_msg(&buffer[..len]);
			},
			Err(e)  => info!("SimpleClient {:?} failed to read err={:?}", self.token.unwrap().as_usize(), e)
		}

		let interest = Interest::writable();
		let poll_opt = PollOpt::edge();
		event_loop.reregister(&self.stream, self.token.unwrap(), interest, poll_opt).unwrap();*/
	}

	fn switch_to_write_state(&mut self, event_loop: &mut EventLoop<SimpleServer>) {
		let interest = Interest::writable();
		let poll_opt = PollOpt::edge();
		event_loop.reregister(&self.stream, self.token.unwrap(), interest, poll_opt).unwrap();
	}

	fn writable(&mut self, _: &mut EventLoop<SimpleServer>) -> bool {
		info!("SimpleClient::writable {:?}", self.token.unwrap().as_usize());

		match self.stream.write_all(b"baudet\r\n") {
			Ok(()) => {info!("Reply sent"); true},
			Err(e) => {error!("Failed to reply: {} !", e); false}
		}

	}

	fn switch_to_read_state(&mut self, event_loop: &mut EventLoop<SimpleServer>) {
		let interest = Interest::readable();
		let poll_opt = PollOpt::edge();
		event_loop.reregister(&self.stream, self.token.unwrap(), interest, poll_opt).unwrap();
	}

	/*fn on_msg(&mut self, bytes: &[u8]) {
		match str::from_utf8(bytes) {
            Ok(text) => info!("SimpleClient::on_msg msg={:?}", text),
            Err(e) => info!("SimpleClient::on_msg err={:?}", e)
        }
	}*/
}

struct SimpleServer {
	listener: NonBlock<TcpListener>,
	clients: Slab<SimpleClient>,
	pending_reply_count: u8
}

impl SimpleServer {
	fn accept(&mut self, event_loop: &mut EventLoop<Self>) {
		let client_stream = self.listener.accept().unwrap().unwrap();
		let client = SimpleClient::new(client_stream, ByteBuf::mut_with_capacity(2048));
		let client_token = self.clients.insert(client).ok().unwrap();
		let client_interest = Interest::readable();
		let client_poll_opt = PollOpt::edge() | PollOpt::oneshot();

		self.get_client(client_token).token = Some(client_token);

		info!("SimpleServer::accept client {:?}", client_token.as_usize());

		event_loop.register_opt(&self.clients[client_token].stream, client_token, client_interest, client_poll_opt).unwrap();
	}

	fn readable(&mut self, event_loop: &mut EventLoop<Self>, client_token: Token) {
		info!("SimpleServer::readable {:?}", client_token.as_usize());

		let read_res = self.get_client(client_token).readable(event_loop);

		if read_res {
			for client in self.clients.iter_mut() {
				info!("Tell client {:?} that someone has spoken ...", client.token);
				client.switch_to_write_state(event_loop);
				self.pending_reply_count += 1;
			}
		}
	}

	fn writable(&mut self, event_loop: &mut EventLoop<Self>, client_token: Token) {
		info!("SimpleServer::writable {:?}", client_token.as_usize());

		let write_res = self.get_client(client_token).writable(event_loop);

		if write_res {
			self.pending_reply_count -= 1;

			if self.pending_reply_count == 0 {
				info!("Everyone was sent the spoken truth ...");

				for client in self.clients.iter_mut() {
					client.switch_to_read_state(event_loop);
				}
			}
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
	let listener = listen(&addr).unwrap();
    //let socket = TcpListener::bind(&addr).unwrap();
    let interest = Interest::readable();

    event_loop.register_opt(&listener, SERVER, interest, PollOpt::edge()).unwrap();

    let clients = Slab::new_starting_at(Token(1), 128);
    let mut server = SimpleServer { listener: listener, clients: clients, pending_reply_count: 0 };
    event_loop.run(&mut server).unwrap();
}