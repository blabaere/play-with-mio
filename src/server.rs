extern crate mio;

use mio::*;
use mio::tcp::*;
use mio::buf::{ByteBuf, MutByteBuf, SliceBuf};
use mio::util::Slab;
use std::io;
use std::io::Read;
use std::net::{Ipv4Addr, IpAddr, SocketAddr};
use std::str::FromStr;

const SERVER: Token = Token(0);

struct SimpleClient {
    stream: TcpStream
}

impl SimpleClient {
	fn readable(&mut self, event_loop: &mut EventLoop<SimpleServer>) {
		println!("SimpleClient::readable");

		let mut buffer = Vec::new();
		match self.stream.read_to_end(&mut buffer) {
            Ok(count) => {
                println!("CONN : we read {} bytes!", count);
                //self.interest.remove(Interest::readable());
                //self.interest.insert(Interest::writable());
            }
            Err(e) => {
                println!("not implemented; client err={:?}", e);
                //self.interest.remove(Interest::readable());
            }

        };
	}
}

struct SimpleServer {
	listener: TcpListener,
	clients: Slab<SimpleClient>
}

impl SimpleServer {
	fn accept(&mut self, event_loop: &mut EventLoop<Self>) {
		println!("SimpleServer::accept");

		let (client_stream, _) = self.listener.accept().unwrap();
		let client = SimpleClient { stream: client_stream };
		let client_token = self.clients.insert(client).ok().unwrap();

		event_loop.register_opt(&self.clients[client_token].stream, client_token, Interest::readable(), PollOpt::edge() | PollOpt::oneshot()).unwrap();
	}

	fn readable(&mut self, event_loop: &mut EventLoop<Self>, client_token: Token) {
		println!("SimpleServer::read_from_client");
		self.get_client(client_token).readable(event_loop)
	}

    fn get_client<'a>(&'a mut self, token: Token) -> &'a mut SimpleClient {
        &mut self.clients[token]
    }
}

impl Handler for SimpleServer {
    type Timeout = usize;
    type Message = ();

    fn readable(&mut self, event_loop: &mut EventLoop<Self>, token: Token, hint: ReadHint) {
    	println!("SimpleServer::readable {:?} {:?}", token, hint);

        match token {
            Token(0) => self.accept(event_loop),
            i => self.readable(event_loop, i)
        };
    }
}

pub fn run() {
	println!("Server::run");

	let mut event_loop = EventLoop::new().unwrap();
	/*let ipAddr = Ipv4Addr::new(127, 0, 0, 1);
	let addr = SocketAddr::new(IpAddr::V4(ipAddr), 5454);*/
	let addr: SocketAddr = FromStr::from_str("127.0.0.1:5454").unwrap();
    let socket = TcpListener::bind(&addr).unwrap();

    event_loop.register_opt(&socket, SERVER, Interest::readable(), PollOpt::edge() | PollOpt::oneshot()).unwrap();

    let clients = Slab::new_starting_at(Token(1), 128);
    let mut server = SimpleServer { listener: socket, clients: clients };
    event_loop.run(&mut server).unwrap();
}