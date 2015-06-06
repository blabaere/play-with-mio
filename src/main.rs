extern crate mio;
#[macro_use] extern crate log;
extern crate env_logger;

mod server;

fn usage() {
    println!("Usage: play-with-mio [client|server]");
}

fn client() {
    println!("Hello, world!");
}

fn main() {
    let args: Vec<_> = std::env::args().collect();

    if args.len() < 2 {
        return usage()
    }

    env_logger::init().unwrap();
    info!("Logging initialized.");

    match args[1].as_ref() {
        "client" => client(),
        "server" => server::run(),
        _ => usage()
    }
}
