use libmpv::{events::{Event, PropertyData}, Format};

extern crate libmpv;

fn main() {
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
	
	loop {
		if let Some(event) = events.wait_event(60.0) {
			match event.unwrap() {
				Event::PropertyChange { name: "pause", change: PropertyData::Flag(paused), .. } => println!("paused: {paused}"),
				Event::PropertyChange { name: "seeking", change: PropertyData::Flag(s), .. } => {
					seeking = s;
					if !s {
						update_pos = true;
					}
				}
				Event::PropertyChange { name: "path", change: PropertyData::Str(path), .. } => println!("{path}"),
				Event::PropertyChange { name: "playback-time", change: PropertyData::Double(time), .. } => {
					if seeking || update_pos {
						println!("Time: {time}s");
						update_pos = false;
					}
				}
				Event::Shutdown => break,
				_ => {}
			}
		}
	}
}