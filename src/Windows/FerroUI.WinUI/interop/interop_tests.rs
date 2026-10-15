//! Tests of the bindings on every host. Not from upstream: the reference
//! has no bindings of its own.
//!
//! The identifier of an instance of a parameterized interface or delegate
//! is derived by the Windows Runtime from the signature of the instance (a
//! version 5 UUID of the namespace `11f47ad5-7b73-42c0-abae-878b1e16adee`
//! and the signature as UTF-8). The interface definition file declares each
//! instance with the identifier and, in the comment above it, the
//! signature: the test derives every identifier again from its signature.

use super::*;
use ferroui_microcom::{Guid, Interface};

/// SHA-1 (FIPS 180-4), which the derivation of the identifiers uses.
fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x6745_2301, 0xEFCD_AB89, 0x98BA_DCFE, 0x1032_5476, 0xC3D2_E1F0];
    let mut message = data.to_vec();
    let bit_length = (data.len() as u64) * 8;
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());
    for chunk in message.chunks(64) {
        let mut w = [0u32; 80];
        for (i, word) in chunk.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, word) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let temp = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    let mut digest = [0u8; 20];
    for (i, word) in h.iter().enumerate() {
        digest[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    digest
}

/// The identifier of a parameterized instance with the signature.
fn parameterized_iid(signature: &str) -> Guid {
    let namespace = 0x11f47ad5_7b73_42c0_abae_878b1e16adee_u128.to_be_bytes();
    let mut data = namespace.to_vec();
    data.extend_from_slice(signature.as_bytes());
    let hash = sha1(&data);
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    bytes[6] = (bytes[6] & 0x0F) | 0x50;
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    Guid::from_u128(u128::from_be_bytes(bytes))
}

#[test]
fn sha1_of_the_standard_vectors() {
    let hex = |digest: [u8; 20]| digest.iter().map(|b| format!("{b:02x}")).collect::<String>();
    assert_eq!("da39a3ee5e6b4b0d3255bfef95601890afd80709", hex(sha1(b"")));
    assert_eq!("a9993e364706816aba3e25717850c26c9cd0d89d", hex(sha1(b"abc")));
    assert_eq!(
        "84983e441c3bd26ebaae4aa1f95129e5e54670f1",
        hex(sha1(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"))
    );
}

#[test]
fn the_identifier_of_a_known_instance() {
    // IVectorView<String> of the Windows SDK headers
    // (__FIVectorView_1_HSTRING).
    assert_eq!(
        Guid::parse("2f13c006-a03a-5f69-b090-75a43e33423e").unwrap(),
        parameterized_iid("pinterface({bbe1fa4c-b0e3-4583-baef-1f1b2e483e56};string)")
    );
}

#[test]
fn every_parameterized_instance_has_the_identifier_of_its_signature() {
    let idl = include_str!("winui.idl");
    let lines: Vec<&str> = idl.lines().collect();
    let mut checked = 0;
    for (index, line) in lines.iter().enumerate() {
        let Some(comment) = line.strip_prefix("// ") else { continue };
        let Some((_, signature)) = comment.split_once(": ") else { continue };
        if !(signature.starts_with("pinterface(")) {
            continue;
        }
        let uuid_line = lines[index + 1];
        let uuid = uuid_line.trim().strip_prefix("[uuid(").and_then(|rest| rest.strip_suffix(")]")).unwrap();
        let declared = Guid::parse(&uuid.to_lowercase()).unwrap();
        assert_eq!(declared, parameterized_iid(signature), "{}", lines[index + 2]);
        checked += 1;
    }
    // Every instance the integration declares.
    assert_eq!(34, checked);
}

#[test]
fn the_bindings_have_the_identifiers_of_the_metadata() {
    // A few that the reference names in its code or that the system headers
    // carry.
    assert_eq!(Guid::parse("63aad0b8-7c24-40ff-85a8-640d944cc325").unwrap(), ISwapChainPanelNative::IID);
    assert_eq!(Guid::parse("af86e2e0-b12d-4c6a-9c5a-d7aa65101e90").unwrap(), IInspectable::IID);
    assert_eq!(Guid::parse("94ea2b94-e9cc-49e0-c0ff-ee64ca8f5b90").unwrap(), IAgileObject::IID);
    assert_eq!(Guid::parse("bf6608af-4041-47c3-b263-a918eb5eaef2").unwrap(), ICoreTextEditContext::IID);
    assert_eq!(Guid::parse("c3c01020-320c-5cf6-9d24-d396bbfa4d8b").unwrap(), IUIElement::IID);
}

#[test]
fn the_structures_have_the_layout_of_the_abi() {
    use std::mem::{align_of, size_of};
    assert_eq!(12, size_of::<CorePhysicalKeyStatus>());
    assert_eq!(8, size_of::<Point>());
    assert_eq!(16, size_of::<Rect>());
    assert_eq!(4, size_of::<Color>());
    assert_eq!(8, size_of::<EventRegistrationToken>());
    assert_eq!(8, size_of::<WindowId>());
    assert_eq!(8, align_of::<WindowId>());
    assert_eq!(8, size_of::<CoreTextRange>());
}
