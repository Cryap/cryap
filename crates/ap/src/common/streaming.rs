use std::collections::HashMap;

use db::{
    models::{Notification, Post},
    types::DbId,
};
use lazy_static::lazy_static;
use tokio::sync::{broadcast, RwLock};

#[derive(Clone)]
pub enum StreamingEvent {
    Update {
        payload: Post,
        categories: Vec<StreamingCategory>,
    },
    Delete {
        payload: Post,
        categories: Vec<StreamingCategory>,
    },
    Notification {
        payload: Notification,
        categories: Vec<StreamingCategory>,
    },
    FiltersChanged {
        payload: (),
        categories: Vec<StreamingCategory>,
    },
    StatusUpdate {
        payload: Post,
        categories: Vec<StreamingCategory>,
    },
}

impl StreamingEvent {
    pub fn notification(notification: Notification) -> Self {
        Self::Notification {
            payload: notification,
            categories: vec![StreamingCategory::User, StreamingCategory::UserNotification],
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum StreamingCategory {
    Public,
    PublicMedia,
    PublicLocal,
    PublicLocalMedia,
    PublicRemote,
    PublicRemoteMedia,
    Hashtag,
    HashtagLocal,
    User,
    UserNotification,
    List,
    Direct,
}

impl StreamingCategory {
    pub fn by_name(name: &str) -> Option<StreamingCategory> {
        match name {
            "public" => Some(StreamingCategory::Public),
            "public:media" => Some(StreamingCategory::PublicMedia),
            "public:local" => Some(StreamingCategory::PublicLocal),
            "public:local:media" => Some(StreamingCategory::PublicLocalMedia),
            "public:remote" => Some(StreamingCategory::PublicRemote),
            "public:remote:media" => Some(StreamingCategory::PublicRemoteMedia),
            "hashtag" => Some(StreamingCategory::Hashtag),
            "hashtag:local" => Some(StreamingCategory::HashtagLocal),
            "user" => Some(StreamingCategory::User),
            "user:notification" => Some(StreamingCategory::UserNotification),
            "list" => Some(StreamingCategory::List),
            "direct" => Some(StreamingCategory::Direct),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        String::from(match self {
            StreamingCategory::Public => "public",
            StreamingCategory::PublicMedia => "public:media",
            StreamingCategory::PublicLocal => "public:local",
            StreamingCategory::PublicLocalMedia => "public:local:media",
            StreamingCategory::PublicRemote => "public:remote",
            StreamingCategory::PublicRemoteMedia => "public:remote:media",
            StreamingCategory::Hashtag => "hashtag",
            StreamingCategory::HashtagLocal => "hashtag:local",
            StreamingCategory::User => "user",
            StreamingCategory::UserNotification => "user:notification",
            StreamingCategory::List => "list",
            StreamingCategory::Direct => "direct",
        })
    }
}

lazy_static! {
    pub static ref EVENT_BUS: StreamingEventBus = StreamingEventBus::new();
}

pub struct StreamingEventBus {
    channels: RwLock<HashMap<DbId, broadcast::Sender<StreamingEvent>>>,
}

impl StreamingEventBus {
    pub fn new() -> Self {
        Self {
            channels: RwLock::default(),
        }
    }

    pub async fn get_receiver(&self, account_id: &DbId) -> StreamingReceiverGuard {
        let channels = self.channels.read().await;
        let sender = channels.get(account_id);
        if let Some(sender) = sender {
            StreamingReceiverGuard::new(account_id.clone(), sender.subscribe())
        } else {
            drop(channels);
            let mut channels = self.channels.write().await;
            let (sender, receiver) = broadcast::channel(10);
            channels.insert(account_id.clone(), sender);
            StreamingReceiverGuard::new(account_id.clone(), receiver)
        }
    }

    pub async fn send(&self, account_id: &DbId, event: StreamingEvent) {
        let channels = self.channels.read().await;
        if let Some(channel) = channels.get(account_id) {
            let _ = channel.send(event);
        }
    }

    pub async fn close(&self, account_id: &DbId) {
        let channels = self.channels.read().await;
        let sender = channels.get(account_id);
        let receiver_count = sender.map(|sender| sender.receiver_count()).unwrap_or(0);
        if receiver_count <= 1 {
            drop(channels);
            let mut channels = self.channels.write().await;
            let _ = channels.remove(account_id);
        }
    }
}

impl Default for StreamingEventBus {
    fn default() -> Self {
        Self::new()
    }
}

pub struct StreamingReceiverGuard {
    account_id: DbId,
    receiver: broadcast::Receiver<StreamingEvent>,
}

impl StreamingReceiverGuard {
    pub fn new(account_id: DbId, receiver: broadcast::Receiver<StreamingEvent>) -> Self {
        Self {
            account_id,
            receiver,
        }
    }
}

// Not used, but required by Rust
impl std::ops::Deref for StreamingReceiverGuard {
    type Target = broadcast::Receiver<StreamingEvent>;

    fn deref(&self) -> &Self::Target {
        &self.receiver
    }
}

impl std::ops::DerefMut for StreamingReceiverGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.receiver
    }
}

impl std::ops::Drop for StreamingReceiverGuard {
    fn drop(&mut self) {
        let account_id = self.account_id.clone();
        tokio::spawn(async move {
            EVENT_BUS.close(&account_id).await;
        });
    }
}
