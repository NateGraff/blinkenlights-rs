extern crate gstreamer as gst;

use gst::prelude::*;

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

    // Create a file source
    let source = gst::ElementFactory::make("filesrc", Some("source"))
		.expect("Could not create filesrc element");
	source.set_property("location", &inputfile)
		.expect("Can't set uri on source");

	// Add video conversion and scaling and audio conversion to make sure
	// we can support the capabilities of the appsink we're setting.
    let vconvert = gst::ElementFactory::make("videoconvert", Some("vconvert"))
		.expect("Could not create videoconvert element");
	let scale = gst::ElementFactory::make("videoscale", Some("convert"))
		.expect("Could not create videoscale element");

	// Set an appsink and set the capabilities we want
	let sink = gst::ElementFactory::make("appsink", Some("sink"))
	    	.expect("Could not create appsink element");
	let caps = gst::Caps::builder("video/x-raw")
	    .field("format", &"RGB")
        .field("width", &160i32)
        .field("pixel-aspect-ratio", &"1/1")
        .build();
	sink.set_property("caps", &caps)
		.expect("Can't set caps on sink");

	// Create and link the pipeline
	let pipeline = gst::Pipeline::new(Some("my-pipeline"));
	pipeline.add_many(&[&source, &vconvert, &scale, &sink]).unwrap();

	source.link(&vconvert).expect("Could not link source and vconvert");
	vconvert.link(&scale).expect("Could not link vconvert and aconvert");
	scale.link(&sink).expect("Could not link scale and sink");

	// Set the pipeline state to paused to stay on a single frame
	pipeline.set_state(gst::State::Paused).expect("Unable to pause pipeline");
        
    println!("Hello, world!");
}
