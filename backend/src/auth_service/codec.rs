use opaque_ke::RegistrationRequest;
use tls_codec::Size;

use crate::crypto::OPAQUE_REGISTRATION_REQUEST_SIZE;

use super::OpaqueRegistrationRequest;

impl Size for OpaqueRegistrationRequest {
    fn tls_serialized_len(&self) -> usize {
        OPAQUE_REGISTRATION_REQUEST_SIZE
    }
}

impl tls_codec::Serialize for OpaqueRegistrationRequest {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        let serialized_request = self.client_message.serialize().to_vec();
        let len = writer.write(&serialized_request)?;
        Ok(len)
    }
}

impl tls_codec::Deserialize for OpaqueRegistrationRequest {
    fn tls_deserialize<R: std::io::Read>(bytes: &mut R) -> Result<Self, tls_codec::Error>
    where
        Self: Sized,
    {
        let mut serialized_request = vec![0u8; OPAQUE_REGISTRATION_REQUEST_SIZE];
        bytes.read(&mut serialized_request);
        let client_message = RegistrationRequest::deserialize(&serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        Ok(Self { client_message })
    }
}
