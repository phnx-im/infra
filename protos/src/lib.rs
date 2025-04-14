pub mod auth_service;
pub mod common;
pub mod delivery_service;
pub mod error;
pub mod queue_service;

pub trait ToProto<P: prost::Message> {
    fn to_proto(&self) -> P;
}

pub trait TryToProto<P: prost::Message> {
    type Error;

    fn try_to_proto(&self) -> Result<P, Self::Error>;
}

pub trait IntoProto<P: prost::Message> {
    fn into_proto(self) -> P;
}

impl<T, P> IntoProto<P> for T
where
    T: Into<P>,
    P: prost::Message,
{
    fn into_proto(self) -> P {
        self.into()
    }
}
