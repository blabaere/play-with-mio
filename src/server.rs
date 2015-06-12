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
    token: Option<Token>
}

impl SimpleClient {
	fn new(stream: NonBlock<TcpStream>) -> SimpleClient {
		SimpleClient {
			stream: stream,
			token: None
		}
	}

	fn try_read_all(&mut self) -> Result<Option<Vec<u8>>, io::Error> {
        let mut result = Vec::with_capacity(2048);
        let mut buffer = &mut [0u8; 2048];

        let stream = &mut self.stream as &mut NonBlock<::mio::tcp::TcpStream>;

        while let Some(count) = try!(stream.read_slice(buffer)) {
            result.extend(buffer[..count].iter().map(|x| *x));
        }

        Ok(Some(result))
	}

	/*fn switch_to_write_state(&mut self, event_loop: &mut EventLoop<SimpleServer>) {
		let interest = Interest::writable();
		let poll_opt = PollOpt::edge();
		event_loop.reregister(&self.stream, self.token.unwrap(), interest, poll_opt).unwrap();
	}*/

	/*fn switch_to_read_state(&mut self, event_loop: &mut EventLoop<SimpleServer>) {
		let interest = Interest::readable();
		let poll_opt = PollOpt::edge();
		event_loop.reregister(&self.stream, self.token.unwrap(), interest, poll_opt).unwrap();
	}*/

	/*fn on_msg(&mut self, bytes: &[u8]) {
		match str::from_utf8(bytes) {
            Ok(text) => info!("SimpleClient::on_msg msg={:?}", text),
            Err(e) => info!("SimpleClient::on_msg err={:?}", e)
        }
	}*/
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
		let client_interest = Interest::readable();
		let client_poll_opt = PollOpt::edge()/* | PollOpt::oneshot()*/;

		self.get_client(client_token).token = Some(client_token);

		info!("SimpleServer::accept client {:?}", client_token.as_usize());

		event_loop.register_opt(&self.clients[client_token].stream, client_token, client_interest, client_poll_opt).unwrap();
	}

	fn readable(&mut self, _: &mut EventLoop<Self>, client_token: Token, _: ReadHint) {
		info!("SimpleServer::readable {:?}", client_token.as_usize());

		match self.try_read_all_from_client(client_token) {
			Err(e) => {error!("Failed to read from client {:?}: {}", client_token.as_usize(), e)},
			Ok(None) => {warn!("Read from client {:?} would have blocked", client_token.as_usize())}
			Ok(Some(buffer)) => if buffer.len() > 0 {
				info!("Read {} bytes from client {:?}", buffer.len(), client_token.as_usize())
			}
		}
	}

	fn try_read_all_from_client(&mut self, client_token: Token) -> Result<Option<Vec<u8>>, io::Error> {
		self.get_client(client_token).try_read_all()
	}

	fn writable(&mut self, _: &mut EventLoop<Self>, client_token: Token) {
		info!("SimpleServer::writable {:?}", client_token.as_usize());

		//let write_res = self.get_client(client_token).writable(event_loop);

		/*if write_res {

			if true {
				info!("Everyone was sent the spoken truth ...");

				for client in self.clients.iter_mut() {
					client.switch_to_read_state(event_loop);
				}
			}
		}*/
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