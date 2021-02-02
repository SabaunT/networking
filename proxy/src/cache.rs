use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

pub(super) type Cache = Arc<Mutex<CacheInner>>;
pub(super) type Response = Vec<u8>;

#[derive(Default)]
pub(super) struct CacheInner(HashMap<(SystemTime, String), Response>);

impl CacheInner {
    const TTL: u64 = 30;

    pub(super) fn get(&self, target_url: &String) -> Option<&Response> {
        let CacheInner(ref inner_map) = self;
        let key = inner_map.keys().find(|(when, url)| {
            url == target_url
                && SystemTime::now()
                .duration_since(*when)
                .map(|dur| dur.as_secs() <= Self::TTL)
                .expect("internal error: clock went backwards")
        });
        key.and_then(|key| inner_map.get(key))
    }

    pub(super) fn insert(&mut self, target_url: String, resp: Response) {
        let CacheInner(ref mut inner_map) = self;
        inner_map.insert((SystemTime::now(), target_url), resp);
    }
}