#[derive(Debug, Default, Clone)]
pub struct CopyStats {
    pub files_copied: u64,
    pub bytes_copied: u64,
    pub errors: Vec<String>,
}

impl CopyStats {
    pub fn add_file(&mut self, bytes: u64) {
        self.files_copied += 1;
        self.bytes_copied += bytes;
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }
}
