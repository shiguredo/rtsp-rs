#![no_main]
use libfuzzer_sys::fuzz_target;
use shiguredo_rtsp::{RtspClientConnection, RtspConnectionLimits};

fuzz_target!(|data: &[u8]| {
    // Test with small limits to trigger limit checks more often
    let limits = RtspConnectionLimits {
        http_limits: shiguredo_http11::DecoderLimits {
            max_buffer_size: 4096,
            max_headers_count: 10,
            max_header_line_size: 256,
            max_body_size: 1024,
            ..Default::default()
        },
        max_interleaved_frame_size: 2048,
        validate_version: true,
    };

    // Test client connection
    {
        let mut conn = RtspClientConnection::with_limits(limits);
        let _ = conn.feed_recv_buf(data);
        while conn.next_event().is_some() {}
    }
});
