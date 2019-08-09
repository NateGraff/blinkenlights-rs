extern crate gstreamer as gst;
extern crate gstreamer_app as gst_app;

use gst::prelude::*;
use gst_app::*;

use std::convert::TryInto;
use std::env;
use std::process::exit;

fn main() {
	// Input video file is the first argument to the program
	let args: Vec<String> = env::args().collect();
	if args.len() < 2 {
		println!("{}: Please provide a video as argument", args[0]);
		exit(1);
	}
	let inputfile = &args[1];

	gst::init().unwrap();

	let main_loop = glib::MainLoop::new(None, false);

	let pipeline = gst::parse_launch(
		&format!("uridecodebin uri={} ! videoconvert ! videoscale ! appsink", inputfile)).unwrap();
	
	// Set the pipeline state to paused to stay on a single frame
	pipeline.set_state(gst::State::Paused).expect("Unable to pause pipeline");

	let bus = pipeline.get_bus().expect("Failed to get pipeline bus");
	let main_loop_clone = main_loop.clone();
	bus.add_watch(move |_, msg| {
		match msg.view() {
			gst::MessageView::Error(err) => {
				println!(
					"Error from {:?}: {} ({:?})",
					err.get_src().map(|s| s.get_path_string()),
					err.get_error(),
					err.get_debug()
				);
				pipeline.set_state(gst::State::Null)
					.expect("Unable to stop pipeline");
				main_loop_clone.quit();
			}
			gst::MessageView::StateChanged(state_changed) => {
				let current_state = state_changed.get_current();
				println!(
					"Pipeline state changed from {:?} to {:?}",
					state_changed.get_old(),
					current_state
				);

				if current_state == gst::State::Paused {
				let dur: gst::ClockTime = {
					let mut q = gst::Query::new_duration(gst::Format::Time);
					if pipeline.query(&mut q) {
						Some(q.get_result())
					} else {
						None
					}
				}
				.and_then(|dur| dur.try_into().ok())
				.or(Some(gst::ClockTime::from_seconds(0)))
				.unwrap();
				println!("Duration {}", dur);
				}
			}
			_ => (),
		};

		gst::Continue(true)
	});

	main_loop.run();

	/*let dur: gst::ClockTime = {
		let mut q = gst::Query::new_duration(gst::Format::Time);
		if pipeline.query(&mut q) {
			Some(q.get_result())
		} else {
			None
		}
	}
	.and_then(|dur| dur.try_into().ok())
	.or(Some(gst::ClockTime::from_seconds(0)))
	.unwrap();
	println!("Duration of {}: {}", inputfile, dur);*/
}
