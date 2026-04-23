use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::abstractions::{LocalChunkRealizer, MacroGenerator, SharedChunkStore};
use crate::domain::{LocalChunk, LocalChunkKey, MacroChunk, MacroChunkKey, WorldProfile};
use crate::error::{Result, WorldGenError};
use crate::lru_cache::LruCache;

pub type SharedLocalChunk = Arc<Mutex<LocalChunk>>;
type SharedMacroChunk = Arc<MacroChunk>;

#[derive(Debug, Clone, Copy)]
pub struct ChunkManagerOptions {
    pub macro_cache_size: usize,
    pub local_cache_size: usize,
}

impl Default for ChunkManagerOptions {
    fn default() -> Self {
        Self {
            macro_cache_size: 64,
            local_cache_size: 256,
        }
    }
}

pub struct ChunkManager {
    store: SharedChunkStore,
    macro_generator: Arc<dyn MacroGenerator>,
    local_realizer: Arc<dyn LocalChunkRealizer>,
    macro_cache: Mutex<LruCache<MacroChunkKey, SharedMacroChunk>>,
    local_cache: Mutex<LruCache<LocalChunkKey, SharedLocalChunk>>,
    macro_inflight: Mutex<HashMap<MacroChunkKey, Arc<Flight<SharedMacroChunk>>>>,
    local_inflight: Mutex<HashMap<LocalChunkKey, Arc<Flight<SharedLocalChunk>>>>,
    active_profile: RwLock<Option<WorldProfile>>,
}

#[derive(Debug)]
struct Flight<T>
where
    T: Clone,
{
    result: Mutex<Option<std::result::Result<T, String>>>,
    notify: Notify,
}

impl<T> Default for Flight<T>
where
    T: Clone,
{
    fn default() -> Self {
        Self {
            result: Mutex::new(None),
            notify: Notify::new(),
        }
    }
}

impl<T> Flight<T>
where
    T: Clone,
{
    async fn wait(&self, ct: &CancellationToken) -> Result<T> {
        loop {
            if let Some(result) = self.result.lock().clone() {
                return result.map_err(WorldGenError::state);
            }

            tokio::select! {
                _ = self.notify.notified() => {}
                _ = ct.cancelled() => return Err(WorldGenError::Cancelled),
            }
        }
    }

    fn finish(&self, result: std::result::Result<T, String>) {
        *self.result.lock() = Some(result);
        self.notify.notify_waiters();
    }
}

impl ChunkManager {
    pub async fn new(
        store: SharedChunkStore,
        macro_generator: Arc<dyn MacroGenerator>,
        local_realizer: Arc<dyn LocalChunkRealizer>,
        options: Option<ChunkManagerOptions>,
    ) -> Result<Self> {
        let options = options.unwrap_or_default();
        Ok(Self {
            store,
            macro_generator,
            local_realizer,
            macro_cache: Mutex::new(LruCache::new(options.macro_cache_size)),
            local_cache: Mutex::new(LruCache::new(options.local_cache_size)),
            macro_inflight: Mutex::new(HashMap::new()),
            local_inflight: Mutex::new(HashMap::new()),
            active_profile: RwLock::new(None),
        })
    }

    pub fn register_world_profile(&self, profile: WorldProfile) {
        *self.active_profile.write() = Some(profile);
    }

    pub async fn persist_world_profile(&self) -> Result<()> {
        let profile = self.required_profile()?;
        self.store.save_world_profile(&profile).await
    }

    pub async fn load_registered_world_profile(&self) -> Result<Option<WorldProfile>> {
        let loaded = self.store.load_world_profile().await?;
        if let Some(profile) = &loaded {
            self.register_world_profile(profile.clone());
        }
        Ok(loaded)
    }

    pub async fn get_or_create_macro_chunk(
        &self,
        key: MacroChunkKey,
        ct: &CancellationToken,
    ) -> Result<SharedMacroChunk> {
        if let Some(chunk) = self.macro_cache.lock().get(&key).cloned() {
            return Ok(chunk);
        }

        let (flight, is_leader) = {
            let mut inflight = self.macro_inflight.lock();
            if let Some(existing) = inflight.get(&key) {
                (existing.clone(), false)
            } else {
                let flight = Arc::new(Flight::default());
                inflight.insert(key, flight.clone());
                (flight, true)
            }
        };

        if is_leader {
            let result = self.load_or_create_macro_chunk_core(key, ct).await.map(Arc::new);
            match result {
                Ok(chunk) => {
                    self.cache_macro_chunk(key, chunk.clone(), ct).await?;
                    flight.finish(Ok(chunk.clone()));
                    self.macro_inflight.lock().remove(&key);
                    Ok(chunk)
                }
                Err(error) => {
                    flight.finish(Err(error.to_string()));
                    self.macro_inflight.lock().remove(&key);
                    Err(error)
                }
            }
        } else {
            flight.wait(ct).await
        }
    }

    pub async fn get_or_create_local_chunk(
        &self,
        key: &LocalChunkKey,
        ct: &CancellationToken,
    ) -> Result<SharedLocalChunk> {
        if let Some(chunk) = self.local_cache.lock().get(key).cloned() {
            return Ok(chunk);
        }

        let (flight, is_leader) = {
            let mut inflight = self.local_inflight.lock();
            if let Some(existing) = inflight.get(key) {
                (existing.clone(), false)
            } else {
                let flight = Arc::new(Flight::default());
                inflight.insert(key.clone(), flight.clone());
                (flight, true)
            }
        };

        if is_leader {
            let result = self
                .load_or_create_local_chunk_core(key, ct)
                .await
                .map(|chunk| Arc::new(Mutex::new(chunk)));
            match result {
                Ok(chunk) => {
                    self.cache_local_chunk(key.clone(), chunk.clone(), ct).await?;
                    flight.finish(Ok(chunk.clone()));
                    self.local_inflight.lock().remove(key);
                    Ok(chunk)
                }
                Err(error) => {
                    flight.finish(Err(error.to_string()));
                    self.local_inflight.lock().remove(key);
                    Err(error)
                }
            }
        } else {
            flight.wait(ct).await
        }
    }

    pub async fn persist_macro_chunk(&self, chunk: &MacroChunk) -> Result<()> {
        self.store.save_macro_chunk(chunk).await
    }

    pub async fn persist_local_chunk(&self, chunk: &SharedLocalChunk) -> Result<()> {
        let (should_persist, snapshot, key) = {
            let mut guard = chunk.lock();
            let key = guard.key.clone();
            if guard.is_dirty {
                guard.mark_persisted();
                (true, Some(guard.clone()), key)
            } else {
                (false, None, key)
            }
        };

        if should_persist {
            let snapshot = snapshot.expect("snapshot exists when persisting");
            if let Err(error) = self.store.save_local_chunk(&snapshot).await {
                chunk.lock().is_dirty = true;
                return Err(error);
            }
            return Ok(());
        }

        self.store.delete_local_chunk(&key).await
    }

    pub async fn save_dirty(&self, ct: &CancellationToken) -> Result<()> {
        self.persist_world_profile().await?;
        let dirty_chunks: Vec<_> = self
            .local_cache
            .lock()
            .snapshot()
            .into_iter()
            .map(|(_, value)| value.clone())
            .collect();
        for chunk in dirty_chunks {
            if ct.is_cancelled() {
                return Err(WorldGenError::Cancelled);
            }

            if chunk.lock().is_dirty {
                self.persist_local_chunk(&chunk).await?;
            }
        }
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.save_dirty(&CancellationToken::new()).await
    }

    async fn load_or_create_macro_chunk_core(
        &self,
        key: MacroChunkKey,
        ct: &CancellationToken,
    ) -> Result<MacroChunk> {
        if let Some(loaded) = self.store.load_macro_chunk(key).await? {
            return Ok(loaded);
        }

        let profile = self.required_profile()?;
        let generated = self.macro_generator.generate(key, &profile, ct)?;
        self.store.save_macro_chunk(&generated).await?;
        Ok(generated)
    }

    async fn load_or_create_local_chunk_core(
        &self,
        key: &LocalChunkKey,
        ct: &CancellationToken,
    ) -> Result<LocalChunk> {
        if let Some(loaded) = self.store.load_local_chunk(key).await? {
            return Ok(loaded);
        }

        let profile = self.required_profile()?;
        let macro_chunk = self
            .get_or_create_macro_chunk(key.owning_macro_chunk_key(), ct)
            .await?;
        self.local_realizer.realize(key, &macro_chunk, &profile, ct)
    }

    async fn cache_macro_chunk(
        &self,
        key: MacroChunkKey,
        chunk: SharedMacroChunk,
        ct: &CancellationToken,
    ) -> Result<()> {
        let evicted = self.macro_cache.lock().set(key, chunk);
        if let Some((_, evicted_chunk)) = evicted {
            if ct.is_cancelled() {
                return Err(WorldGenError::Cancelled);
            }
            self.store.save_macro_chunk(&evicted_chunk).await?;
        }
        Ok(())
    }

    async fn cache_local_chunk(
        &self,
        key: LocalChunkKey,
        chunk: SharedLocalChunk,
        ct: &CancellationToken,
    ) -> Result<()> {
        let evicted = self.local_cache.lock().set(key, chunk);
        if let Some((_, evicted_chunk)) = evicted {
            if ct.is_cancelled() {
                return Err(WorldGenError::Cancelled);
            }
            if evicted_chunk.lock().is_dirty {
                self.persist_local_chunk(&evicted_chunk).await?;
            }
        }
        Ok(())
    }

    fn required_profile(&self) -> Result<WorldProfile> {
        self.active_profile.read().clone().ok_or_else(|| {
            WorldGenError::state(
                "A world profile must be registered before chunks can be generated.",
            )
        })
    }
}
