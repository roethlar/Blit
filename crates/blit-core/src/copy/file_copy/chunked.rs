use crate::buffer::BufferSizer;
use crate::logger::Logger;
use eyre::Result;
use std::fs;
use std::io::{self, BufReader, BufWriter};
use std::path::Path;

#[cfg(windows)]
use super::metadata::preserve_metadata;
#[cfg(windows)]
use crate::copy::windows;

pub fn chunked_copy_file(
    src: &Path,
    dst: &Path,
    buffer_sizer: &BufferSizer,
    is_network: bool,
    progress: Option<&indicatif::ProgressBar>,
    logger: &dyn Logger,
) -> Result<u64> {
    logger.start(src, dst);

    #[cfg(windows)]
    if !is_network {
        if let Ok(bytes) = windows::windows_copyfile(src, dst) {
            preserve_metadata(src, dst)?;
            logger.copy_done(src, dst, bytes);
            return Ok(bytes);
        }
    }
    let result: Result<u64> = (|| {
        let metadata = fs::metadata(src)?;
        let file_size = metadata.len();

        let chunk_size = if file_size > 1_073_741_824 {
            16 * 1024 * 1024
        } else {
            buffer_sizer.calculate_buffer_size(file_size, is_network)
        };

        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut reader = BufReader::with_capacity(chunk_size, std::fs::File::open(src)?);
        let mut writer = BufWriter::with_capacity(chunk_size, std::fs::File::create(dst)?);
        let total_bytes = io::copy(&mut reader, &mut writer)?;
        if let Some(pb) = progress {
            pb.set_position(total_bytes);
        }

        #[cfg(windows)]
        preserve_metadata(src, dst)?;

        Ok(total_bytes)
    })();

    match result {
        Ok(bytes) => {
            logger.copy_done(src, dst, bytes);
            Ok(bytes)
        }
        Err(e) => {
            logger.error("chunked_copy", src, &e.to_string());
            Err(e)
        }
    }
}
