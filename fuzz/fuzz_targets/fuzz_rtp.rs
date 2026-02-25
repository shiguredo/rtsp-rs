#![no_main]

use libfuzzer_sys::fuzz_target;
use shiguredo_rtsp::rtp::RtpPacket;

fuzz_target!(|data: &[u8]| {
    if let Ok(packet) = RtpPacket::parse(data) {
        let _ = packet.build();
    }
});
