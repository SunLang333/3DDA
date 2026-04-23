use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

#[derive(Debug)]
pub struct LruCache<K, V>
where
    K: Clone + Eq + Hash,
{
    capacity: usize,
    entries: HashMap<K, V>,
    order: VecDeque<K>,
}

impl<K, V> LruCache<K, V>
where
    K: Clone + Eq + Hash,
{
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "capacity must be greater than zero");
        Self {
            capacity,
            entries: HashMap::with_capacity(capacity),
            order: VecDeque::new(),
        }
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.entries.contains_key(key) {
            self.touch(key);
            self.entries.get(key)
        } else {
            None
        }
    }

    pub fn set(&mut self, key: K, value: V) -> Option<(K, V)> {
        if self.entries.contains_key(&key) {
            self.entries.insert(key.clone(), value);
            self.touch(&key);
            return None;
        }

        self.order.push_front(key.clone());
        self.entries.insert(key.clone(), value);

        if self.entries.len() <= self.capacity {
            return None;
        }

        let lru_key = self.order.pop_back()?;
        self.entries.remove(&lru_key).map(|value| (lru_key, value))
    }

    pub fn snapshot(&self) -> Vec<(&K, &V)> {
        self.order
            .iter()
            .filter_map(|key| self.entries.get_key_value(key))
            .collect()
    }

    fn touch(&mut self, key: &K) {
        if let Some(index) = self.order.iter().position(|current| current == key) {
            self.order.remove(index);
        }
        self.order.push_front(key.clone());
    }
}
