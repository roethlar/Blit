mod compare;
mod file_copy;
#[cfg(windows)]
mod windows;

pub use compare::{file_needs_copy, file_needs_copy_with_checksum_type, file_needs_copy_with_mode};
pub use file_copy::resume::{DEFAULT_BLOCK_SIZE, MAX_BLOCK_SIZE};
pub use file_copy::{
    chunked_copy_file, copy_file, mmap_copy_file, resume_copy_file, ResumeCopyOutcome,
};
#[cfg(windows)]
pub use windows::windows_copyfile;
