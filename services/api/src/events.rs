use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct BoardEventHub {
    channels: Arc<Mutex<HashMap<Uuid, broadcast::Sender<BoardEvent>>>>,
}

impl BoardEventHub {
    pub fn subscribe(&self, retro_id: Uuid) -> broadcast::Receiver<BoardEvent> {
        let sender = self.sender(retro_id);
        let receiver = sender.subscribe();
        let _ = sender.send(BoardEvent::BoardSnapshot { retro_id });
        receiver
    }

    pub fn publish(&self, event: BoardEvent) {
        let sender = self.sender(event.retro_id());
        let _ = sender.send(event);
    }

    fn sender(&self, retro_id: Uuid) -> broadcast::Sender<BoardEvent> {
        let mut channels = self
            .channels
            .lock()
            .expect("board event hub mutex poisoned");
        channels
            .entry(retro_id)
            .or_insert_with(|| {
                let (sender, _) = broadcast::channel(128);
                sender
            })
            .clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BoardEvent {
    BoardSnapshot { retro_id: Uuid },
    CardChanged { retro_id: Uuid },
    ReadyChanged { retro_id: Uuid },
    PhaseChanged { retro_id: Uuid },
    ClusteringChanged { retro_id: Uuid },
}

impl BoardEvent {
    pub fn retro_id(&self) -> Uuid {
        match self {
            Self::BoardSnapshot { retro_id }
            | Self::CardChanged { retro_id }
            | Self::ReadyChanged { retro_id }
            | Self::PhaseChanged { retro_id }
            | Self::ClusteringChanged { retro_id } => *retro_id,
        }
    }
}
