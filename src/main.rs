extern crate libmpv;

use libmpv::{events::{Event, PropertyData}, FileState, Format};
use std::{collections::VecDeque, io::{ErrorKind, Read, Write}, net::TcpStream};

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Config {
	host_address: String,
	port: u16,
	media_path: String
}




fn send(stream: &mut TcpStream, code: u8, data: &[u8]) {
	let size = (data.len() + 1) as u16;
	stream.write_all(&[&[size as u8, (size >> 8) as u8, code], data].concat()).unwrap()
}

fn main() {
	let Config { host_address, port, media_path } = confy::load_path(std::env::current_exe().unwrap().with_extension("toml")).unwrap();
	
	println!("Connecting to server at {} on port {}...", host_address, port);
	
	let address = dns_lookup::lookup_host(&host_address).unwrap()[0];
	let mut stream = TcpStream::connect((address, port)).unwrap();
	//stream.set_nodelay(true).unwrap();
	stream.set_nonblocking(true).unwrap();
	
	println!("Connected!");
	
	let mpv = libmpv::Mpv::new().unwrap();
	
	mpv.set_property("input-default-bindings", "yes").unwrap();
	mpv.set_property("keep-open", "yes").unwrap();
	mpv.set_property("force-window", "yes").unwrap();
	mpv.set_property("osc", "yes").unwrap();
	
	let mut events = mpv.create_event_context();
	events.observe_property("playback-time", Format::Double, 0).unwrap();
	events.observe_property("path", Format::String, 0).unwrap();
	events.observe_property("seeking", Format::Flag, 0).unwrap();
	events.observe_property("pause", Format::Flag, 0).unwrap();
	
	
	let event_cooldown = 0.5;
	let mut last_event = std::time::Instant::now();
	
	let mut seeking = false;
	let mut update_pos = false;
	
	let mut paused = true;
	let mut timestamp = 0.0;
	
	let mut buffer = VecDeque::new();
	
	loop {
		let mut buf = vec![];
		if let Err(e) = stream.read_to_end(&mut buf) {
			match e.kind() {
				ErrorKind::WouldBlock => (),
				ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted => panic!("Server connection lost"),
				kind => unimplemented!("Error kind: {kind}")
			}
		}
		
		if buf.len() > 0 {
			buffer.append(&mut VecDeque::from(buf));
			if buffer.len() >= 2 {
				let data_size = (buffer[0]) as usize + ((buffer[1] as usize) << 8);
				if buffer.len() >= data_size + 2 {
					buffer.drain(..2);
					let data = buffer.drain(..data_size).collect::<Vec<_>>();
					match *data.get(0).unwrap_or(&0) as char {
						'p' => {
							mpv.set_property("pause", match data[1] { 0 => false, _ => true }).unwrap_or_else(|e| println!("{}", e));
							mpv.set_property("playback-time", f64::from_le_bytes([data[2], data[3], data[4], data[5], data[6], data[7], data[8], data[9]])).unwrap_or_else(|e| println!("{}", e));
							last_event = std::time::Instant::now();
						}
						'f' => mpv.playlist_load_files(&[(&(media_path.clone() + &String::from_utf8(data.split_at(1).1.to_vec()).unwrap()), FileState::Replace, None)]).unwrap_or_else(|e| println!("{}", e)),
						_ => ()
					}
				}
			}
		}
		
		
		if let Some(event) = events.wait_event(0.0) {
			match event.unwrap() {
				Event::PropertyChange { name: "pause", change: PropertyData::Flag(p), .. } => {
					paused = p;
					if last_event.elapsed().as_secs_f64() > event_cooldown {
						send(&mut stream, 'p' as u8, &[vec![match paused { true => 1, false => 0 }], f64::to_le_bytes(timestamp).to_vec()].concat());
					}
				}
				Event::PropertyChange { name: "seeking", change: PropertyData::Flag(s), .. } => {
					seeking = s;
					if !s {
						update_pos = true;
					}
				}
				Event::PropertyChange { name: "path", change: PropertyData::Str(p), .. } => {
					if let Some(path) = p.replace("\\", "/").strip_prefix(&media_path) {
						send(&mut stream, 'f' as u8, &path.as_bytes());
					}
				}
				Event::PropertyChange { name: "playback-time", change: PropertyData::Double(t), .. } => {
					timestamp = t;
					if seeking || update_pos {
						if last_event.elapsed().as_secs_f64() > event_cooldown {
							send(&mut stream, 'p' as u8, &[vec![match paused { true => 1, false => 0 }], f64::to_le_bytes(timestamp).to_vec()].concat());
						}
						update_pos = false;
					}
				}
				Event::Shutdown => {
					stream.write_all(&[4, 0, 'e' as u8, 'x' as u8, 'i' as u8, 't' as u8]).unwrap();
					break;
				}
				_ => {}
			}
		}
	}
}
