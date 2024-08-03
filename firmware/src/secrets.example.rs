use etpwtc::{encrypted, Secret};

pub const CODE_LENGTH: usize = 6;
pub const CODE_BUTTONS: Secret<64> =
    encrypted!(b"ababxy", b"The quick brown fox jumps over the lazy dog.");

pub const PASS_COUNT: usize = 2;
pub const PASS_NAMES: [[u8; 4]; PASS_COUNT] = [*b" XYZ", *b"ABCD"];
pub const PASS_USERS: [&'static [u8]; PASS_COUNT] = [b"xyz-user", b"abcd_user"];
pub const PASS_WORDS: [Secret<64>; PASS_COUNT] = [
    encrypted!(b"ababxy", b"{32>fFd!"),
    encrypted!(b"ababxy", b"sw0rd*f1sh"),
];
