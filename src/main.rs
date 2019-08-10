extern crate gstreamer as gst;
extern crate gstreamer_app as gst_app;

use gst::prelude::*;
use gst_app::*;

use itertools::Itertools;

use std::convert::TryInto;
use std::env;
use std::process::exit;

fn main() {

	/*
	 * Initialization
	 */

	// Input video file is the first argument to the program
	let args: Vec<String> = env::args().collect();
	if args.len() < 2 {
		println!("{}: Please provide a video as argument", args[0]);
		exit(1);
	}
	let inputfile = &args[1];

	gst::init().unwrap();
	//let main_loop = glib::MainLoop::new(None, false);
	let pipeline = gst::Pipeline::new(None);

	let width: usize = 1080 / 10;
	let height: usize = 720 / 20;


	/*
	 * Pipeline Elements
	 */

	let source = gst::ElementFactory::make("filesrc", Some("src")).unwrap();
	source.set_property("location", inputfile).unwrap();
	let decode = gst::ElementFactory::make("decodebin", Some("decodebin")).unwrap();
	let convert = gst::ElementFactory::make("videoconvert", Some("vconvert")).unwrap();
	let scale = gst::ElementFactory::make("videoscale", Some("vscale")).unwrap();
	let sink = gst::ElementFactory::make("appsink", Some("appsink")).unwrap();
	let caps = gst::Caps::builder("video/x-raw")
		.field("format", &"RGB")
		.field("width", &(width as i32))
		.field("height", &(height as i32))
		.build();
	sink.set_property("caps", &caps).unwrap();

	/*
	 * Build the Pipeline
	 */

	let elements = &[&source, &decode, &convert, &scale, &sink];
	pipeline.add_many(elements).unwrap();

	gst::Element::link_many(&[&source, &decode]).unwrap();
	gst::Element::link_many(&[&convert, &scale, &sink]).unwrap();

	for e in elements {
		e.sync_state_with_parent().unwrap();
	}

	// pipeline_weak is moved into the following closure and cannot be reused
	let pipeline_weak = pipeline.downgrade();

	/* Wait until decodebin presents a source pad, and then link it to the
	 * sink pad of videoconvert. */
	decode.connect_pad_added(move |_, src_pad| {
		let pipeline = match pipeline_weak.upgrade() {
            Some(pipeline) => pipeline,
            None => return,
        };

        let convert = pipeline.get_by_name("vconvert").unwrap();
        let convert_sink_pad = convert.get_static_pad("sink").unwrap();

        // Only link the pads once
		if ! ( src_pad.is_linked() || convert_sink_pad.is_linked() ) {
			src_pad.link(&convert_sink_pad).unwrap();

			// Resync the pipeline elements
			for e in pipeline.get_children() {
				e.sync_state_with_parent().unwrap();
			}
		}
	});

	// Set the pipeline state to paused to stay on a single frame
	pipeline.set_state(gst::State::Paused).expect("Unable to pause pipeline");

	/*
	 * Error and State Change Message Callbacks
	 */

	let bus = pipeline.get_bus().expect("Failed to get pipeline bus");
	//let main_loop_clone = main_loop.clone();
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
				//main_loop_clone.quit();
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

	/*
	 * Fetch a framebuffer from the AppSink
	 */

	let appsink = sink
		.dynamic_cast::<AppSink>()
		.expect("Sink element is expected to be an appsink!");

	let sample = appsink.pull_preroll().expect("Failed to pull appsink preroll");
	let buffer = sample.get_buffer().expect("Failed to get buffer");
	let map = buffer.map_readable().unwrap();

	// Convert the mapped buffer to a slice of u8
	let samples: &[u8] = map.as_slice();

	for row in samples.chunks(3 * width) {
		for pixel in row.chunks(3) {
			print!("\x1b[38;2;{};{};{}m█\x1b[0m", pixel[0], pixel[1], pixel[2]);
		}
		print!("\n");
	}

	//main_loop.run();
}
