use std::path::Path;

use crate::atomic;

pub struct MutationTransaction<'a> {
    repo: &'a Path,
    backups: Vec<(String, String)>,
    needs_unstage: bool,
    active: bool,
}

impl<'a> MutationTransaction<'a> {
    pub fn new(repo: &'a Path) -> Self {
        Self {
            repo,
            backups: Vec::new(),
            needs_unstage: false,
            active: true,
        }
    }

    pub fn record_backup(&mut self, path: String, original: String) {
        self.backups.push((path, original));
    }

    pub fn mark_staged(&mut self) {
        self.needs_unstage = true;
    }

    pub fn commit(mut self) {
        self.active = false;
    }
}

impl<'a> Drop for MutationTransaction<'a> {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        for (path, original) in self.backups.iter().rev() {
            let _ = atomic::write_atomic(path, original);
        }
        if self.needs_unstage {
            unstage(self.repo);
        }
    }
}

pub fn unstage(repo: &Path) {
    let _ = std::process::Command::new("git")
        .current_dir(repo)
        .args(["reset", "HEAD", "--quiet"])
        .status();
}
