pub const PASSCODE: [u8; 4] = *b"abab";
pub const COUNT: usize = 2;
pub const DISPLAY_NAMES: [[u8; 4]; COUNT] = [*b" XYZ", *b"ABCD"];
pub const USERNAMES: [&'static [u8]; COUNT] = [b"xyz-user", b"abcd_user"];
pub const PASSWORDS: [&'static [u8]; COUNT] = [b"{32>fFd!", b"sw0rd*f1sh"];