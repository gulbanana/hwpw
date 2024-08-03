use crate::{Endec, EndecError};

#[test]
fn roundtrip_same_instance() {
    let mut endec = Endec::new(0);

    let plaintext = b"secret";

    let message = endec
        .enc::<32>(b"01234567890123456789012345678901", plaintext)
        .unwrap();

    let replaintext = endec
        .dec(b"01234567890123456789012345678901", &message)
        .unwrap();

    assert_eq!(plaintext, replaintext);
}

#[test]
fn roundtrip_separate_instances() {
    let mut endec1 = Endec::new(0);

    let plaintext = b"secret";

    let message = endec1
        .enc::<32>(b"01234567890123456789012345678901", plaintext)
        .unwrap();

    let mut endec2 = Endec::new(0);

    let replaintext = endec2
        .dec(b"01234567890123456789012345678901", &message)
        .unwrap();

    assert_eq!(plaintext, replaintext);
}

#[test]
fn incorrect_tag() {
    let mut endec1 = Endec::new(0);

    let plaintext = b"secret";

    let message = endec1
        .enc::<32>(b"01234567890123456789012345678901", plaintext)
        .unwrap();

    let mut endec2 = Endec::new(1);

    let err = endec2
        .dec(b"01234567890123456789012345678901", &message)
        .unwrap_err();

    assert_eq!(EndecError::DecryptionFailed, err);
}

#[test]
fn incorrect_key() {
    let mut endec1 = Endec::new(0);

    let plaintext = b"secret";

    let message = endec1
        .enc::<32>(b"01234567890123456789012345678901", plaintext)
        .unwrap();

    let mut endec2 = Endec::new(0);

    let err = endec2
        .dec(b"0123456789012345678901234567890x", &message)
        .unwrap_err();

    assert_eq!(EndecError::DecryptionFailed, err);
}

#[test]
fn incorrect_nonce() {
    let mut endec1 = Endec::new(0);

    let plaintext = b"secret";

    let mut message = endec1
        .enc::<32>(b"01234567890123456789012345678901", plaintext)
        .unwrap();

    message.nonce[0] += 1;

    let mut endec2 = Endec::new(0);

    let err = endec2
        .dec(b"0123456789012345678901234567890x", &message)
        .unwrap_err();

    assert_eq!(EndecError::DecryptionFailed, err);
}

#[test]
fn nonce_varies() {
    let mut endec1 = Endec::new(0);

    let plaintext = b"secret";

    let nonce1 = endec1
        .enc::<32>(b"01234567890123456789012345678901", plaintext)
        .unwrap()
        .nonce;

    let nonce2 = endec1
        .enc::<32>(b"01234567890123456789012345678901", plaintext)
        .unwrap()
        .nonce;

    assert_ne!(nonce1, nonce2);
}
