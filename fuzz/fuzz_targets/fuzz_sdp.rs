#![no_main]

use libfuzzer_sys::fuzz_target;
use shiguredo_rtsp::sdp::Sdp;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(sdp) = Sdp::parse(text) {
            let _ = sdp.to_string();
        }
    }
});
