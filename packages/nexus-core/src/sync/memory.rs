use async_trait::async_trait;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use super::SyncEvent;
use crate::Result;

/// In-process sync bus for MVP. Replace with IPC in Beta.
pub struct MemorySyncBus {
    next_id: AtomicU64,
    tx: broadcast::Sender<(u64, SyncEvent)>,
    cursors: Arc<RwLock<std::collections::HashMap<String, u64>>>,
}

impl MemorySyncBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            next_id: AtomicU64::new(0),
            tx,
            cursors: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
}

impl Default for MemorySyncBus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::SyncBus for MemorySyncBus {
    async fn publish(&self, event: SyncEvent) -> Result<u64> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        let _ = self.tx.send((id, event));
        Ok(id)
    }

    async fn subscribe(
        &self,
        client_id: &str,
        since_event_id: u64,
    ) -> Result<tokio::sync::mpsc::Receiver<SyncEvent>> {
        let (out_tx, out_rx) = tokio::sync::mpsc::channel(64);
        let mut rx = self.tx.subscribe();
        {
            let mut cursors = self.cursors.write().await;
            cursors.insert(client_id.to_string(), since_event_id);
        }
        tokio::spawn(async move {
            while let Ok((id, ev)) = rx.recv().await {
                if id > since_event_id {
                    if out_tx.send(ev).await.is_err() {
                        break;
                    }
                }
            }
        });
        Ok(out_rx)
    }
}
