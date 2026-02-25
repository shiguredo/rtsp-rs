#![no_main]

use libfuzzer_sys::fuzz_target;
use shiguredo_http11::RequestDecoder;

fuzz_target!(|data: &[u8]| {
    let mut decoder = RequestDecoder::new();
    let _ = decoder.feed(data);
    // パースが失敗してもパニックしなければOK
    let _ = decoder.decode();
});
