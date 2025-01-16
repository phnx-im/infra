// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::openmls_provider::PhnxOpenMlsProvider;
use anyhow::{anyhow, bail, Result};
use mls_assist::messages::AssistedMessageOut;
use openmls_traits::storage::StorageProvider;
use phnxtypes::{
    credentials::{
        keys::{ClientSigningKey, InfraCredentialSigningKey},
        ClientCredential, EncryptedClientCredential,
    },
    crypto::{
        ear::{
            keys::{
                ClientCredentialEarKey, EncryptedSignatureEarKey, GroupStateEarKey,
                SignatureEarKey, SignatureEarKeyWrapperKey, WelcomeAttributionInfoEarKey,
            },
            EarDecryptable, EarEncryptable,
        },
        hpke::{HpkeDecryptable, JoinerInfoDecryptionKey},
        signatures::{
            keys::{UserAuthSigningKey, UserAuthVerifyingKey},
            signable::{Signable, Verifiable},
        },
    },
    identifiers::{
        AsClientId, QsClientReference, QualifiedUserName, QS_CLIENT_REFERENCE_EXTENSION_TYPE,
    },
    keypackage_batch::{KeyPackageBatch, VERIFIED},
    messages::{
        client_ds::{
            DsJoinerInformationIn, InfraAadMessage, InfraAadPayload, UpdateClientParamsAad,
            WelcomeBundle,
        },
        client_ds_out::{
            CreateGroupParamsOut, DeleteGroupParamsOut, ExternalCommitInfoIn,
            SelfRemoveClientParamsOut, SendMessageParamsOut, UpdateClientParamsOut,
        },
        welcome_attribution_info::{
            WelcomeAttributionInfo, WelcomeAttributionInfoPayload, WelcomeAttributionInfoTbs,
        },
    },
    time::TimeStamp,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tls_codec::DeserializeBytes as TlsDeserializeBytes;

use crate::{
    clients::api_clients::ApiClients, contacts::ContactAddInfos,
    conversations::messages::TimestampedMessage, key_stores::leaf_keys::LeafKeys,
    mimi_content::MimiContent, utils::persistence::SqliteConnection, SystemMessage,
};
use std::collections::HashSet;

use openmls::{
    group::ProcessedWelcome,
    key_packages::KeyPackageBundle,
    prelude::{
        tls_codec::Serialize as TlsSerializeTrait, Capabilities, Ciphersuite, Credential,
        CredentialType, CredentialWithKey, Extension, ExtensionType, Extensions, GroupId,
        KeyPackage, LeafNodeIndex, MlsGroup, MlsGroupJoinConfig, MlsMessageOut, OpenMlsProvider,
        ProcessedMessage, ProcessedMessageContent, Proposal, ProposalType, ProtocolMessage,
        ProtocolVersion, QueuedProposal, RequiredCapabilitiesExtension, Sender, StagedCommit,
        UnknownExtension, PURE_PLAINTEXT_WIRE_FORMAT_POLICY,
    },
    treesync::{LeafNodeParameters, RatchetTree},
};

use super::{
    client_auth_info::{ClientAuthInfo, GroupMembership, StorableClientCredential},
    diff::{GroupDiff, StagedGroupDiff},
};
