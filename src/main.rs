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

	let pipeline = gst::Pipeline::new(Some("my-pipeline"));

	// Create a file source
	let source = gst::ElementFactory::make("filesrc", None)
		.expect("Could not create filesrc element");
	source.set_property("location", &inputfile)
		.expect("Can't set uri on source");
	let vconvert = gst::ElementFactory::make("videoconvert", None)
		.expect("Could not create videoconvert element");

	pipeline.add_many(&[&source, &vconvert]).unwrap();
	source.link(&vconvert).expect("Could not link source and decode");

	let pipeline_weak = pipeline.downgrade();

	vconvert.connect_pad_added(move |_, src_pad| {
		let pipeline = match pipeline_weak.upgrade() {
			Some(pipeline) => pipeline,
			None => return,
		};

		// Add video conversion and scaling and audio conversion to make sure
		// we can support the capabilities of the appsink we're setting.
		let scale = gst::ElementFactory::make("videoscale", None)
			.expect("Could not create videoscale element");

		// Add an appsink
		let sink = gst::ElementFactory::make("appsink", None)
				.expect("Could not create appsink element");

		let elements = &[&scale, &sink];
		pipeline.add_many(elements).unwrap();
		gst::Element::link_many(elements).unwrap();

		for e in elements {
			e.sync_state_with_parent().unwrap();
		}

		let sink_pad = scale.get_static_pad("sink")
			.expect("Could not get sink pad from vconvert");
		src_pad.link(&sink_pad).unwrap();

		// Cast the sink to an appsink and set the capabilities we want
		let appsink = sink
			.dynamic_cast::<AppSink>()
			.expect("Failed to cast sink to AppSink");
		let set_caps = gst::Caps::builder("video/x-raw")
			.field("format", &"RGB")
			.field("width", &160i32)
			.field("pixel-aspect-ratio", &"1/1")
			.build();
		appsink.set_caps(Some(&set_caps));
	});
	
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
