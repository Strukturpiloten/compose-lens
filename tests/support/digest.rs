//! Byte-based hexadecimal encoding independent of a digest wrapper's formatting traits.

pub(crate) fn lowercase_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                char::from(HEX[usize::from(byte >> 4)]),
                char::from(HEX[usize::from(byte & 15)]),
            ]
        })
        .collect()
}

#[test]
fn hexadecimal_preserves_leading_zeroes_and_byte_boundaries() {
    assert_eq!(lowercase_hex(&[]), "");
    assert_eq!(lowercase_hex(&[0, 1, 15, 16, 128, 255]), "00010f1080ff");
}

#[test]
fn sha256_matches_an_independent_known_vector() {
    use sha2::{Digest, Sha256};

    assert_eq!(
        lowercase_hex(&Sha256::digest(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
