#![no_main]

use libfuzzer_sys::fuzz_target;
use shiguredo_rtsp::rtsp_connection::parse_interleaved_frame;

fuzz_target!(|data: &[u8]| {
    let _ = parse_interleaved_frame(data);
});
