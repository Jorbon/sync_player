use std::{collections::VecDeque, io::{ErrorKind, Read, Write}, net::{SocketAddr, TcpListener, TcpStream}};


#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Config {
	pub port: u16
}

#[derive(Debug)]
enum Connection {
	Opened { address: SocketAddr, stream: TcpStream, buffer: VecDeque<u8> },
	Active { address: SocketAddr, stream: TcpStream, buffer: VecDeque<u8> },
	Closed { address: SocketAddr }
}

impl Connection {
	fn close(&mut self) {
		match self {
			Connection::Opened { address, stream, buffer: _ } | Connection::Active { address, stream, buffer: _ } => {
				stream.shutdown(std::net::Shutdown::Both).unwrap();
				*self = Connection::Closed { address: *address };
			}
			Connection::Closed { address: _ } => ()
		}
	}
	fn activate(&mut self) {
		if let Connection::Opened { address, stream, buffer } = self {
			*self = Connection::Active { address: *address, stream: stream.try_clone().unwrap(), buffer: buffer.clone() };
		}
	}
}

fn format_message(s: &str) -> Vec<u8> {
	let data = s.as_bytes();
	let data_size = data.len() as u16;
	[&data_size.to_le_bytes(), data].concat()
}


fn main() {
	let Config { port } = confy::load_path(std::env::current_exe().unwrap().with_extension("toml")).unwrap();
	
	let listener = TcpListener::bind(format!("0.0.0.0:{port}")).unwrap();
	listener.set_nonblocking(true).unwrap();
	
	let mut file = String::new();
	let mut time = 0.0;
	let mut paused = true;
	
	println!("Started sync player server on port {port}");
	
	let mut connections = vec![];
	
	loop {
		while let Ok((stream, address)) = listener.accept() {
			connections.push(Connection::Opened { address, stream: stream, buffer: VecDeque::new() });
			println!("Connected to {address}, {} connections open", connections.len());
		}
		
		for i in 0..connections.len() {
			match &mut connections[i] {
				Connection::Opened { address: _, stream, buffer } => {
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
					if buffer.len() < 2 { continue }
					
					let data_size = (buffer[0]) as usize + ((buffer[1] as usize) << 8);
					if buffer.len() < data_size + 2 { continue }
					
					let data = buffer.drain(..(data_size + 2)).collect::<Vec<_>>();
					
					if data == format_message("exit") {
						connections[i].close();
						continue
					} else if data == format_message("jhello") {
						if let Err(e) = stream.write_all(&format_message("howdy!")) {
							match e.kind() {
								ErrorKind::WouldBlock => panic!("blocked write"),
								ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted => {
									connections[i].close();
									continue
								}
								kind => unimplemented!("Error type: {kind}")
							}
						}
						connections[i].activate();
						continue
					}
					
					
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
				Connection::Active { address: _, stream, buffer } => {
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
				Connection::Closed { address: _ } => ()
			}
		}
		
		let mut i = 0;
		while i < connections.len() {
			match connections[i] {
				Connection::Opened { .. } | Connection::Active { .. } => i += 1,
				Connection::Closed { address } => {
					connections.swap_remove(i);
					println!("Disconnected from {address}, {} connections open", connections.len());
				}
			}
		}
	}
}