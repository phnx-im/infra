pub mod v1 {
    use tls_codec::DeserializeBytes;

    tonic::include_proto!("delivery_service.v1");

    #[derive(Debug, thiserror::Error)]
    pub enum QsReferenceError {
        #[error("Missing client homeserver domain")]
        MissingClientHomeserverDomain,
        #[error("Missing sealed reference")]
        MissingSealedReference,
        #[error(transparent)]
        InvalidClientHomeserverDomain(#[from] phnxtypes::identifiers::FqdnError),
        #[error("Invalid sealed reference: {0}")]
        InvalidHpkeCiphertext(tls_codec::Error),
    }

    impl TryFrom<QsReference> for phnxtypes::identifiers::QsReference {
        type Error = QsReferenceError;

        fn try_from(value: QsReference) -> Result<Self, Self::Error> {
            let client_homeserver_domain = value
                .client_homeserver_domain
                .ok_or(QsReferenceError::MissingClientHomeserverDomain)?
                .try_into()?;
            let sealed_reference_bytes = value
                .sealed_reference
                .ok_or(QsReferenceError::MissingSealedReference)?
                .ciphertext
                .ok_or(QsReferenceError::MissingSealedReference)?
                .tls;
            let sealed_reference = openmls::prelude::HpkeCiphertext::tls_deserialize_exact_bytes(
                &sealed_reference_bytes,
            )
            .map_err(QsReferenceError::InvalidHpkeCiphertext)?
            .into();

            Ok(Self {
                client_homeserver_domain,
                sealed_reference,
            })
        }
    }

    impl TryFrom<MlsMessage> for openmls::framing::MlsMessageIn {
        type Error = tls_codec::Error;

        fn try_from(value: MlsMessage) -> Result<Self, Self::Error> {
            Self::tls_deserialize_exact_bytes(&value.tls)
        }
    }

    impl TryFrom<RatchetTree> for openmls::treesync::RatchetTreeIn {
        type Error = tls_codec::Error;

        fn try_from(value: RatchetTree) -> Result<Self, Self::Error> {
            Self::tls_deserialize_exact_bytes(&value.tls)
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("Invalid group state EAR key length")]
    pub struct InvalidGroupStateEarKeyLength;

    impl TryFrom<GroupStateEarKey> for phnxtypes::crypto::ear::keys::GroupStateEarKey {
        type Error = InvalidGroupStateEarKeyLength;

        fn try_from(value: GroupStateEarKey) -> Result<Self, Self::Error> {
            let bytes: [u8; 32] = value
                .key
                .as_slice()
                .try_into()
                .map_err(|_| InvalidGroupStateEarKeyLength)?;
            let key = phnxtypes::crypto::ear::keys::GroupStateEarKeySecret::from(bytes);
            Ok(key.into())
        }
    }
}
