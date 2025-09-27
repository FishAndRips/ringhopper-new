use alloc::string::String;
use core::fmt::{Debug, Display, Formatter};
use crate::definitions::TagGroup;
use core::cmp::Ordering;
use crate::util::*;

/// Maximum length for a path.
pub const MAX_PATH_LEN: usize = 255;

/// Win32 path separator.
pub const HALO_PATH_SEPARATOR: char = '\\';

/// Unix path separator, also supported by Win32 (automatically converted to Win32 on load).
pub const UNIX_PATH_SEPARATOR: char = '/';

/// Return true if the character is a path separator.
#[inline]
#[must_use]
pub const fn is_path_separator(c: char) -> bool {
    #[cfg(feature = "std")]
    if c == std::path::MAIN_SEPARATOR {
        return true
    }
    c == HALO_PATH_SEPARATOR || c == UNIX_PATH_SEPARATOR
}

/// A list of characters banned in Win32 paths.
pub const WIN32_BANNED_PATH_CHARACTERS: &[char] = &[
    '"', '*', '/', ':', '<', '>', '?', '|'
];

// assert that WIN32_BANNED_PATH_CHARACTERS is ordered
const _: () = const {
    let mut last_read_index = 1usize;

    while last_read_index < WIN32_BANNED_PATH_CHARACTERS.len() {
        let a = WIN32_BANNED_PATH_CHARACTERS[last_read_index];
        let b = WIN32_BANNED_PATH_CHARACTERS[last_read_index - 1];
        assert!(a > b, "WIN32_BANNED_PATH_CHARACTERS is not ordered or has repeated elements");
        last_read_index += 1;
    }
};

/// A list of directory names that are banned in Win32 paths.
pub const WIN32_BANNED_DIRECTORIES: &[&str] = &[
    "aux",
    "com0",
    "com1",
    "com2",
    "com3",
    "com4",
    "com5",
    "com6",
    "com7",
    "com8",
    "com9",
    "con",
    "lpt0",
    "lpt1",
    "lpt2",
    "lpt3",
    "lpt4",
    "lpt5",
    "lpt6",
    "lpt7",
    "lpt8",
    "lpt9",
    "nul",
    "prn",
];

// assert that WIN32_BANNED_DIRECTORIES is ordered
const _: () = const {
    let mut last_read_index = 1usize;

    while last_read_index < WIN32_BANNED_DIRECTORIES.len() {
        let a = WIN32_BANNED_DIRECTORIES[last_read_index];
        let b = WIN32_BANNED_DIRECTORIES[last_read_index - 1];

        assert!(
            strcmp_const(a, b) as i8 == Ordering::Greater as i8,
            "WIN32_BANNED_DIRECTORIES is not ordered"
        );

        last_read_index += 1;
    }
};

/// Tag path primitive
///
/// Up to [`MAX_PATH_LEN`] characters are allowed.
///
/// All lowercase alphanumeric characters are allowed, as are some forms of punctuation.
///
/// Since Halo tag paths are a subset of Win32 tag paths, some characters and directory names are
/// not allowed (see [`WIN32_BANNED_PATH_CHARACTERS`] and [`WIN32_BANNED_DIRECTORIES`]), and paths
/// use Win32 path separators (see [`HALO_PATH_SEPARATOR`]).
///
/// Path separators are backslashes, and forward slashes are automatically converted into
/// backslashes. If the `std` feature is enabled, the system's native path separator is also
/// converted.
///
/// All uppercase characters are automatically converted into lowercase.
///
/// Control characters (e.g. `NUL`, `\n`, etc.) are not allowed.
#[derive(Clone, PartialEq, Ord, PartialOrd, Eq)]
pub struct TagPath {
    path: String,
    group: TagGroup
}

impl TagPath {
    /// Construct a tag reference from a path with combined path and group components.
    ///
    /// Return `Err` if the path is not valid or no extension is present.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_structs::TagPath;
    /// use ringhopper_structs::definitions::TagGroup;
    ///
    /// let path = TagPath::from_path_with_extension("weapons\\myweapon\\myweapon.isthebest.weapon")
    ///                 .expect("tag path should be valid");
    /// assert_eq!(path.path(), "weapons\\myweapon\\myweapon.isthebest");
    /// assert_eq!(path.group(), TagGroup::Weapon);
    ///
    /// let path = TagPath::from_path_with_extension("weapons/myweapon/myweapon.isthebest.weapon")
    ///                 .expect("tag path should be valid");
    /// assert_eq!(path.path(), "weapons\\myweapon\\myweapon.isthebest");
    /// assert_eq!(path.group(), TagGroup::Weapon);
    /// ```
    pub fn from_path_with_extension(p: &str) -> Result<TagPath, &'static str> {
        let Some(dot_pos) = p.rfind(".") else {
            return Err("no file extension found in path");
        };
        let (path, ext) = p.split_at(dot_pos);
        let Some(group) = TagGroup::from_str(&ext[1..]) else {
            return Err("invalid tag group")
        };

        Self::from_path_without_extension(path, group)
    }

    /// Construct a tag reference from a path with separate path and group components.
    ///
    /// Return `Err` if the path is not valid.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_structs::TagPath;
    /// use ringhopper_structs::definitions::TagGroup;
    ///
    /// let path = TagPath::from_path_without_extension("weapons\\myweapon\\myweapon.isthebest", TagGroup::Weapon)
    ///                 .expect("tag path should be valid");
    /// assert_eq!(path.path(), "weapons\\myweapon\\myweapon.isthebest");
    /// assert_eq!(path.group(), TagGroup::Weapon);
    ///
    /// let path = TagPath::from_path_without_extension("weapons/myweapon/myweapon.isthebest", TagGroup::Weapon)
    ///                 .expect("tag path should be valid");
    /// assert_eq!(path.path(), "weapons\\myweapon\\myweapon.isthebest");
    /// assert_eq!(path.group(), TagGroup::Weapon);
    /// ```
    pub fn from_path_without_extension(path: &str, group: TagGroup) -> Result<TagPath, &'static str> {
        let len = path.len();
        if len > MAX_PATH_LEN {
            return Err("maximum path length exceeded");
        }

        let mut path_buffer = String::new();
        path_buffer.try_reserve(len).map_err(|_| "failed to allocate RAM for tag path")?;

        for p in path.chars() {
            if is_path_separator(p) {
                let Some(c) = path_buffer.chars().next_back() else {
                    return Err("path starts with path separator")
                };

                if !is_path_separator(c) {
                    path_buffer.push(HALO_PATH_SEPARATOR);
                }
            }
            else if !p.is_ascii() {
                return Err("path contains a non-ASCII character (which is not allowed)")
            }
            else if p.is_ascii_control() {
                return Err("path contains control characters (which is not allowed)")
            }
            else if WIN32_BANNED_PATH_CHARACTERS.binary_search(&p).is_ok() {
                return Err("path contains characters not allowed on win32 file paths (which is not allowed)")
            }
            else {
                path_buffer.push(p.to_ascii_lowercase())
            }
        }

        let Some(last_char) = path_buffer.chars().next_back() else {
            return Err("no path given")
        };

        if is_path_separator(last_char) {
            return Err("empty filename")
        }

        for p in path.split(HALO_PATH_SEPARATOR) {
            if WIN32_BANNED_DIRECTORIES.binary_search(&p).is_ok() {
                return Err("path contains directory names not allowed on win32 file paths (which is not allowed")
            }
        }

        Ok(Self { path: path_buffer, group })
    }

    /// Return the tag path using Halo path separators (i.e. `\`).
    #[inline]
    #[must_use]
    pub const fn path(&self) -> &str {
        self.path.as_str()
    }

    /// Return a displayable version of the tag path.
    ///
    /// If the `std` feature is enabled, this will display with the system's native path separators.
    ///
    /// Otherwise, this will just display using Halo path separators.
    #[must_use]
    #[inline]
    pub const fn path_display(&self) -> impl Display {
        struct PathDisplay<'a> {
            path: &'a str
        }
        impl<'a> Display for PathDisplay<'a> {
            #[cfg(feature = "std")]
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                let main_separator = std::path::MAIN_SEPARATOR;

                if main_separator == HALO_PATH_SEPARATOR {
                    // same path separators; just display as is
                    return f.write_str(self.path);
                }

                let mut components = self.path.split(HALO_PATH_SEPARATOR).peekable();
                while let Some(q) = components.next() {
                    f.write_str(q)?;
                    if components.peek().is_some() {
                        f.write_char(main_separator)?;
                    }
                }

                Ok(())
            }
            #[cfg(not(feature = "std"))]
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                f.write_str(self.path)
            }
        }

        PathDisplay {
            path: self.path()
        }
    }

    /// Return the tag group.
    #[inline]
    #[must_use]
    pub const fn group(&self) -> TagGroup {
        self.group
    }

    /// Change the tag path.
    ///
    /// Return `Err` if the path is not valid.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper_structs::TagPath;
    /// use ringhopper_structs::definitions::TagGroup;
    ///
    /// let mut path = TagPath::from_path_without_extension("weapons\\myweapon\\myweapon.isthebest", TagGroup::Weapon)
    ///                 .expect("tag path should be valid");
    /// assert_eq!(path.path(), "weapons\\myweapon\\myweapon.isthebest");
    /// assert_eq!(path.group(), TagGroup::Weapon);
    ///
    /// path.set_path("my/new/path").expect("new tag path should be valid");
    /// assert_eq!(path.path(), "my\\new\\path");
    /// assert_eq!(path.group(), TagGroup::Weapon);
    /// ```
    #[inline]
    pub fn set_path(&mut self, new_path: &str) -> Result<(), &'static str> {
        self.path = Self::from_path_without_extension(new_path, self.group)?.path;
        Ok(())
    }

    /// Set the tag group.
    #[inline]
    pub const fn set_group(&mut self, new_group: TagGroup) {
        self.group = new_group;
    }
}

impl Debug for TagPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{}.{}", self.path, self.group))
    }
}

impl Display for TagPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{}.{}", self.path_display(), self.group))
    }
}

/// Tag reference primitive.
///
/// This can be set or unset. If unset, it still has a tag group associated with it.
#[derive(Clone, PartialEq, Debug)]
pub enum TagReference {
    Unset(TagGroup),
    Set(TagPath)
}

impl Default for TagReference {
    #[inline]
    fn default() -> Self {
        Self::Unset(TagGroup::None)
    }
}

impl Display for TagReference {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Set(path) => Display::fmt(&path, f),
            Self::Unset(group) => f.write_fmt(format_args!("<null>.{group}"))
        }
    }
}
