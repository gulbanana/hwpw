use etpwtc::encrypted;

pub const CODE_LENGTH: usize = 6;
pub const CODE_BUTTONS: endec::Secret = encrypted!(b"ababxy":0, b"CODE_BUTTONS");

pub const PASS_COUNT: usize = 2;
pub const PASS_NAMES: [[u8; 4]; PASS_COUNT] = [*b" XYZ", *b"ABCD"];
pub const PASS_USERS: [&'static [u8]; PASS_COUNT] = [b"xyz-user", b"abcd_user"];
pub const PASS_WORDS: [endec::Secret; PASS_COUNT] = [
    encrypted!(b"ababxy":1, b"{32>fFd!"),
    encrypted!(b"ababxy":2, b"sw0rd*f1sh"),
];
