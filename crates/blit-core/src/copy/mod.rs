mod compare;
mod file_copy;
#[cfg(windows)]
mod windows;

pub use compare::{
    file_needs_copy, file_needs_copy_with_checksum_type, file_needs_copy_with_mode,
    file_needs_copy_with_mode_opened,
};
#[cfg(unix)]
pub(crate) use file_copy::resume::effective_user;
#[cfg(all(test, unix))]
pub(crate) use file_copy::resume::RUN_AS;
pub use file_copy::resume::{DEFAULT_BLOCK_SIZE, MAX_BLOCK_SIZE};
pub use file_copy::{
    copy_file, copy_opened, copy_opened_into, mmap_copy_file, open_resume_partial,
    resume_copy_file, resume_copy_from, resume_copy_into, ResumeCopyOutcome,
};
#[cfg(unix)]
pub use file_copy::{fresh_partial, fresh_partial_ready, make_partial_private};
#[cfg(windows)]
pub use windows::windows_copyfile;
