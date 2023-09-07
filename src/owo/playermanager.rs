use std::{collections::HashMap, sync::Arc};
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    Mutex,
};

use windows::{
    Foundation::{EventRegistrationToken, TypedEventHandler},
    Media::Control::{
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    },
};

use crate::owo::player::Player;

#[allow(clippy::enum_variant_names)]
pub enum ManagerEvent {
    SessionsChanged,
    ActiveSessionChanged,
    SystemSessionChanged,
}

struct EventToken {
    sessions_changed_token: EventRegistrationToken,
    current_session_changed_token: EventRegistrationToken,
}
pub struct PlayerManager {
    session_manager: GlobalSystemMediaTransportControlsSessionManager,

    active_player_key: Option<String>,
    system_player_key: Option<String>,
    players: HashMap<String, Arc<Mutex<Player>>>,

    tx: Option<UnboundedSender<ManagerEvent>>,

    event_tokens: Option<EventToken>,
}

impl PlayerManager {
    pub async fn new() -> Option<Self> {
        if let Ok(_binding) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
            if let Ok(session_manager) = _binding.await {
                return Some(PlayerManager {
                    session_manager,

                    players: HashMap::new(),
                    active_player_key: None,
                    system_player_key: None,

                    tx: None,

                    event_tokens: None,
                });
            }
        }
        None
    }

    pub fn set_events(&mut self) -> UnboundedReceiver<ManagerEvent> {
        if let Some(_tokens) = &self.event_tokens {
            // deregister to register again, invalidates stuff
            self.unset_events();
        }

        let (tx, rx) = unbounded_channel();

        let sessions_changed_handler = TypedEventHandler::new({
            let tx = tx.clone();
            move |_, _| {
                let _ = tx.send(ManagerEvent::SessionsChanged);
                Ok(())
            }
        });

        let current_session_changed_handler = TypedEventHandler::new({
            let tx = tx.clone();
            move |_, _| {
                let _ = tx.send(ManagerEvent::SystemSessionChanged);
                Ok(())
            }
        });

        let sessions_changed_token = self
            .session_manager
            .SessionsChanged(&sessions_changed_handler)
            .unwrap();
        let current_session_changed_token = self
            .session_manager
            .CurrentSessionChanged(&current_session_changed_handler)
            .unwrap();

        self.event_tokens = Some(EventToken {
            sessions_changed_token,
            current_session_changed_token,
        });

        let _ = tx.send(ManagerEvent::SessionsChanged);

        self.tx = Some(tx);

        rx
    }

    pub fn unset_events(&mut self) {
        let _ = self
            .session_manager
            .RemoveSessionsChanged(self.event_tokens.as_mut().unwrap().sessions_changed_token);
        let _ = self.session_manager.RemoveCurrentSessionChanged(
            self.event_tokens
                .as_mut()
                .unwrap()
                .current_session_changed_token,
        );

        self.tx = None;
        self.event_tokens = None;
    }

    pub fn get_active_session(&self) -> Option<Arc<Mutex<Player>>> {
        if let Some(player_key) = &self.active_player_key {
            return Some(self.players.get(player_key)?.clone());
        }
        None
    }

    pub fn get_session(&self, aumid: &String) -> Option<Arc<Mutex<Player>>> {
        Some(self.players.get(aumid)?.clone())
    }

    pub fn get_sessions_keys(&self) -> Vec<String> {
        self.players
            .keys()
            .map(String::from)
            .collect::<Vec<String>>()
    }

    pub fn get_system_session(&self) -> Option<Arc<Mutex<Player>>> {
        if let Some(player_key) = &self.system_player_key {
            return Some(self.players.get(player_key)?.clone());
        }
        None
    }

    pub fn update_system_session(&mut self) {
        if let Ok(session) = self.session_manager.GetCurrentSession() {
            self.system_player_key = None;

            if let Ok(aumid) = session.SourceAppUserModelId() {
                let _aumid = aumid.to_string();
                if _aumid.is_empty() {
                    return;
                }

                self.system_player_key = Some(_aumid);
            }
        }
    }

    pub fn update_sessions(&mut self, denylist: Option<&Vec<String>>) {
        let mut player_keys: Vec<String> = Vec::new();
        if let Ok(sessions) = self.session_manager.GetSessions() {
            for session in sessions {
                if let Ok(aumid) = session.SourceAppUserModelId() {
                    let _aumid = aumid.to_string();
                    if _aumid.is_empty() {
                        continue;
                    }

                    if denylist.is_some_and(|x| x.contains(&_aumid)) {
                        continue;
                    }

                    player_keys.push(_aumid.clone());

                    if !self.players.contains_key(&_aumid) {
                        let player = Arc::new(Mutex::new(Player::new(session, _aumid.clone())));
                        self.players.insert(_aumid.clone(), player);
                    }
                }
            }

            for key in self.players.clone().keys() {
                if !player_keys.contains(key) {
                    self.players.remove(key);
                }
            }

            self.update_active_player(self.active_player_key.clone());
        }
    }

    fn update_active_player(&mut self, preferred: Option<String>) {
        if let Ok(sessions) = self.session_manager.GetSessions() {
            let old = self.active_player_key.clone();
            self.active_player_key = None;

            for session in sessions {
                if let Ok(aumid) = session.SourceAppUserModelId() {
                    let _aumid = aumid.to_string();
                    if _aumid.is_empty() {
                        continue;
                    }
                    if !self.players.contains_key(&_aumid) {
                        continue;
                    }
                    if session.GetPlaybackInfo().unwrap().PlaybackStatus().unwrap()
                        == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
                    {
                        self.active_player_key = Some(_aumid.to_string());
                        break;
                    }
                }
            }
            if self.active_player_key.is_none()
                && preferred.is_some()
                && self.players.contains_key(&preferred.clone().unwrap())
            {
                self.active_player_key = preferred.clone();
            }
            if self.active_player_key.is_none()
                && self.system_player_key.is_some()
                && self
                    .players
                    .contains_key::<String>(&self.system_player_key.clone().unwrap())
            {
                self.active_player_key = preferred.clone();
            }
            if self.active_player_key.is_none() && !self.players.is_empty() {
                self.active_player_key = Some(
                    self.players
                        .keys()
                        .collect::<Vec<_>>()
                        .get(0)
                        .unwrap()
                        .to_string(),
                )
            }
            if dbg!(!old.eq(&self.active_player_key)) {
                if let Some(tx) = &self.tx {
                    let _ = tx.send(ManagerEvent::ActiveSessionChanged);
                }
            }
        }
    }
}

impl Drop for PlayerManager {
    fn drop(&mut self) {
        self.unset_events();
    }
}
