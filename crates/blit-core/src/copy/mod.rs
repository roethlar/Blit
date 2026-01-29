mod compare;
mod file_copy;
mod parallel;
mod stats;
#[cfg(windows)]
mod windows;

pub use compare::{file_needs_copy, file_needs_copy_with_checksum_type};
pub use file_copy::{chunked_copy_file, copy_file, mmap_copy_file, resume_copy_file, ResumeCopyOutcome};
pub use file_copy::resume::{DEFAULT_BLOCK_SIZE, MAX_BLOCK_SIZE};
pub use parallel::parallel_copy_files;
pub use stats::CopyStats;
#[cfg(windows)]
pub use windows::windows_copyfile;
