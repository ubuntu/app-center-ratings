//! Contains a generic authenticator implementation for use in different backends

use hyper::header::{self, HeaderValue};
use tracing::error;

pub mod admin;
pub mod jwt;

/// Verifies the given credentials, this is only really ever used by [`Authenticator`]
/// and unless you're adding a new auth method or endpoint, is probably useless to you.
pub trait CredentialVerifier {
    /// Any extensions that will be added to the request's extensions field before being
    /// passed down to the handler, should authentication succeed.
    type Extension: Send + Sync;
    /// Any errors that can be encountered during the verification procedure. These must
    /// be convertable to [`tonic::Status`] values, especially so anything sensitive can be
    /// erased before sending the error back to the client.
    type Error: std::error::Error;

    /// Verifies the passed in header has the authentication values necessary. This does
    /// NOT need to verify paths, nor that the header is actually an authorization header,
    /// the [`Authenticator`] does that already. However, it should validate things like
    /// header length, authentication type (Basic, etc) on its own.
    fn verify(&self, credential: &HeaderValue) -> Result<Option<Self::Extension>, Self::Error>;

    /// Returns this verifier's error but with the authenticator's message. This is mostly
    /// a workaround for the awkward way multiplexing makes us use errors, it becomes hard
    /// to commit to returning both valid GRPC/tonic and HTTP/axum responses from the [`Authenticator`]
    /// when it detects a problem upfront otherwise.
    fn unauthorized(&self, message: &str) -> Self::Error;
}

/// A utility meant to verify requests.
#[derive(Debug, Clone)]
pub struct Authenticator<V, A> {
    /// The paths that can be accessed without verifying whether a user is authorized
    public_paths: Vec<A>,
    /// A [`CredentialVerifier`] that will check if the auth header is valid for non-public paths.
    verifier: V,
}

impl<V: CredentialVerifier, A: AsRef<str>> Authenticator<V, A>
where
    V: CredentialVerifier,
    V::Extension: 'static,
    A: AsRef<str>,
{
    /// Attempts to authenticate the request's Authorization Header with the underlying [`CredentialVerifier`],
    /// unless the URL path was designated as public during construction.
    pub fn authenticate(&self, req: &mut hyper::Request<hyper::Body>) -> Result<(), V::Error> {
        let uri = req.uri().to_string();

        if self.public_paths.iter().any(|s| uri.ends_with(s.as_ref())) {
            return Ok(());
        }

        let Some(header) = req.headers().get(header::AUTHORIZATION.as_str()) else {
            let error = self.verifier.unauthorized("missing authz header");
            error!("{error}");
            return Err(error);
        };

        let extension = self
            .verifier
            .verify(header)
            .inspect_err(|err| error!("{err}"))
            .map_err(Into::into)?;

        if let Some(extension) = extension {
            req.extensions_mut().insert(extension);
        }
        Ok(())
    }
}

/// Builder pattern for [`Authenticator`], this is mostly used for iteratively adding
/// new public paths.
#[allow(clippy::missing_docs_in_private_items)]
pub struct AuthenticatorBuilder<V, A> {
    verifier: V,
    public_paths: Vec<A>,
}

impl<V: CredentialVerifier, A> AuthenticatorBuilder<V, A> {
    /// Constructs a new in-progress authenticator from the given [`CredentialVerifier`].
    pub fn new(verifier: V) -> Self {
        Self {
            verifier,
            public_paths: Vec::new(),
        }
    }

    /// Overrides an existing [`CredentialVerifier`] with a new one.
    #[allow(dead_code)]
    pub fn override_verifier<V2: CredentialVerifier>(
        self,
        verifier: V2,
    ) -> AuthenticatorBuilder<V2, A> {
        AuthenticatorBuilder {
            verifier,
            public_paths: self.public_paths,
        }
    }
}

impl<V, A: AsRef<str>> AuthenticatorBuilder<V, A> {
    /// Adds a new path that the constructed [`Authenticator`] will consider public,
    /// and thus pass without attempting to authenticate the passed header.
    pub fn with_public_path(mut self, path: A) -> Self {
        self.public_paths.push(path);
        self
    }

    /// Like [AuthenticatorBuilder::with_public_path], but adds multiple at once. This
    /// is slightly more efficient (and cleaner) than doing it yourself in a loop.
    pub fn with_public_paths<I: Iterator<Item = A>>(mut self, paths: I) -> Self {
        self.public_paths.extend(paths);
        self
    }
}

impl<V: CredentialVerifier, A: AsRef<str>> AuthenticatorBuilder<V, A> {
    /// Renders this builder into an [`Authenticator`], ready to be used.
    pub fn build(self) -> Authenticator<V, A> {
        Authenticator {
            verifier: self.verifier,
            public_paths: self.public_paths,
        }
    }
}

impl<V, A> Default for AuthenticatorBuilder<V, A>
where
    V: Default,
{
    fn default() -> Self {
        Self {
            verifier: Default::default(),
            public_paths: Default::default(),
        }
    }
}
