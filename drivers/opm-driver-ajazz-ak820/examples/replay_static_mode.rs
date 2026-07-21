//! Throwaway diagnostic — NOT part of the driver, not meant to stay.
//!
//! Replays the exact SET_REPORT sequence observed in a real capture
//! around the moment the vendor software's "Static" mode was clicked
//! (docs/protocols/ajazz-ak820/findings.md, 2026-07-20 entry), including
//! the surrounding "04 xx" heartbeat commands previously assumed to be
//! inert background polling. Testing whether the keyboard only accepts
//! `set_color`/`activate_static` while that heartbeat pattern is
//! present around it, since sending `activate_static` + `set_color` in
//! isolation (via `pmctl rgb set`) had no visible effect on real
//! hardware, even though the exact same bytes worked live in the real
//! capture.

use std::thread::sleep;
use std::time::Duration;

use opm_core::transport::Transport;
use opm_transport::HidTransport;

const PATH: &str = "/dev/hidraw4";

fn report(bytes: &[(usize, u8)]) -> [u8; 64] {
    let mut buf = [0u8; 64];
    for &(index, value) in bytes {
        buf[index] = value;
    }
    buf
}

fn main() {
    let target_r = 0xfe;
    let target_g = 0xb9;
    let target_b = 0x73;

    let transport = HidTransport::open(PATH).expect("open the AK820 vendor interface");
    let gap = Duration::from_millis(50);

    // Structure per github.com/TaxMachine/ajazz-keyboard-software-linux
    // (a community Linux tool for the closely related PID 0x8009 variant):
    // every mode/color write is bracketed START -> START_MODE -> data ->
    // FINISH. Byte values below are OUR device's own captured bytes
    // (PID 0x800a), not copied from that repo — its exact byte offsets
    // don't match ours 1:1 (see findings.md), only the 4-step shape does.
    let steps: Vec<(&str, [u8; 64])> = vec![
        ("START (04 18)", report(&[(0, 0x04), (1, 0x18)])),
        (
            "START_MODE (04 13)",
            report(&[(0, 0x04), (1, 0x13), (9, 0x01)]),
        ),
        (
            "data: mode=STATIC + target color",
            report(&[
                (0, 0x01),
                (1, target_r),
                (2, target_g),
                (3, target_b),
                (9, 0x05),
                (10, 0x03),
                (14, 0xaa),
                (15, 0x55),
            ]),
        ),
        ("FINISH (04 f0)", report(&[(0, 0x04), (1, 0xf0)])),
    ];

    for (label, payload) in steps {
        println!("-> {label}: {:02x?}", &payload[..17]);
        transport
            .set_feature(0, &payload)
            .unwrap_or_else(|err| panic!("set_feature failed on step {label:?}: {err}"));
        sleep(gap);

        let mut read_buf = [0u8; 64];
        match transport.get_feature(0, &mut read_buf) {
            Ok(len) => println!("   get_feature -> {:02x?}", &read_buf[..len.min(17)]),
            Err(err) => println!("   get_feature failed: {err}"),
        }
        sleep(gap);
    }

    println!("done — check the keyboard now");
}
