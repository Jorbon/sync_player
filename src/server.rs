use std::{collections::VecDeque, io::{ErrorKind, Read, Write}, net::{SocketAddr, TcpListener, TcpStream}};


#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Config {
	pub port: u16
}

#[derive(Debug)]
enum Connection {
	Active { address: SocketAddr, stream: TcpStream, buffer: VecDeque<u8> },
	Closed { address: SocketAddr }
}

impl Connection {
	fn close(&mut self) {
		if let Connection::Active { address, stream, buffer: _ } = self {
			stream.shutdown(std::net::Shutdown::Both).unwrap();
			*self = Connection::Closed { address: *address };
		}
	}
}

fn main() {
	let Config { port } = confy::load_path(std::env::current_exe().unwrap().with_extension("toml")).unwrap();
	
	let listener = TcpListener::bind(format!("0.0.0.0:{port}")).unwrap();
	listener.set_nonblocking(true).unwrap();
	
	println!("Started sync player server on port {port}");
	
	let mut connections = vec![];
	
	loop {
		while let Ok((stream, address)) = listener.accept() {
			connections.push(Connection::Active { address, stream, buffer: VecDeque::new() });
			println!("Connected to {address}, {} connections open", connections.len());
		}
		
		for i in 0..connections.len() {
			if let Connection::Active { address: _, stream, buffer } = &mut connections[i] {
				
				let mut buf = vec![];
				if let Err(e) = stream.read_to_end(&mut buf) {
					match e.kind() {
						ErrorKind::WouldBlock => (),
						ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted => {
							connections[i].close();
							continue;
						}
						kind => unimplemented!("Error type: {kind}")
					}
				}
				
				if buf.len() == 0 { continue }
				
				buffer.append(&mut VecDeque::from(buf));
				
				if buffer.len() >= 2 {
					let data_size = (buffer[0]) as usize + ((buffer[1] as usize) << 8);
					if buffer.len() >= data_size + 2 {
						
						let data = buffer.drain(..(data_size + 2)).collect::<Vec<_>>();
						
						if data == [4, 0, 'e' as u8, 'x' as u8, 'i' as u8, 't' as u8] {
							connections[i].close();
							continue
						}
						
						//println!("{address} sent {} {} bytes", data_size, data.get(2).map(|n| *n as char).unwrap_or('z'));
						
						for j in 0..connections.len() {
							if i == j { continue }
							if let Connection::Active { stream, .. } = &mut connections[j] {
								if let Err(e) = stream.write_all(&data) {
									match e.kind() {
										ErrorKind::WouldBlock => panic!("blocked write"),
										ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted => connections[j].close(),
										kind => unimplemented!("Error type: {kind}")
									}
								}
							}
						}
					}
				}
			}
		}
		
		let mut i = 0;
		while i < connections.len() {
			match connections[i] {
				Connection::Active { .. } => i += 1,
				Connection::Closed { address } => {
					connections.swap_remove(i);
					println!("Disconnected from {address}, {} connections open", connections.len());
				}
			}
		}
	}
}