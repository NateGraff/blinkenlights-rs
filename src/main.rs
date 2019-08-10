extern crate gstreamer as gst;
extern crate gstreamer_app as gst_app;
extern crate itertools;

use gst::prelude::*;
use gst_app::*;

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
	let main_loop = glib::MainLoop::new(None, false);
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

		let caps = src_pad.get_current_caps().expect("Failed to get decodebin caps");
		let structure = caps.get_structure(0).expect("Failed to get structure of decodebin caps");

		// Only link if the pad provides video
		if structure.get_name().starts_with("video/") {
			src_pad.link(&convert_sink_pad).unwrap();

			// Resync the pipeline elements after linking
			for e in pipeline.get_children() {
				e.sync_state_with_parent().unwrap();
			}
		}
	});

	// Set the pipeline state to paused to stay on a single frame
	pipeline.set_state(gst::State::Playing).expect("Unable to Play pipeline");

	/*
	 * Error Message Callback
	 */

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
			},
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

	appsink.set_callbacks(
		gst_app::AppSinkCallbacks::new()
			.new_sample(move |appsink| {
				let sample = appsink.pull_sample().expect("Failed to pull appsink preroll");
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

				Ok(gst::FlowSuccess::Ok)
			}).build());

	main_loop.run();
}
