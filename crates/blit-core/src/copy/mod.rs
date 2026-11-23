mod compare;
mod file_copy;
#[cfg(windows)]
mod windows;

pub use compare::{
    file_needs_copy, file_needs_copy_with_checksum_type, file_needs_copy_with_mode,
    file_needs_copy_with_mode_opened,
};
#[cfg(unix)]
pub(crate) use file_copy::resume::{effective_user, is_link, resolve_destination};
#[cfg(all(test, unix))]
pub(crate) use file_copy::resume::{
    AFTER_A_RESOLVE_HOOKS, BEFORE_A_FRESH_PARTIAL_IS_MADE_HOOKS, RUN_AS,
};
pub use file_copy::resume::{DEFAULT_BLOCK_SIZE, MAX_BLOCK_SIZE};
pub use file_copy::{
    copy_file, copy_opened, copy_opened_into, mmap_copy_file, open_resume_partial,
    open_resume_partial_held, resume_copy_file, resume_copy_from, resume_copy_into,
    resume_copy_through, ResumeCopyOutcome,
};
pub use file_copy::{create_regular, open_regular, open_to_read};
#[cfg(unix)]
pub use file_copy::{fresh_partial, fresh_partial_ready, make_partial_private};
#[cfg(windows)]
pub use windows::windows_copyfile;
