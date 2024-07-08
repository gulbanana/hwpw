pub const CODE_LENGTH: usize = 6;
pub const CODE_BUTTONS: [u8; CODE_LENGTH] = *b"ababxy";

pub const PASS_COUNT: usize = 2;
pub const PASS_NAMES: [[u8; 4]; PASS_COUNT] = [*b" XYZ", *b"ABCD"];
pub const PASS_USERS: [&'static [u8]; PASS_COUNT] = [b"xyz-user", b"abcd_user"];
pub const PASS_WORDS: [&'static [u8]; PASS_COUNT] = [b"{32>fFd!", b"sw0rd*f1sh"];
