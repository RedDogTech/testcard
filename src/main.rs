extern crate gstreamer as gst;
extern crate gstreamer_video as gst_video;

use crate::gst::prelude::*;
use anyhow::Error;
use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
#[display(fmt = "Received error from {src}: {error} (debug: {debug:?})")]
struct ErrorMessage {
    src: glib::GString,
    error: glib::Error,
    debug: Option<glib::GString>,
}

fn main() -> Result<(), Error> {
    gst::init()?;

    let pipeline = gst::Pipeline::default();

    let video_caps = gst_video::VideoCapsBuilder::new()
        .width(1920)
        .height(1080)
        .framerate((60, 1).into())
        .build();

    let video_source = gst::ElementFactory::make("videotestsrc")
        .property_from_str("pattern", "smpte")
        .property("is-live", true)
        .build()?;

    let overlay = gst::ElementFactory::make("timeoverlay")
        .property_from_str("text", "SDI Output:\n")
        .property_from_str("halignment", "center")
        .property_from_str("valignment", "center")
        .property_from_str("font-desc", "Sans, 36")
        .build()?;

    let caps = gst::ElementFactory::make("capsfilter")
        .property("caps", &video_caps)
        .build()?;

    let timecode = gst::ElementFactory::make("timecodestamper").build()?;
    let convert = gst::ElementFactory::make("videoconvert").build()?;

    let video_sink = gst::ElementFactory::make("decklinkvideosink")
        .property_from_str("mode", "1080p60")
        .property("sync", true)
        .build()?;

    let audio_source = gst::ElementFactory::make("audiotestsrc").build()?;
    let audio_sink = gst::ElementFactory::make("decklinkaudiosink").build()?;

    pipeline.add_many([
        &video_source,
        &overlay,
        &caps,
        &timecode,
        &convert,
        &video_sink,
        &audio_source,
        &audio_sink,
    ])?;

    gst::Element::link_many([
        &video_source,
        &overlay,
        &caps,
        &timecode,
        &convert,
        &video_sink,
    ])?;

    gst::Element::link_many([&audio_source, &audio_sink])?;

    pipeline.set_state(gst::State::Playing)?;

    let bus = pipeline
        .bus()
        .expect("Pipeline without bus. Shouldn't happen!");

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                pipeline.set_state(gst::State::Null)?;

                return Err(ErrorMessage {
                    src: msg
                        .src()
                        .map(|s| s.path_string())
                        .unwrap_or_else(|| glib::GString::from("UNKNOWN")),
                    error: err.error(),
                    debug: err.debug(),
                }
                .into());
            }
            MessageView::StateChanged(s) => {
                println!(
                    "State changed from {:?}: {:?} -> {:?} ({:?})",
                    s.src().map(|s| s.path_string()),
                    s.old(),
                    s.current(),
                    s.pending()
                );
            }
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null)?;

    Ok(())
}
