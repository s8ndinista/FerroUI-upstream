//! Port of `Vnc/VncPasswordChallenge.cs` of RemoteViewing (BSD-2-Clause,
//! Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).
//!
//! The random numbers of the challenge come from the operating system
//! (`getrandom`, as `RandomNumberGenerator` of .NET) and DES is the one of
//! the RustCrypto project (`des`), in place of the providers of .NET.

use super::{IVncPasswordChallenge, VncStream};
use crate::remote_viewing::utility::AutoClear;
use des::cipher::{Array, BlockCipherEncrypt, KeyInit};
use des::Des;

/// Provides helper methods which implement the VNC password challenge
/// protocol.
#[derive(Clone, Copy, Debug, Default)]
pub struct VncPasswordChallenge;

impl VncPasswordChallenge {
    /// A new password challenge.
    pub fn new() -> Self {
        Self
    }

    /// Reverses the bits of a byte.
    /// See <http://www.vidarholen.net/contents/junk/vnc.html>.
    fn reverse_bits(value: u8) -> u8 {
        let mut out_value = 0u8;
        for i in 0..8 {
            if (value & (1 << i)) != 0 {
                out_value |= 128 >> i;
            }
        }

        out_value
    }
}

impl IVncPasswordChallenge for VncPasswordChallenge {
    /// Generates a 16-byte challenge.
    ///
    /// # Panics
    /// When the operating system has no random numbers to give.
    fn generate_challenge(&self) -> Vec<u8> {
        let mut challenge = vec![0u8; 16];
        getrandom::fill(&mut challenge).expect("the operating system provides random numbers");
        challenge
    }

    fn get_challenge_response_chars(
        &self,
        challenge: &[u8],
        password: &[char],
        response: &mut [u8],
    ) {
        let mut password_bytes = VncStream::encode_chars(password);
        let password_bytes = AutoClear::new(&mut password_bytes);
        self.get_challenge_response(challenge, password_bytes.array(), response);
    }

    fn get_challenge_response(&self, challenge: &[u8], password: &[u8], response: &mut [u8]) {
        assert!(
            challenge.len() == 16,
            "Challenge must be 16 bytes. (Parameter 'challenge')"
        );
        assert!(
            response.len() == 16,
            "Response must be 16 bytes. (Parameter 'response')"
        );

        let mut key = [0u8; 8];
        let length = password.len().min(key.len());
        key[..length].copy_from_slice(&password[..length]);
        for byte in &mut key {
            *byte = Self::reverse_bits(*byte);
        }

        let des = Des::new(&Array::from(key));
        drop(AutoClear::new(&mut key));

        // ECB: each half of the challenge on its own.
        for half in 0..2 {
            let mut block = Array::from(
                <[u8; 8]>::try_from(&challenge[half * 8..half * 8 + 8]).expect("8 bytes"),
            );
            des.encrypt_block(&mut block);
            response[half * 8..half * 8 + 8].copy_from_slice(&block);
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn the_challenge_is_random() {
        let challenge = VncPasswordChallenge::new();
        let first = challenge.generate_challenge();
        let second = challenge.generate_challenge();
        assert_eq!(16, first.len());
        assert_ne!(first, second);
    }

    #[test]
    fn the_response_is_des_of_the_challenge_with_the_bit_reversed_password() {
        // The response of the password "password" to the challenge of the bytes 0 to 15.
        let challenge: Vec<u8> = (0..16).collect();
        let mut response = [0u8; 16];
        VncPasswordChallenge::new().get_challenge_response_chars(
            &challenge,
            &"password".chars().collect::<Vec<_>>(),
            &mut response,
        );

        let mut again = [0u8; 16];
        VncPasswordChallenge::new().get_challenge_response(&challenge, b"password", &mut again);
        assert_eq!(response, again);

        // Longer passwords count with their first eight characters.
        let mut longer = [0u8; 16];
        VncPasswordChallenge::new().get_challenge_response(&challenge, b"password123", &mut longer);
        assert_eq!(response, longer);

        // Encrypting with the key of the bit reversed password decrypts again.
        use des::cipher::BlockCipherDecrypt;
        let mut key = *b"password";
        for byte in &mut key {
            *byte = VncPasswordChallenge::reverse_bits(*byte);
        }
        let des = Des::new(&Array::from(key));
        let mut block = Array::from(<[u8; 8]>::try_from(&response[..8]).unwrap());
        des.decrypt_block(&mut block);
        assert_eq!(&challenge[..8], block.as_slice());
    }

    #[test]
    fn bits_are_reversed() {
        assert_eq!(0b1000_0000, VncPasswordChallenge::reverse_bits(0b0000_0001));
        assert_eq!(0b0000_1101, VncPasswordChallenge::reverse_bits(0b1011_0000));
    }
}
