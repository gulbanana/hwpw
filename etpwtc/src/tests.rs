use etpwtc_macros::encrypted;
use etpwtc_runtime::{Endec, Secret};

const BAKED: Secret<64> = encrypted!(b"ababxy":0, b"Test string - should be decrypted");

#[test]
fn integration_test() {
    let mut endec = Endec::new(0);
    let key = Endec::make_key(b"ababxy");

    assert!(
        matches!(endec.dec(&key, &BAKED), Ok(buffer) if buffer.as_slice() == b"Test string - should be decrypted")
    );
}
