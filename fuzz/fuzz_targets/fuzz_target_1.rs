#![no_main]

use libfuzzer_sys::fuzz_target;
use phnxapplogic::api::markdown::MessageContent;

fuzz_target!(|data: &[u8]| {
    // fuzzed code goes here
    MessageContent::try_parse_markdown_raw(data.to_vec()).unwrap();
});
