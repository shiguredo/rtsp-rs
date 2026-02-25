#![no_main]
use libfuzzer_sys::fuzz_target;
use shiguredo_rtsp::RtspClientConnection;

fuzz_target!(|data: &[u8]| {
    let mut conn = RtspClientConnection::new();

    // Feed data and ignore errors (we're testing for crashes, not correctness)
    let _ = conn.feed_recv_buf(data);

    // Drain all events
    while conn.next_event().is_some() {}
});
