extern crate gstreamer as gst;

use gst::prelude::*;

use std::env;
use std::process::exit;

fn main() {
	let args: Vec<String> = env::args().collect();
	if args.len() < 2 {
		println!("{}: Please provide a video as argument", args[0]);
		exit(1);
	}
	let inputfile = &args[1];

    gst::init().unwrap();

    let source = gst::ElementFactory::make("filesrc", Some("source"))
    	.expect("Could not create playbin element");
	let sink = gst::ElementFactory::make("appsink", Some("sink"))
	    	.expect("Could not create appsink element");

	let pipeline = gst::Pipeline::new(Some("my-pipeline"));

	pipeline.add_many(&[&source, &sink]).unwrap();

	source.set_property("location", &inputfile)
		.expect("Can't set uri on source");
	source.link(&sink).expect("Could not link source and sink");

	pipeline.set_state(gst::State::Paused).expect("Unable to pause pipeline");
        
    println!("Hello, world!");
}
