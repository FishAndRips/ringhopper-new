use alloc::collections::BTreeMap;
use ringhopper_structs::{EditableTag, TagPath};
use super::TagSet;
use alloc::boxed::Box;
use alloc::borrow::ToOwned;
use alloc::collections::BTreeSet;
use spin::rwlock::RwLock;

#[derive(Copy, Clone, PartialEq)]
pub enum CacheBehavior {
    WriteImmediate,
    WriteDelayed
}

pub struct Cache<T: TagSet> {
    delegate: T,
    behavior: CacheBehavior,
    cache: RwLock<BTreeMap<TagPath, Box<dyn EditableTag>>>,
    edited_tags: RwLock<BTreeSet<TagPath>>
}

impl<T: TagSet> Cache<T> {
    pub const fn new(delegate: T, behavior: CacheBehavior) -> Self {
        Self {
            delegate, behavior, cache: RwLock::new(BTreeMap::new()), edited_tags: RwLock::new(BTreeSet::new())
        }
    }

    /// Flush the cache.
    ///
    /// Writes all cached tags back to the delegate.
    ///
    /// Does nothing if behavior is [CacheBehavior::WriteImmediate].
    pub fn flush(&self) {
        let mut cache = self.cache.write();
        let mut editing_cache = self.edited_tags.write();
        match self.behavior {
            CacheBehavior::WriteImmediate => {
                cache.clear();
            }
            CacheBehavior::WriteDelayed => {
                for i in editing_cache.iter() {
                    let entry = Box::as_ref(cache.get(i).expect("cache missing editing_cache contents"));
                    self.delegate.write_tag(i, entry);
                }
                cache.clear();
                editing_cache.clear();
            }
        }
    }

    /// Clear the cache, including writes.
    pub fn reset(&self) {
        self.cache.write().clear();
        self.edited_tags.write().clear();
    }
}

impl<T: TagSet> TagSet for Cache<T> {
    fn read_tag(&self, tag_path: &TagPath) -> Option<Box<dyn EditableTag>> {
        match self.cache.read().get(tag_path) {
            Some(t) => Some(t.clone_to_boxed_tag()),
            None => {
                let tag = self.delegate.read_tag(tag_path)?;
                self.cache.write().insert(tag_path.to_owned(), tag.clone_to_boxed_tag());
                Some(tag)
            }
        }
    }
    fn write_tag(&self, tag_path: &TagPath, tag: &dyn EditableTag) {
        let mut cache = self.cache.write();
        let mut editing_cache = self.edited_tags.write();
        match self.behavior {
            CacheBehavior::WriteImmediate => {
                cache.remove(tag_path);
                self.write_tag(tag_path, tag)
            }
            CacheBehavior::WriteDelayed => {
                cache.insert(tag_path.to_owned(), tag.clone_to_boxed_tag());
                editing_cache.insert(tag_path.to_owned());
            }
        }
    }
    fn has_tag(&self, tag_path: &TagPath) -> bool {
        self.cache.read().contains_key(tag_path) || self.delegate.has_tag(tag_path)
    }
}
