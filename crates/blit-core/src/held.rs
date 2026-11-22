//! What a read-only walk reads, through what holds it (cr-uv9-1).
//!
//! `ls`, `du` and `find` read a tree. Read by its PATHS, each read is a new
//! look-up of every name from the top down: a directory replaced by a
//! link between two reads leads the next one wherever the link points — a
//! daemon's walk out of its module, past the check that let it start.
//!
//! On Unix a walk here HOLDS each directory it reads, open, and reads
//! what is in it through that: each entry looked up there, each directory
//! below entered there, never through a link. The start is reached the
//! same way — from the root the walk may not leave, one name at a time,
//! none of them a link; a link the start was typed to be read through is
//! resolved first, and its place must lie in the root. Anything replaced
//! by a link after that is an error, not a way out.
//!
//! Elsewhere the walk reads by path, as it always has.

use std::ffi::OsStr;
use std::path::Path;

use crate::listing::Kind;

/// What a look at one entry says: the entry itself, a link being a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Looked {
    pub kind: Kind,
    pub len: u64,
    /// When it was last written: nanoseconds since 1970, less than none
    /// before it.
    pub mtime_nanos: i128,
    /// The same, in whole seconds, as a header carries it.
    pub mtime_seconds: i64,
    /// The space allocated to it: 512-byte blocks on Unix; its length
    /// elsewhere, a directory counting nothing of its own.
    pub allocated: u64,
    /// Which file it is, and how many names it has, where the look says
    /// (Unix).
    pub identity: Option<(u64, u64)>,
    pub names: u64,
}

impl Looked {
    pub(crate) fn is_dir(&self) -> bool {
        self.kind == Kind::Directory
    }

    /// The look `metadata` is.
    #[cfg(not(unix))]
    pub(crate) fn of(metadata: &std::fs::Metadata) -> Self {
        Self {
            kind: Kind::of(metadata.file_type()),
            len: metadata.len(),
            mtime_nanos: metadata.modified().map_or(0, crate::find::nanos_of),
            mtime_seconds: crate::wire_metadata::mtime_seconds(metadata).unwrap_or(0),
            allocated: if metadata.is_dir() { 0 } else { metadata.len() },
            identity: None,
            names: 1,
        }
    }
}

#[cfg(unix)]
pub(crate) use imp::directory_unlinked;
pub(crate) use imp::{Directory, Listed, Start};

/// An entry a directory lists: its name, and its kind where the listing
/// says (no look-up).
impl Listed {
    pub(crate) fn name(&self) -> &OsStr {
        &self.name
    }

    pub(crate) fn kind(&self) -> Option<Kind> {
        self.kind
    }
}

#[cfg(test)]
thread_local! {
    static BEFORE_ENTERING: std::cell::RefCell<Option<Box<dyn FnMut(&Path)>>> =
        const { std::cell::RefCell::new(None) };
}

/// Test seam: the moment before a walk enters a directory, by the path it
/// would have by name — for a test to replace it. Production never sets
/// it.
fn before_entering(_path: &Path) {
    #[cfg(test)]
    BEFORE_ENTERING.with(|hook| {
        if let Some(hook) = hook.borrow_mut().as_mut() {
            hook(_path);
        }
    });
}

/// Run `hook` at each [`before_entering`] on this thread while the guard
/// lives.
#[cfg(all(test, unix))]
pub(crate) fn before_each_entering(hook: impl FnMut(&Path) + 'static) -> impl Drop {
    struct Cleared;
    impl Drop for Cleared {
        fn drop(&mut self) {
            BEFORE_ENTERING.with(|hook| hook.borrow_mut().take());
        }
    }
    BEFORE_ENTERING.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
    Cleared
}

/// Whether `start` was typed for the system to look through a link
/// standing at its last name: a separator, or a `.`, after it.
#[cfg(unix)]
fn typed_through(start: &Path) -> bool {
    let bytes = start.as_os_str().as_encoded_bytes();
    bytes.ends_with(b"/") || bytes.ends_with(b"/.")
}

#[cfg(unix)]
mod imp {
    use super::{before_entering, typed_through, Kind, Looked};
    use std::ffi::{CStr, CString, OsStr, OsString};
    use std::io;
    use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};
    use std::os::unix::io::{AsRawFd as _, FromRawFd as _, OwnedFd, RawFd};
    use std::path::{Component, Path, PathBuf};
    use std::ptr::NonNull;

    #[cfg(not(target_os = "linux"))]
    use libc::{fstat, fstatat, readdir, stat};
    #[cfg(target_os = "linux")]
    use libc::{fstat64 as fstat, fstatat64 as fstatat, readdir64 as readdir, stat64 as stat};

    /// How a directory on the way to a start is opened: to be searched and
    /// no more, where the system has such a way (as `rm`'s passages are).
    #[cfg(any(target_os = "linux", target_os = "android"))]
    const TO_SEARCH: libc::c_int = libc::O_PATH;
    #[cfg(any(target_vendor = "apple", target_os = "freebsd"))]
    const TO_SEARCH: libc::c_int = libc::O_SEARCH;
    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_vendor = "apple",
        target_os = "freebsd"
    )))]
    const TO_SEARCH: libc::c_int = libc::O_RDONLY;

    const TO_LIST: libc::c_int = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC;

    fn c_name(name: &OsStr) -> io::Result<CString> {
        CString::new(name.as_bytes())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
    }

    fn leaves_the_root() -> io::Error {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "it leads out of the directory it is read in",
        )
    }

    fn kind_of_mode(mode: libc::mode_t) -> Kind {
        match mode & libc::S_IFMT {
            libc::S_IFREG => Kind::File,
            libc::S_IFDIR => Kind::Directory,
            libc::S_IFLNK => Kind::Link,
            libc::S_IFIFO => Kind::Fifo,
            libc::S_IFSOCK => Kind::Socket,
            libc::S_IFCHR => Kind::CharDevice,
            libc::S_IFBLK => Kind::BlockDevice,
            _ => Kind::Other,
        }
    }

    #[allow(clippy::useless_conversion, clippy::unnecessary_cast)]
    fn looked(found: &stat) -> Looked {
        let seconds = found.st_mtime as i64;
        Looked {
            kind: kind_of_mode(found.st_mode as libc::mode_t),
            len: found.st_size as u64,
            mtime_nanos: i128::from(seconds) * 1_000_000_000
                + i128::from(found.st_mtime_nsec as i64),
            mtime_seconds: seconds,
            allocated: (found.st_blocks as u64).saturating_mul(512),
            identity: Some((found.st_dev as u64, found.st_ino as u64)),
            names: found.st_nlink as u64,
        }
    }

    /// The entry called `name` in the directory `fd` holds — itself, or
    /// with `follow`, what a link there points to.
    fn look_at(fd: RawFd, name: &CStr, follow: bool) -> io::Result<Looked> {
        let mut found = std::mem::MaybeUninit::<stat>::uninit();
        let flags = if follow { 0 } else { libc::AT_SYMLINK_NOFOLLOW };
        // SAFETY: `fd` is open for the length of the call, `name` is a
        // NUL-terminated string that outlives it, and `found` is a
        // writable record the call fills on success.
        if unsafe { fstatat(fd, name.as_ptr(), found.as_mut_ptr(), flags) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the call succeeded, so the record is filled in.
        Ok(looked(&unsafe { found.assume_init() }))
    }

    /// The entry at `path` itself, by its path: a root a walk is given, or
    /// a start with no last name — never one read in a root.
    fn look_path(path: &Path) -> io::Result<Looked> {
        look_at(libc::AT_FDCWD, &c_name(path.as_os_str())?, false)
    }

    fn look_held(fd: RawFd) -> io::Result<Looked> {
        let mut found = std::mem::MaybeUninit::<stat>::uninit();
        // SAFETY: as in `look_at`.
        if unsafe { fstat(fd, found.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the call succeeded.
        Ok(looked(&unsafe { found.assume_init() }))
    }

    fn opened(fd: RawFd) -> io::Result<OwnedFd> {
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: `fd` is an open descriptor nothing else owns.
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }

    fn open_path(path: &Path, flags: libc::c_int) -> io::Result<OwnedFd> {
        let path = c_name(path.as_os_str())?;
        // SAFETY: `path` is NUL-terminated and outlives the call.
        opened(unsafe { libc::open(path.as_ptr(), flags) })
    }

    fn open_at(fd: RawFd, name: &CStr, flags: libc::c_int) -> io::Result<OwnedFd> {
        // SAFETY: `fd` is open for the length of the call and `name` is a
        // NUL-terminated string that outlives it.
        opened(unsafe { libc::openat(fd, name.as_ptr(), flags) })
    }

    fn read_link_at(fd: RawFd, name: &CStr) -> io::Result<PathBuf> {
        let mut buffer = vec![0u8; 256];
        loop {
            // SAFETY: `buffer` is writable for its length; the call
            // writes at most that many bytes.
            let read = unsafe {
                libc::readlinkat(fd, name.as_ptr(), buffer.as_mut_ptr().cast(), buffer.len())
            };
            if read < 0 {
                return Err(io::Error::last_os_error());
            }
            let read = read as usize;
            if read < buffer.len() {
                buffer.truncate(read);
                return Ok(PathBuf::from(OsString::from_vec(buffer)));
            }
            buffer.resize(buffer.len() * 2, 0);
        }
    }

    /// The directory a CANONICAL path names — one with no link in it,
    /// lying in `root` — reached from `root` down, one name at a time,
    /// each in the one before and never through a link; searched, and
    /// opened to be listed with `to_list`. With no root, opened by its
    /// path.
    /// cr-uv10-1: the directory at an ABSOLUTE path with no link in it — the
    /// root a walk is given, resolved when it was configured — opened from
    /// `/` one name at a time, never through a link. A folder above it
    /// replaced by a link would have led the open, by the path, wherever
    /// the link points; here it is an error. A relative path is opened by
    /// its path.
    fn open_unlinked(path: &Path, last_flags: libc::c_int) -> io::Result<OwnedFd> {
        if !path.is_absolute() {
            return open_path(path, last_flags);
        }
        let names: Vec<&OsStr> = path
            .components()
            .filter_map(|component| match component {
                Component::Normal(name) => Some(Ok(name)),
                Component::RootDir => None,
                _ => Some(Err(io::Error::from(io::ErrorKind::InvalidInput))),
            })
            .collect::<io::Result<_>>()?;
        if names.is_empty() {
            return open_path(path, last_flags);
        }
        let mut held = open_path(
            Path::new("/"),
            TO_SEARCH | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )?;
        for (index, name) in names.iter().enumerate() {
            let flags = if index + 1 == names.len() {
                last_flags
            } else {
                TO_SEARCH | libc::O_DIRECTORY | libc::O_CLOEXEC
            };
            held = open_at(held.as_raw_fd(), &c_name(name)?, flags | libc::O_NOFOLLOW)?;
        }
        Ok(held)
    }

    /// cr-ecm23-3: the directory at an absolute path with no link in it,
    /// held — opened from `/` one name at a time, never through a link —
    /// for a file to be made and named in it.
    pub(crate) fn directory_unlinked(path: &Path) -> io::Result<OwnedFd> {
        open_unlinked(path, libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC)
    }

    /// cr-uv10-1: the entry at an absolute path with no link in it, itself:
    /// looked up in its directory, opened by [`open_unlinked`].
    fn look_unlinked(path: &Path) -> io::Result<Looked> {
        match (path.parent(), path.file_name()) {
            (Some(parent), Some(name)) if path.is_absolute() => {
                let held = open_unlinked(parent, TO_SEARCH | libc::O_DIRECTORY | libc::O_CLOEXEC)?;
                look_at(held.as_raw_fd(), &c_name(name)?, false)
            }
            _ => look_path(path),
        }
    }

    fn open_canonical(canonical: &Path, root: Option<&Path>, to_list: bool) -> io::Result<OwnedFd> {
        let last_flags = if to_list {
            TO_LIST
        } else {
            TO_SEARCH | libc::O_DIRECTORY | libc::O_CLOEXEC
        };
        let Some(root) = root else {
            before_entering(canonical);
            return open_path(canonical, last_flags);
        };
        let below = canonical
            .strip_prefix(root)
            .map_err(|_| leaves_the_root())?;
        let names: Vec<&OsStr> = below
            .components()
            .map(|component| match component {
                Component::Normal(name) => Ok(name),
                _ => Err(io::Error::from(io::ErrorKind::InvalidInput)),
            })
            .collect::<io::Result<_>>()?;
        if names.is_empty() {
            before_entering(root);
            return open_unlinked(root, last_flags);
        }
        let mut held = open_unlinked(root, TO_SEARCH | libc::O_DIRECTORY | libc::O_CLOEXEC)?;
        let mut at = root.to_path_buf();
        for (index, name) in names.iter().enumerate() {
            at.push(name);
            let flags = if index + 1 == names.len() {
                last_flags
            } else {
                TO_SEARCH | libc::O_DIRECTORY | libc::O_CLOEXEC
            };
            before_entering(&at);
            held = open_at(held.as_raw_fd(), &c_name(name)?, flags | libc::O_NOFOLLOW)?;
        }
        Ok(held)
    }

    /// One directory a walk reads: listed, looked into, and entered
    /// through what holds it.
    pub(crate) struct Directory {
        /// The listing stream; it owns the descriptor.
        stream: NonNull<libc::DIR>,
        ended: bool,
        /// Its path, by its names — said of it, and never opened.
        path: PathBuf,
        /// It is the root a walk may not leave: its `..` is itself.
        is_root: bool,
    }

    impl Directory {
        fn listing(fd: OwnedFd, path: PathBuf, is_root: bool) -> io::Result<Self> {
            let raw = std::os::unix::io::IntoRawFd::into_raw_fd(fd);
            // SAFETY: `raw` is an open directory descriptor this function
            // owns; on success the stream owns it, on failure it is closed.
            match NonNull::new(unsafe { libc::fdopendir(raw) }) {
                Some(stream) => Ok(Self {
                    stream,
                    ended: false,
                    path,
                    is_root,
                }),
                None => {
                    let error = io::Error::last_os_error();
                    // SAFETY: still this function's descriptor.
                    unsafe { libc::close(raw) };
                    Err(error)
                }
            }
        }

        fn fd(&self) -> RawFd {
            // SAFETY: the stream is open until this value is dropped.
            unsafe { libc::dirfd(self.stream.as_ptr()) }
        }

        /// Its next entry, `.` and `..` passed over. After the end, or an
        /// error — given once — nothing more.
        pub(crate) fn next_entry(&mut self) -> Option<io::Result<Listed>> {
            loop {
                if self.ended {
                    return None;
                }
                // SAFETY: setting this thread's errno, read just below to
                // tell the end of the listing from a failure.
                unsafe { *errno_location() = 0 };
                // SAFETY: the stream is open and only this call reads it.
                let entry = unsafe { readdir(self.stream.as_ptr()) };
                if entry.is_null() {
                    self.ended = true;
                    let error = io::Error::last_os_error();
                    return match error.raw_os_error() {
                        Some(code) if code != 0 => Some(Err(error)),
                        _ => None,
                    };
                }
                // SAFETY: a non-null entry is valid until the next read of
                // this stream; its name is NUL-terminated within it, and
                // is copied before that read.
                let (name, kind) = unsafe {
                    (
                        CStr::from_ptr(std::ptr::addr_of!((*entry).d_name).cast()).to_owned(),
                        (*entry).d_type,
                    )
                };
                if matches!(name.to_bytes(), b"." | b"..") {
                    continue;
                }
                let kind = match kind {
                    libc::DT_REG => Some(Kind::File),
                    libc::DT_DIR => Some(Kind::Directory),
                    libc::DT_LNK => Some(Kind::Link),
                    libc::DT_FIFO => Some(Kind::Fifo),
                    libc::DT_SOCK => Some(Kind::Socket),
                    libc::DT_CHR => Some(Kind::CharDevice),
                    libc::DT_BLK => Some(Kind::BlockDevice),
                    libc::DT_UNKNOWN => None,
                    _ => Some(Kind::Other),
                };
                return Some(Ok(Listed {
                    name: OsString::from_vec(name.as_bytes().to_vec()),
                    kind,
                    c_name: name,
                }));
            }
        }

        /// What `listed` is — the entry itself, a link being a link.
        pub(crate) fn look(&self, listed: &Listed) -> io::Result<Looked> {
            look_at(self.fd(), &listed.c_name, false)
        }

        /// The directory `listed` is, entered HERE, never through a link
        /// standing in its place.
        pub(crate) fn enter(&self, listed: &Listed) -> io::Result<Directory> {
            let path = self.path.join(&listed.name);
            before_entering(&path);
            let fd = open_at(self.fd(), &listed.c_name, TO_LIST | libc::O_NOFOLLOW)?;
            Directory::listing(fd, path, false)
        }

        /// Where the link `listed` points, as it holds it.
        pub(crate) fn link_target(&self, listed: &Listed) -> io::Result<PathBuf> {
            read_link_at(self.fd(), &listed.c_name)
        }

        /// The identity of a file of several names: what the look says.
        pub(crate) fn several_names(
            &self,
            _listed: &Listed,
            looked: &Looked,
        ) -> Option<(u64, u64)> {
            several(looked)
        }

        /// This directory itself.
        pub(crate) fn itself(&self) -> io::Result<Looked> {
            look_held(self.fd())
        }

        /// The directory above it — itself, for the root a walk may not
        /// leave.
        pub(crate) fn above(&self) -> io::Result<Looked> {
            if self.is_root {
                return self.itself();
            }
            look_at(self.fd(), c"..", true)
        }
    }

    impl Drop for Directory {
        fn drop(&mut self) {
            // SAFETY: the stream is open and is not used after this.
            unsafe { libc::closedir(self.stream.as_ptr()) };
        }
    }

    fn several(looked: &Looked) -> Option<(u64, u64)> {
        (!looked.is_dir() && looked.names > 1)
            .then_some(looked.identity)
            .flatten()
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe fn errno_location() -> *mut libc::c_int {
        libc::__errno_location()
    }
    #[cfg(any(target_vendor = "apple", target_os = "freebsd"))]
    unsafe fn errno_location() -> *mut libc::c_int {
        libc::__error()
    }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_vendor = "apple",
        target_os = "freebsd"
    )))]
    unsafe fn errno_location() -> *mut libc::c_int {
        libc::__errno()
    }

    /// An entry a directory listed.
    pub(crate) struct Listed {
        pub(super) name: OsString,
        pub(super) kind: Option<Kind>,
        c_name: CString,
    }

    /// Where a start's own entry is.
    enum Place {
        /// In this directory, held to be searched, by this name.
        In(OwnedFd, CString),
        /// A directory read whole — the root, or a path with no last name
        /// (`/`, `.`): its canonical path.
        Whole(PathBuf),
    }

    /// A walk's start, as typed.
    pub(crate) struct Start {
        /// Its names alone: no separator or `.` after it.
        named: PathBuf,
        /// Typed for the system to look through a link at its last name.
        through: bool,
        root: Option<PathBuf>,
        place: Place,
    }

    impl Start {
        /// `start` as typed, read in `root` and never out of it — the
        /// directory its entry is in held, reached from `root` one name at
        /// a time. An error is the start's own: it cannot be looked up.
        pub(crate) fn at(start: &Path, root: Option<&Path>) -> io::Result<Self> {
            let named: PathBuf = start.components().collect();
            let through = typed_through(start);
            // Where it lies is judged by the one way in: `open_canonical`,
            // from the root down, refuses a path that is not below it.
            let place = match (named.parent(), named.file_name()) {
                _ if root.is_some_and(|root| named == root) => {
                    Place::Whole(root.expect("checked").to_path_buf())
                }
                (Some(parent), Some(name)) => {
                    let parent = if parent.as_os_str().is_empty() {
                        Path::new(".")
                    } else {
                        parent
                    };
                    let canonical = std::fs::canonicalize(parent)?;
                    Place::In(open_canonical(&canonical, root, false)?, c_name(name)?)
                }
                _ => Place::Whole(std::fs::canonicalize(&named)?),
            };
            Ok(Self {
                named,
                through,
                root: root.map(Path::to_path_buf),
                place,
            })
        }

        /// Where what the start names lies, through every link — judged
        /// in the root by what then opens it ([`open_canonical`]).
        fn canonical(&self) -> io::Result<PathBuf> {
            std::fs::canonicalize(&self.named)
        }

        /// A directory read whole (`Place::Whole`), itself: the root by its
        /// path — a module may be one file — and anything else from the
        /// root down.
        fn look_whole(&self, path: &Path) -> io::Result<Looked> {
            match self.root.as_deref() {
                Some(root) if path != root => {
                    look_held(open_canonical(path, Some(root), false)?.as_raw_fd())
                }
                // cr-uv10-1: the root itself, never through a link above it.
                Some(_) => look_unlinked(path),
                None => look_path(path),
            }
        }

        /// The start as typed: its own entry — or, typed with a separator
        /// after it, the directory a link there leads to (what is no
        /// directory is `Not a directory`).
        pub(crate) fn look(&self) -> io::Result<Looked> {
            if self.through {
                let held = open_canonical(&self.canonical()?, self.root.as_deref(), false)?;
                return look_held(held.as_raw_fd());
            }
            match &self.place {
                Place::In(dir, name) => look_at(dir.as_raw_fd(), name, false),
                Place::Whole(path) => self.look_whole(path),
            }
        }

        /// What a link at the start leads to.
        pub(crate) fn look_through(&self) -> io::Result<Looked> {
            let canonical = self.canonical()?;
            if self.root.as_deref() == Some(canonical.as_path()) {
                return self.look_whole(&canonical);
            }
            match (canonical.parent(), canonical.file_name()) {
                (Some(parent), Some(name)) => {
                    let held = open_canonical(parent, self.root.as_deref(), false)?;
                    look_at(held.as_raw_fd(), &c_name(name)?, false)
                }
                _ => self.look_whole(&canonical),
            }
        }

        /// The start, read as a directory as typed.
        pub(crate) fn enter(&self) -> io::Result<Directory> {
            match (&self.place, self.through) {
                (Place::In(dir, name), false) => {
                    before_entering(&self.named);
                    let fd = open_at(dir.as_raw_fd(), name, TO_LIST | libc::O_NOFOLLOW)?;
                    Directory::listing(fd, self.named.clone(), false)
                }
                _ => self.enter_through(),
            }
        }

        /// The directory a link at the start leads to.
        pub(crate) fn enter_through(&self) -> io::Result<Directory> {
            let canonical = match &self.place {
                Place::Whole(path) if !self.through => path.clone(),
                _ => self.canonical()?,
            };
            let is_root = self.root.as_deref() == Some(canonical.as_path());
            let fd = open_canonical(&canonical, self.root.as_deref(), true)?;
            Directory::listing(fd, self.named.clone(), is_root)
        }

        /// Where a link at the start points, as it holds it.
        pub(crate) fn link_target(&self) -> io::Result<PathBuf> {
            match &self.place {
                Place::In(dir, name) => read_link_at(dir.as_raw_fd(), name),
                Place::Whole(path) => std::fs::read_link(path),
            }
        }

        /// The identity of a start that is a file of several names.
        pub(crate) fn several_names(&self, looked: &Looked) -> Option<(u64, u64)> {
            several(looked)
        }

        /// cr-uv10-2: where the start's own entry is — the directory it is
        /// in, held, and which file `looked` says it is; nothing for a
        /// start read whole.
        pub(crate) fn seen(&self, looked: &Looked) -> Option<((u64, u64), (u64, u64))> {
            match &self.place {
                Place::In(dir, _) => look_held(dir.as_raw_fd())
                    .ok()
                    .and_then(|within| within.identity)
                    .zip(looked.identity),
                Place::Whole(_) => None,
            }
        }
    }
}

#[cfg(not(unix))]
mod imp {
    use super::{Kind, Looked};
    use std::ffi::OsString;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};

    /// One directory a walk reads, by its path.
    pub(crate) struct Directory {
        path: PathBuf,
        entries: fs::ReadDir,
        root: Option<PathBuf>,
    }

    /// An entry a directory listed.
    pub(crate) struct Listed {
        pub(super) name: OsString,
        pub(super) kind: Option<Kind>,
        entry: fs::DirEntry,
    }

    impl Directory {
        fn read(path: &Path, root: Option<&Path>) -> io::Result<Self> {
            super::before_entering(path);
            Ok(Self {
                path: path.to_path_buf(),
                entries: fs::read_dir(path)?,
                root: root.map(Path::to_path_buf),
            })
        }

        pub(crate) fn next_entry(&mut self) -> Option<io::Result<Listed>> {
            let entry = match self.entries.next()? {
                Ok(entry) => entry,
                Err(error) => return Some(Err(error)),
            };
            let kind = entry.file_type().ok().map(Kind::of);
            Some(Ok(Listed {
                name: entry.file_name(),
                kind,
                entry,
            }))
        }

        pub(crate) fn look(&self, listed: &Listed) -> io::Result<Looked> {
            listed
                .entry
                .metadata()
                .map(|metadata| Looked::of(&metadata))
        }

        pub(crate) fn enter(&self, listed: &Listed) -> io::Result<Directory> {
            Self::read(&listed.entry.path(), self.root.as_deref())
        }

        pub(crate) fn link_target(&self, listed: &Listed) -> io::Result<PathBuf> {
            fs::read_link(listed.entry.path())
        }

        pub(crate) fn several_names(&self, listed: &Listed, looked: &Looked) -> Option<(u64, u64)> {
            several_names_at(&listed.entry.path(), looked)
        }

        pub(crate) fn itself(&self) -> io::Result<Looked> {
            fs::metadata(&self.path).map(|metadata| Looked::of(&metadata))
        }

        /// The directory above this one. Above the root — the whole of what
        /// the asker may see — there is nothing: its `..` is itself.
        pub(crate) fn above(&self) -> io::Result<Looked> {
            let canonical = fs::canonicalize(&self.path)?;
            let above = if self.root.as_deref() == Some(canonical.as_path()) {
                canonical
            } else {
                match canonical.parent() {
                    Some(parent) => parent.to_path_buf(),
                    None => canonical,
                }
            };
            fs::metadata(above).map(|metadata| Looked::of(&metadata))
        }
    }

    fn several_names_at(path: &Path, looked: &Looked) -> Option<(u64, u64)> {
        if looked.is_dir() {
            return None;
        }
        // The directory listing does not say how many names a file has;
        // the file has to be opened to learn it (and its identity).
        let (identity, names) = crate::transfers::filter::entry_identity(path)?;
        (names > 1).then_some(identity)
    }

    /// A walk's start, by its path as typed.
    pub(crate) struct Start {
        typed: PathBuf,
        root: Option<PathBuf>,
    }

    impl Start {
        pub(crate) fn at(start: &Path, root: Option<&Path>) -> io::Result<Self> {
            Ok(Self {
                typed: start.to_path_buf(),
                root: root.map(Path::to_path_buf),
            })
        }

        pub(crate) fn look(&self) -> io::Result<Looked> {
            fs::symlink_metadata(&self.typed).map(|metadata| Looked::of(&metadata))
        }

        pub(crate) fn look_through(&self) -> io::Result<Looked> {
            fs::metadata(&self.typed).map(|metadata| Looked::of(&metadata))
        }

        pub(crate) fn enter(&self) -> io::Result<Directory> {
            Directory::read(&self.typed, self.root.as_deref())
        }

        pub(crate) fn enter_through(&self) -> io::Result<Directory> {
            Directory::read(&self.typed, self.root.as_deref())
        }

        pub(crate) fn link_target(&self) -> io::Result<PathBuf> {
            fs::read_link(&self.typed)
        }

        pub(crate) fn several_names(&self, looked: &Looked) -> Option<(u64, u64)> {
            several_names_at(&self.typed, looked)
        }

        pub(crate) fn seen(&self, _looked: &Looked) -> Option<((u64, u64), (u64, u64))> {
            None
        }
    }
}
