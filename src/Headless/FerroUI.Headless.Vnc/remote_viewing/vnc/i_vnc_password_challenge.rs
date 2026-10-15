//! Port of `Vnc/IVncPasswordChallenge.cs` of RemoteViewing (BSD-2-Clause,
//! Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).

/// A common interface for types which implement the VNC password challenge
/// protocol.
pub trait IVncPasswordChallenge: Send + Sync {
    /// Generates a 16-byte challenge.
    fn generate_challenge(&self) -> Vec<u8>;

    /// Calculates a `response` for a password `challenge`, using the
    /// characters of a `password`.
    ///
    /// # Panics
    /// When the challenge or the response is not 16 bytes long
    /// (`ArgumentOutOfRangeException`).
    fn get_challenge_response_chars(
        &self,
        challenge: &[u8],
        password: &[char],
        response: &mut [u8],
    );

    /// Calculates a `response` for a password `challenge`, using the bytes
    /// of a `password`.
    ///
    /// # Panics
    /// When the challenge or the response is not 16 bytes long
    /// (`ArgumentOutOfRangeException`).
    fn get_challenge_response(&self, challenge: &[u8], password: &[u8], response: &mut [u8]);
}
