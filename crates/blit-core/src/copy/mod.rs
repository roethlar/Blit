mod compare;
mod file_copy;
#[cfg(windows)]
mod windows;

pub use compare::{
    file_needs_copy, file_needs_copy_with_checksum_type, file_needs_copy_with_mode,
    file_needs_copy_with_mode_opened,
};
pub use file_copy::resume::{DEFAULT_BLOCK_SIZE, MAX_BLOCK_SIZE};
pub use file_copy::{
    copy_file, copy_opened, mmap_copy_file, resume_copy_file, resume_copy_from, ResumeCopyOutcome,
};
#[cfg(windows)]
pub use windows::windows_copyfile;
