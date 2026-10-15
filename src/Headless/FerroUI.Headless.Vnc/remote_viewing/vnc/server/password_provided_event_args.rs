//! Port of `Vnc/Server/PasswordProvidedEventArgs.cs` of RemoteViewing
//! (BSD-2-Clause, Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of
//! this crate).

use crate::remote_viewing::utility::AutoClear;
use crate::remote_viewing::vnc::IVncPasswordChallenge;
use std::cell::Cell;
use std::sync::Arc;

/// Provides data for the password provided event of the server session.
pub struct PasswordProvidedEventArgs {
    password_challenge: Arc<dyn IVncPasswordChallenge>,
    challenge: Vec<u8>,
    response: Vec<u8>,
    is_authenticated: Cell<bool>,
}

impl PasswordProvidedEventArgs {
    /// Initializes a new instance of the [`PasswordProvidedEventArgs`]
    /// class: `challenge` is what the server sent, `response` the bytes of
    /// the response from the client and `password_challenge` the object
    /// that is able to validate the password.
    ///
    /// # Panics
    /// When the challenge or the response is not 16 bytes long
    /// (`ArgumentOutOfRangeException`).
    pub fn new(
        password_challenge: Arc<dyn IVncPasswordChallenge>,
        challenge: &[u8],
        response: &[u8],
    ) -> Self {
        assert!(
            challenge.len() == 16,
            "Challenge must be 16 bytes. (Parameter 'challenge')"
        );
        assert!(
            response.len() == 16,
            "Response must be 16 bytes. (Parameter 'response')"
        );

        Self {
            password_challenge,
            challenge: challenge.to_vec(),
            response: response.to_vec(),
            is_authenticated: Cell::new(false),
        }
    }

    /// Gets a value indicating whether the client has successfully
    /// authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.is_authenticated.get()
    }

    /// Successfully authenticates the client. Returns always true.
    pub fn accept(&self) -> bool {
        self.is_authenticated.set(true);
        true
    }

    /// Authenticates the client, if the password bytes match. Returns true
    /// if authentication succeeded.
    pub fn accept_bytes(&self, password: &[u8]) -> bool {
        let mut response = [0u8; 16];
        self.password_challenge
            .get_challenge_response(&self.challenge, password, &mut response);
        self.test(&mut response)
    }

    /// Authenticates the client, if the password characters match. Returns
    /// true if authentication succeeded.
    pub fn accept_chars(&self, password: &[char]) -> bool {
        let mut response = [0u8; 16];
        self.password_challenge.get_challenge_response_chars(
            &self.challenge,
            password,
            &mut response,
        );
        self.test(&mut response)
    }

    fn test(&self, response: &mut [u8]) -> bool {
        let response = AutoClear::new(response);
        if self.response != response.array() {
            return false;
        }

        self.accept()
    }
}

impl Drop for PasswordProvidedEventArgs {
    fn drop(&mut self) {
        // The session clears the challenge and the response once the event was raised.
        self.challenge.fill(0);
        self.response.fill(0);
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use crate::remote_viewing::vnc::VncPasswordChallenge;

    #[test]
    fn the_password_of_the_response_is_accepted_and_another_is_not() {
        let challenge_provider: Arc<dyn IVncPasswordChallenge> =
            Arc::new(VncPasswordChallenge::new());
        let challenge = challenge_provider.generate_challenge();
        let mut response = [0u8; 16];
        challenge_provider.get_challenge_response(&challenge, b"secret", &mut response);

        let wrong =
            PasswordProvidedEventArgs::new(challenge_provider.clone(), &challenge, &response);
        assert!(!wrong.accept_chars(&['o', 't', 'h', 'e', 'r']));
        assert!(!wrong.is_authenticated());

        let right = PasswordProvidedEventArgs::new(challenge_provider, &challenge, &response);
        assert!(right.accept_chars(&['s', 'e', 'c', 'r', 'e', 't']));
        assert!(right.is_authenticated());
    }
}
