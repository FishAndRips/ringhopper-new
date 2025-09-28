use alloc::collections::BTreeMap;
use ringhopper_structs::{EditableTag, Parameters, TagPath};
use super::{ReadTagError, Tagset, WriteTagError};
use alloc::boxed::Box;
use alloc::borrow::ToOwned;
use alloc::sync::Arc;
use spin::Mutex;
use spin::rwlock::RwLock;

#[derive(Copy, Clone, PartialEq)]
pub enum CacheBehavior {
    /// Writes are immediately committed into the tagset.
    WriteImmediate,

    /// Writes are not immediately committed into the tagset until manually flushed.
    WriteDelayed
}

/// A caching tag set
///
/// If T::is_writeable is false, then all writes will fail without any further interaction with the
/// delegate.
pub struct CachingTagset<T: Tagset> {
    delegate: T,
    behavior: CacheBehavior,
    cache: RwLock<InnerCache>
}

struct InnerCache {
    tag_cache: BTreeMap<TagPath, Arc<Mutex<Box<dyn EditableTag>>>>,
    edited_tags: BTreeMap<TagPath, Parameters>
}

impl InnerCache {
    fn get_caches(&mut self) -> (&mut BTreeMap<TagPath, Arc<Mutex<Box<dyn EditableTag>>>>, &mut BTreeMap<TagPath, Parameters>) {
        // SAFETY: These are independent of one another. It's fine to mutably borrow them simultaneously.
        let tag_cache_ptr = &mut self.tag_cache as *mut _;
        let edited_tags_ptr = &mut self.edited_tags as *mut _;
        unsafe { (&mut *tag_cache_ptr, &mut *edited_tags_ptr) }
    }
}

impl<T: Tagset> CachingTagset<T> {
    /// Instantiate a new caching tag set.
    #[inline]
    pub const fn new(delegate: T, behavior: CacheBehavior) -> Self {
        Self {
            delegate, behavior, cache: RwLock::new(InnerCache { tag_cache: BTreeMap::new(), edited_tags: BTreeMap::new() })
        }
    }

    /// Flush the cache.
    ///
    /// Writes all cached tags back to the delegate.
    ///
    /// Does nothing if behavior is [CacheBehavior::WriteImmediate].
    pub fn flush(&mut self) -> Result<(), BTreeMap<TagPath, WriteTagError>> {
        let mut cache = self.cache.write();
        let (tag_cache, edited_tags) = cache.get_caches();

        if tag_cache.is_empty() || self.behavior == CacheBehavior::WriteImmediate {
            return Ok(())
        }

        if !self.is_writeable() {
            return Err(
                cache.edited_tags
                    .iter()
                    .map(|c| (c.0.to_owned(), WriteTagError::ReadOnlyTagset))
                    .collect()
            );
        }

        let mut errors = BTreeMap::new();

        edited_tags.retain(|t, p| {
            let entry = tag_cache
                .get(t)
                .expect("cache missing editing_cache contents");
            if let Err(e) = self.delegate.write_tag(t, Box::as_ref(&entry.lock()), *p) {
                errors.insert(t.to_owned(), e);
                true
            }
            else {
                false
            }
        });

        tag_cache.retain(|path, _| { edited_tags.contains_key(path) });

        if errors.is_empty() {
            Ok(())
        }
        else {
            Err(errors)
        }
    }

    /// Clear the cache, including writes.
    #[inline]
    pub fn reset(&self) {
        let mut cache = self.cache.write();
        cache.tag_cache.clear();
        cache.edited_tags.clear();
    }

    /// Get the internal cache entry.
    #[inline]
    pub fn get_direct(&self, tag_path: &TagPath) -> Option<Arc<Mutex<Box<dyn EditableTag>>>> {
        self.cache.read().tag_cache.get(tag_path).cloned()
    }

    /// Load and get the internal cache entry.
    pub fn load_direct(&self, tag_path: &TagPath, parameters: Parameters) -> Result<Arc<Mutex<Box<dyn EditableTag>>>, ReadTagError> {
        if let Some(t) = self.cache.read().tag_cache.get(tag_path) {
            return Ok(t.clone())
        }

        let mut cache = self.cache.write();
        let tag = self.delegate.read_tag(tag_path, parameters)?;
        let inner = Arc::new(Mutex::new(tag));
        cache.tag_cache.insert(tag_path.to_owned(), inner.clone());
        Ok(inner)
    }

    pub fn mark_as_written(&self, tag_path: &TagPath, parameters: Parameters) {
        let mut write_cache = self.cache.write();

        if !write_cache.edited_tags.contains_key(tag_path) {
            panic!("tag was not opened");
        }

        write_cache.edited_tags.insert(tag_path.to_owned(), parameters);
    }
}

impl<T: Tagset> Tagset for CachingTagset<T> {
    #[inline]
    fn read_tag(&self, tag_path: &TagPath, parameters: Parameters) -> Result<Box<dyn EditableTag>, ReadTagError> {
        self.load_direct(tag_path, parameters).map(|t| t.lock().clone_to_boxed_tag())
    }
    fn write_tag(&mut self, tag_path: &TagPath, tag: &dyn EditableTag, parameters: Parameters) -> Result<(), WriteTagError> {
        if !self.is_writeable() {
            return Err(WriteTagError::ReadOnlyTagset)
        }

        let mut cache = self.cache.write();
        match self.behavior {
            CacheBehavior::WriteImmediate => {
                self.delegate.write_tag(tag_path, tag, parameters)?;
            }
            CacheBehavior::WriteDelayed => {
                cache.edited_tags.insert(tag_path.to_owned(), parameters);
            }
        }

        let cloned = Arc::new(Mutex::new(tag.clone_to_boxed_tag()));
        cache.tag_cache.insert(tag_path.clone(), cloned);
        Ok(())
    }
    #[inline]
    fn has_tag(&self, tag_path: &TagPath) -> bool {
        self.cache.read().tag_cache.contains_key(tag_path) || self.delegate.has_tag(tag_path)
    }
    #[inline]
    fn is_writeable(&self) -> bool {
        self.delegate.is_writeable()
    }
}
