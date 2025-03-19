// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::mem;

use flutter_rust_bridge::frb;
use phnxcoreclient::ConversationId;
use tokio::sync::watch;

use crate::{
    StreamSink,
    notifications::NotificationId,
    util::{Cubit, CubitCore},
};

/// State of the global App navigation
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, PartialEq, Eq, derive_more::From)]
pub enum NavigationState {
    /// Intro screen: welcome and registration screen
    Intro {
        #[frb(default = "[]")]
        screens: Vec<IntroScreenType>,
    },
    Home {
        #[frb(default = "HomeNavigationState()")]
        home: HomeNavigationState,
    },
}

/// Possible intro screens
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[frb(dart_metadata = ("freezed"))]
pub enum IntroScreenType {
    Intro,
    ServerChoice,
    UsernamePassword,
    DisplayNamePicture,
    DeveloperSettings,
}

/// Conversations screen: main screen of the app
///
/// Note: this can be represented in a better way disallowing invalid states.
/// For now, following KISS we represent the navigation stack in a very simple
/// way by just storing true/false or an optional value representing if a
/// screen is opened.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[frb(dart_metadata = ("freezed"))]
pub struct HomeNavigationState {
    pub conversation_id: Option<ConversationId>,
    pub developer_settings_screen: Option<DeveloperSettingsScreenType>,
    /// User name of the member that details are currently open
    pub member_details: Option<String>,
    #[frb(default = false)]
    pub user_settings_open: bool,
    #[frb(default = false)]
    pub conversation_details_open: bool,
    #[frb(default = false)]
    pub add_members_open: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[frb(dart_metadata = ("freezed"))]
pub enum DeveloperSettingsScreenType {
    Root,
    ChangeUser,
    Logs,
}

impl NavigationState {
    pub(crate) fn conversation_id(&self) -> Option<ConversationId> {
        match self {
            Self::Intro { .. } => None,
            Self::Home { home } => home.conversation_id,
        }
    }

    fn intro() -> Self {
        Self::Intro {
            screens: Vec::new(),
        }
    }

    fn home() -> NavigationState {
        Self::Home {
            home: HomeNavigationState::default(),
        }
    }
}

/// Provides the navigation state and navigation actions to the app
///
/// This is main entry point for navigation.
///
/// For the actual translation of the state to the actual screens, see
/// `AppRouter` in Dart.
pub struct NavigationCubitBase {
    core: CubitCore<NavigationState>,
}

impl NavigationCubitBase {
    #[frb(sync)]
    pub fn new() -> Self {
        let core = CubitCore::with_initial_state(NavigationState::intro());
        Self { core }
    }

    // Cubit interface

    #[frb(getter, sync)]
    pub fn is_closed(&self) -> bool {
        self.core.is_closed()
    }

    pub fn close(&mut self) {
        self.core.close();
    }

    #[frb(getter, sync)]
    pub fn state(&self) -> NavigationState {
        self.core.state()
    }

    pub async fn stream(&mut self, sink: StreamSink<NavigationState>) {
        self.core.stream(sink).await;
    }

    // Rust private methods

    #[frb(ignore)]
    pub(crate) fn subscribe(&self) -> watch::Receiver<NavigationState> {
        self.core.state_tx().subscribe()
    }

    // Cubit methods

    pub fn open_into(&self) {
        self.core.state_tx().send_modify(|state| {
            *state = NavigationState::intro();
        });
    }

    pub fn open_home(&self) {
        self.core.state_tx().send_modify(|state| {
            *state = NavigationState::home();
        });
    }

    pub fn open_conversation(&self, conversation_id: ConversationId) {
        self.core.state_tx().send_if_modified(|state| match state {
            NavigationState::Intro { .. } => {
                *state = HomeNavigationState {
                    conversation_id: Some(conversation_id),
                    ..Default::default()
                }
                .into();
                true
            }
            NavigationState::Home { home } => {
                home.conversation_id.replace(conversation_id) != Some(conversation_id)
            }
        });
    }

    pub async fn open_conversation_with_cleared_notifications(
        &self,
        conversation_id: ConversationId,
    ) {
        self.open_conversation(conversation_id);

        let mut pending = notifications::delivered().await;
        pending.retain(|s| {
            let id = s.parse::<NotificationId>();
            if let Ok(id) = id {
                id.belongs_to(conversation_id)
            } else {
                false
            }
        });
        notifications::remove(&pending);
    }

    pub fn close_conversation(&self) {
        self.core.state_tx().send_if_modified(|state| match state {
            NavigationState::Intro { .. } => false,
            NavigationState::Home { home } => home.conversation_id.take().is_some(),
        });
    }

    pub fn open_member_details(&self, member: String) {
        self.core.state_tx().send_if_modified(|state| match state {
            NavigationState::Intro { .. } => false,
            NavigationState::Home { home } => match home.member_details.as_mut() {
                Some(value) if *value != member => {
                    *value = member;
                    true
                }
                None => {
                    home.member_details.replace(member);
                    true
                }
                _ => false,
            },
        });
    }

    pub fn open_conversation_details(&self) {
        self.core.state_tx().send_if_modified(|state| match state {
            NavigationState::Intro { .. } => false,
            NavigationState::Home { home } => {
                !mem::replace(&mut home.conversation_details_open, true)
            }
        });
    }

    pub fn open_add_members(&self) {
        self.core.state_tx().send_if_modified(|state| match state {
            NavigationState::Intro { .. } => false,
            NavigationState::Home { home } => !mem::replace(&mut home.add_members_open, true),
        });
    }

    pub fn open_user_settings(&self) {
        self.core.state_tx().send_if_modified(|state| match state {
            NavigationState::Intro { .. } => false,
            NavigationState::Home { home } => !mem::replace(&mut home.user_settings_open, true),
        });
    }

    pub fn open_developer_settings(&self, screen: DeveloperSettingsScreenType) {
        self.core.state_tx().send_if_modified(|state| match state {
            NavigationState::Intro { screens } => {
                if screens.last() != Some(&IntroScreenType::DeveloperSettings) {
                    screens.push(IntroScreenType::DeveloperSettings);
                    true
                } else {
                    false
                }
            }
            NavigationState::Home { home } => {
                home.developer_settings_screen.replace(screen) != Some(screen)
            }
        });
    }

    pub fn open_intro_screen(&self, screen: IntroScreenType) {
        self.core.state_tx().send_if_modified(|state| match state {
            NavigationState::Intro { screens } => {
                if screens.last() != Some(&screen) {
                    screens.push(screen);
                    true
                } else {
                    false
                }
            }
            NavigationState::Home { .. } => false,
        });
    }

    #[frb(sync)]
    pub fn pop(&self) -> bool {
        self.core.state_tx().send_if_modified(|state| match state {
            NavigationState::Intro { screens } => screens.pop().is_some(),
            NavigationState::Home {
                home:
                    home @ HomeNavigationState {
                        developer_settings_screen: Some(DeveloperSettingsScreenType::Root),
                        ..
                    },
            } => {
                home.developer_settings_screen.take();
                true
            }
            NavigationState::Home {
                home:
                    home @ HomeNavigationState {
                        developer_settings_screen:
                            Some(
                                DeveloperSettingsScreenType::ChangeUser
                                | DeveloperSettingsScreenType::Logs,
                            ),
                        ..
                    },
            } => {
                home.developer_settings_screen
                    .replace(DeveloperSettingsScreenType::Root);
                true
            }
            NavigationState::Home { home } if home.user_settings_open => {
                home.user_settings_open = false;
                true
            }
            NavigationState::Home { home } if home.member_details.is_some() => {
                home.member_details.take();
                true
            }
            NavigationState::Home { home }
                if home.conversation_id.is_some() && home.add_members_open =>
            {
                home.add_members_open = false;
                true
            }
            NavigationState::Home { home }
                if home.conversation_id.is_some() && home.conversation_details_open =>
            {
                home.conversation_details_open = false;
                true
            }
            NavigationState::Home { home } if home.conversation_id.is_some() => {
                home.conversation_id.take();
                true
            }
            NavigationState::Home { .. } => false,
        })
    }
}
