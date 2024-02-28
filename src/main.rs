use libmpv::{events::{Event, PropertyData}, FileState, Format};
use std::{io::{Read, Write}, net::TcpStream};

extern crate libmpv;

fn send(stream: &mut TcpStream, code: u8, data: &[u8]) {
	let size = (data.len() + 1) as u16;
	stream.write_all(&[&[size as u8, (size >> 8) as u8, code], data].concat()).unwrap()
}

fn main() {
	let hostname = "localhost"; // "jorbonvm.centralus.cloudapp.azure.com";
	let address = dns_lookup::lookup_host(hostname).unwrap();
	let mut stream = TcpStream::connect((address[1], 7777u16)).unwrap();
	//stream.set_nodelay(true).unwrap();
	stream.set_nonblocking(true).unwrap();
	
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
	
	let mut seeking = false;
	let mut update_pos = false;
	
	let mut buf = [0u8; 256];
	
	loop {
		if let Ok(n) = stream.read(&mut buf) {
			if n > 0 {
				match buf[0] as char {
					'p' => mpv.set_property("pause", match buf[1] { 0 => false, _ => true }).unwrap_or(()),
					't' => mpv.set_property("playback-time", f64::from_le_bytes([buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8]])).unwrap_or(()),
					'f' => mpv.playlist_load_files(&[(&String::from_utf8(buf.split_at(1).1.to_vec()).unwrap(), FileState::Replace, None)]).unwrap_or(()),
					_ => ()
				}
			}
		}
		
		
		if let Some(event) = events.wait_event(0.0) {
			match event.unwrap() {
				Event::PropertyChange { name: "pause", change: PropertyData::Flag(paused), .. } => send(&mut stream, 'p' as u8, &[match paused { true => 1, false => 0 }]),
				Event::PropertyChange { name: "seeking", change: PropertyData::Flag(s), .. } => {
					seeking = s;
					if !s {
						update_pos = true;
					}
				}
				Event::PropertyChange { name: "path", change: PropertyData::Str(path), .. } => send(&mut stream, 'f' as u8, &path.as_bytes()),
				Event::PropertyChange { name: "playback-time", change: PropertyData::Double(time), .. } => {
					if seeking || update_pos {
						send(&mut stream, 't' as u8, &f64::to_le_bytes(time));
						update_pos = false;
					}
				}
				Event::Shutdown => {
					stream.write_all(&[0, 4, 'e' as u8, 'x' as u8, 'i' as u8, 't' as u8]).unwrap();
					break;
				}
				_ => {}
			}
		}
	}
}
