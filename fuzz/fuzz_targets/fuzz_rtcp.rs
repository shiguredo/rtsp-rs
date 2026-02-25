#![no_main]

use libfuzzer_sys::fuzz_target;
use shiguredo_rtsp::rtcp::RtcpPacket;

fuzz_target!(|data: &[u8]| {
    if let Ok(packets) = RtcpPacket::parse(data) {
        let _ = RtcpPacket::build(&packets);
    }
});
