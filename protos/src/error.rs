use std::fmt;

use tonic::Status;

#[derive(Debug, thiserror::Error)]
#[error("missing field: {field}")]
pub struct MissingFieldError<E: fmt::Display> {
    field: E,
}

impl<E: fmt::Display> From<MissingFieldError<E>> for Status {
    fn from(e: MissingFieldError<E>) -> Self {
        Status::invalid_argument(e.to_string())
    }
}

pub trait MissingFieldExt<T, E: fmt::Display> {
    fn ok_or_missing_field(self, field: E) -> Result<T, MissingFieldError<E>>;
}

impl<T, E: fmt::Display> MissingFieldExt<T, E> for Option<T> {
    fn ok_or_missing_field(self, field: E) -> Result<T, MissingFieldError<E>> {
        self.ok_or(MissingFieldError { field })
    }
}

pub struct InvalidTls<F> {
    error: tls_codec::Error,
    field: F,
}

impl<F: fmt::Display> From<InvalidTls<F>> for Status {
    fn from(InvalidTls { error, field }: InvalidTls<F>) -> Status {
        Status::invalid_argument(format!("Invalid TLS field {field}: {error}"))
    }
}

pub struct TlsFailed<F> {
    error: tls_codec::Error,
    field: F,
}

impl<F: fmt::Display> From<TlsFailed<F>> for Status {
    fn from(TlsFailed { error, field }: TlsFailed<F>) -> Status {
        Status::internal(format!("TLS serialization of {field} failed: {error}",))
    }
}

pub trait InvalidTlsExt {
    type Value;

    fn invalid_tls<F>(self, field: F) -> Result<Self::Value, InvalidTls<F>>;

    fn tls_failed<F>(self, field: F) -> Result<Self::Value, TlsFailed<F>>;
}

impl<T> InvalidTlsExt for Result<T, tls_codec::Error> {
    type Value = T;

    fn invalid_tls<F>(self, field: F) -> Result<T, InvalidTls<F>> {
        self.map_err(|error| InvalidTls { error, field })
    }

    fn tls_failed<F>(self, field: F) -> Result<T, TlsFailed<F>> {
        self.map_err(|error| TlsFailed { error, field })
    }
}
