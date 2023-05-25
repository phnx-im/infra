#[macro_use]
mod errors;
mod backend;
mod contacts;
mod conversations;
mod groups;
mod notifications;
mod providers;
mod types;
mod users;
mod utils;

#[cfg(feature = "dart-bridge")]
mod dart_api;

use std::collections::HashMap;

pub(crate) use crate::errors::*;
use crate::{backend::Backend, conversations::*, groups::*, types::*, users::*};

use notifications::{Notifiable, NotificationHub};
pub(crate) use openmls::prelude::*;
pub(crate) use openmls_rust_crypto::OpenMlsRustCrypto;

use ds_lib::{ClientInfo, GroupMessage};

use uuid::Uuid;

#[derive(Default)]
pub struct Corelib<T>
where
    T: Notifiable,
{
    backend: Option<Backend>,
    conversation_store: ConversationStore,
    group_store: GroupStore,
    self_user: Option<SelfUser>,
    notification_hub: NotificationHub<T>,
}

impl<T: Notifiable> Corelib<T> {
    pub fn new() -> Self {
        Self {
            backend: None,
            conversation_store: ConversationStore::default(),
            group_store: GroupStore::default(),
            self_user: None,
            notification_hub: NotificationHub::<T>::default(),
        }
    }

    /// Set the corelib's backend url.
    pub fn initialize_backend(&mut self, backend_url: &str) {
        self.backend = Some(Backend::new(backend_url));
    }

    /// Reset the backend.
    pub fn reset_backend(&mut self) -> Result<(), CorelibError> {
        match &self.backend {
            Some(backend) => match backend.reset_backend() {
                Ok(_) => Ok(()),
                Err(_) => Err(CorelibError::NetworkError),
            },
            None => Err(CorelibError::BackendNotInitialized),
        }
    }

    /// Create user
    pub fn create_user(&mut self, username: &str) -> Result<(), CorelibError> {
        match &self.backend {
            Some(backend) => {
                let user = SelfUser::new(username.to_string());
                match backend.register_client(&user) {
                    Ok(response) => {
                        log::debug!("Created new user: {:?}", response);
                        self.self_user = Some(user);
                        Ok(())
                    }
                    Err(error) => {
                        println!("Error creating user: {:?}", error);
                        Err(CorelibError::NetworkError)
                    }
                }
            }
            None => Err(CorelibError::BackendNotInitialized),
        }
    }

    /// List clients from the backend
    pub fn list_clients(&self) -> Result<Vec<ClientInfo>, CorelibError> {
        match &self.backend {
            Some(backend) => Ok(backend
                .list_clients()
                .map_err(|_| CorelibError::NetworkError)?
                .into()),
            None => Err(CorelibError::BackendNotInitialized),
        }
    }

    /// Create new group
    pub fn create_conversation(&mut self, title: &str) -> Result<Uuid, CorelibError> {
        match &mut self.self_user {
            Some(user) => match self.group_store.create_group(user) {
                Ok(conversation_id) => {
                    let attributes = ConversationAttributes {
                        title: title.to_string(),
                    };
                    self.conversation_store
                        .create_group_conversation(conversation_id, attributes);
                    Ok(conversation_id)
                }
                Err(e) => Err(CorelibError::GroupStore(e)),
            },
            None => Err(CorelibError::UserNotInitialized),
        }
    }

    /// Get existing conversations
    pub fn get_conversations(&self) -> Vec<Conversation> {
        self.conversation_store.conversations()
    }

    /// Invite user to an existing group
    pub fn invite_user(&mut self, group_id: Uuid, invited_user: &str) -> Result<(), CorelibError> {
        if let Some(backend) = &self.backend {
            if let Some(self_user) = &mut self.self_user {
                if let Ok(key_package_in) = backend.fetch_key_package(invited_user.as_bytes()) {
                    let key_package = key_package_in.0[0]
                        .1
                        .clone()
                        .validate(self_user.crypto_backend.crypto(), ProtocolVersion::Mls10)
                        .map_err(|_| CorelibError::InvalidKeyPackage)?;
                    let group = self.group_store.get_group_mut(&group_id).unwrap();
                    // Adds new member and staged commit
                    match group
                        .invite(self_user, key_package.clone(), backend)
                        .map_err(CorelibError::Group)
                    {
                        Ok(staged_commit) => {
                            let conversation_messages = staged_commit_to_conversation_messages(
                                &self_user.credential_with_key.credential,
                                staged_commit,
                            );
                            group.merge_pending_commit(&self_user)?;
                            for conversation_message in conversation_messages {
                                let dispatched_conversation_message =
                                    DispatchedConversationMessage {
                                        conversation_id: UuidBytes::from_uuid(&group_id),
                                        conversation_message: conversation_message.clone(),
                                    };
                                self.conversation_store
                                    .store_message(&group_id, conversation_message)?;
                                self.notification_hub
                                    .dispatch_message_notification(dispatched_conversation_message);
                            }
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    Err(CorelibError::NetworkError)
                }
            } else {
                Err(CorelibError::UserNotInitialized)
            }
        } else {
            Err(CorelibError::BackendNotInitialized)
        }
    }

    /// Send a message
    pub fn send_message(
        &mut self,
        group_id: Uuid,
        message: &str,
    ) -> Result<ConversationMessage, CorelibError> {
        match &self.self_user {
            Some(ref user) => {
                let sender = user.username.clone();
                // Generate ciphertext
                let group_message = self
                    .group_store
                    .create_message(user, &group_id, message)
                    .map_err(CorelibError::Group)?;

                // Store message locally
                let message = Message::Content(ContentMessage {
                    sender,
                    content: MessageContentType::Text(TextMessage {
                        message: message.to_string(),
                    }),
                });
                let conversation_message = new_conversation_message(message);
                self.conversation_store
                    .store_message(&group_id, conversation_message.clone())
                    .map_err(CorelibError::ConversationStore)?;

                // Send message to DS
                println!("Sending message to DS");
                match &self.backend {
                    Some(backend) => match backend.send_msg(&group_message) {
                        Ok(_) => Ok(conversation_message),
                        Err(e) => {
                            println!("Error sending message: {e}");
                            println!("Message: {:?}", group_message);
                            let bytes = &mut group_message.tls_serialize_detached().unwrap();
                            match GroupMessage::tls_deserialize(&mut bytes.as_slice()) {
                                Ok(_) => println!("Codec worked."),
                                Err(_) => println!("Codec did not work."),
                            }
                            Err(CorelibError::NetworkError)
                        }
                    },
                    None => Err(CorelibError::BackendNotInitialized),
                }
            }
            None => Err(CorelibError::UserNotInitialized),
        }
    }

    pub fn get_messages(&self, conversation_id: &Uuid, last_n: usize) -> Vec<ConversationMessage> {
        self.conversation_store.messages(conversation_id, last_n)
    }

    /// Process the queue messages from the DS
    pub fn process_queue_messages(
        &mut self,
        messages: Vec<MlsMessageIn>,
    ) -> Result<(), CorelibError> {
        let self_user = match &self.self_user {
            Some(self_user) => self_user,
            None => return Err(CorelibError::UserNotInitialized),
        };
        let mut group_queues: HashMap<Uuid, Vec<MlsMessageIn>> = HashMap::new();

        for message in messages {
            match message.wire_format() {
                WireFormat::PrivateMessage => {
                    println!("Received a private message");
                    let group_id = UuidBytes::from_bytes(
                        message
                            .clone()
                            .into_protocol_message()
                            .unwrap()
                            .group_id()
                            .as_slice(),
                    )
                    .as_uuid();
                    match group_queues.get_mut(&group_id) {
                        Some(group_queue) => {
                            group_queue.push(message);
                        }
                        None => {
                            group_queues.insert(group_id, vec![message]);
                        }
                    }
                }
                WireFormat::Welcome => {
                    if let Some(welcome) = message.into_welcome() {
                        println!("Received a Welcome message");
                        match Group::join_group(self_user, welcome) {
                            Ok(group) => {
                                let group_id = group.group_id();
                                match self.group_store.store_group(group) {
                                    Ok(()) => {
                                        let attributes = ConversationAttributes {
                                            title: "New conversation".to_string(),
                                        };
                                        self.conversation_store
                                            .create_group_conversation(group_id, attributes);
                                        self.notification_hub.dispatch_conversation_notification();
                                    }
                                    Err(_) => {
                                        println!("Group already exists");
                                    }
                                }
                            }
                            Err(e) => {
                                println!("Could not join group: {:?}", e);
                            }
                        }
                    }
                }
                _ => {
                    println!("Received an unsupported message type");
                }
            }
        }
        for (group_id, group_queue) in group_queues {
            self.process_messages(group_id, group_queue)?;
        }
        Ok(())
    }

    /// Process received messages by group
    pub fn process_messages(
        &mut self,
        group_id: Uuid,
        messages: Vec<MlsMessageIn>,
    ) -> Result<(), CorelibError> {
        match self.group_store.get_group_mut(&group_id) {
            Some(group) => {
                for message in messages {
                    match group.process_message(self.self_user.as_ref().unwrap(), message) {
                        Ok(processed_message) => {
                            let sender_credential = processed_message.credential().clone();
                            let conversation_messages = match processed_message.into_content() {
                                ProcessedMessageContent::ApplicationMessage(
                                    application_message,
                                ) => application_message_to_conversation_messages(
                                    &sender_credential,
                                    application_message,
                                ),
                                ProcessedMessageContent::ProposalMessage(_) => {
                                    unimplemented!()
                                }
                                ProcessedMessageContent::StagedCommitMessage(staged_commit) => {
                                    staged_commit_to_conversation_messages(
                                        &sender_credential,
                                        &staged_commit,
                                    )
                                }
                                ProcessedMessageContent::ExternalJoinProposalMessage(_) => todo!(),
                            };

                            for conversation_message in conversation_messages {
                                let dispatched_conversation_message =
                                    DispatchedConversationMessage {
                                        conversation_id: UuidBytes::from_uuid(&group_id),
                                        conversation_message: conversation_message.clone(),
                                    };
                                self.conversation_store
                                    .store_message(&group_id, conversation_message)?;
                                self.notification_hub
                                    .dispatch_message_notification(dispatched_conversation_message);
                            }
                        }
                        Err(e) => {
                            println!("Error occured while processing inbound messages: {:?}", e);
                        }
                    }
                }

                Ok(())
            }
            None => Err(CorelibError::GroupStore(GroupStoreError::UnknownGroup)),
        }
    }
}

// Expose FFI functions
//implement_dart_ffi!(Corelib);

#[test]
fn test_create_user() {
    use rand::prelude::*;

    let username = &format!("unittest_{}", random::<u64>());

    #[derive(Debug, Clone, Default)]
    struct Notifier {}

    impl Notifiable for Notifier {
        fn notify(&self, _notification: NotificationType) -> bool {
            true
        }
    }

    let mut corelib = Corelib::<Notifier>::default();
    corelib.initialize_backend("https://127.0.0.1");
    corelib
        .create_user(username)
        .expect("Could not create user");
}

#[test]
fn test_user_full_cycle() {
    use rand::prelude::*;

    #[derive(Debug, Clone, Default)]
    struct Notifier {}

    impl Notifiable for Notifier {
        fn notify(&self, _notification: NotificationType) -> bool {
            true
        }
    }

    let url = "https://127.0.0.1";
    let rand_str = format!("{}", random::<u64>());
    let alice = &format!("unittest_alice_{}", rand_str);
    let bob = &format!("unittest_bob_{}", rand_str);
    let group_name = "test_conversation";
    let message = "Hello world!";

    // Create user Alice
    let mut alice_corelib = Corelib::<Notifier>::default();
    alice_corelib.initialize_backend(url);
    alice_corelib
        .create_user(alice)
        .expect("Could not create user Alice");

    // Create user Bob
    let mut bob_corelib = Corelib::<Notifier>::default();
    bob_corelib.initialize_backend(url);
    bob_corelib
        .create_user(bob)
        .expect("Could not create user Bob");

    // Alice invites Bob
    println!("Create conversation");
    let group_uuid = alice_corelib
        .create_conversation(group_name)
        .expect("Could not create conversation.");

    println!("Send message in new conversation");
    alice_corelib
        .send_message(group_uuid, message)
        .expect("Could not send application message before invitation:");

    println!("Invite user Bob");
    alice_corelib
        .invite_user(group_uuid, bob)
        .expect("Could not invite user");

    // Alice sends a message
    println!("Send message after Bob joined");
    alice_corelib
        .send_message(group_uuid, message)
        .expect("Could not send application message after invitation:");

    // Bob retrieves messages
    println!("Bob retrieves messages");
    let bob_messages = bob_corelib.get_messages(&group_uuid, 1);

    assert_eq!(bob_messages.len(), 1);
    let bob_message = &bob_messages[0];

    if let Message::Content(bob_content_message) = &bob_message.message {
        if let MessageContentType::Text(text_message) = &bob_content_message.content {
            assert_eq!(&text_message.message, message);
        } else {
            panic!("Wrong content type");
        }
    } else {
        panic!("Wrong message type");
    }
}

#[test]
fn test_list_clients() {
    #[derive(Debug, Clone, Default)]
    struct Notifier {}

    impl Notifiable for Notifier {
        fn notify(&self, _notification: NotificationType) -> bool {
            true
        }
    }

    let mut corelib = Corelib::<Notifier>::default();
    corelib.initialize_backend("https://127.0.0.1");
    let clients = corelib.list_clients().expect("Could not fetch clients");
    println!("Client list:");
    for client in clients {
        println!("\t{}", client.client_name);
    }
}
