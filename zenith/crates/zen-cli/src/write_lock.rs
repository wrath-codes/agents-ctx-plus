use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

const LOCK_WAIT_TIMEOUT: Duration = Duration::from_secs(300);
const LOCK_RETRY_DELAY: Duration = Duration::from_millis(250);

pub struct WriteLockGuard {
    path: PathBuf,
}

impl Drop for WriteLockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub async fn acquire_for_project(project_root: &Path) -> anyhow::Result<WriteLockGuard> {
    let lock_path = project_root.join(".zenith").join("lake.write.lock");
    let started = std::time::Instant::now();

    loop {
        match try_acquire(&lock_path) {
            Ok(guard) => return Ok(guard),
            Err(LockState::HeldBy(pid)) => {
                if started.elapsed() >= LOCK_WAIT_TIMEOUT {
                    anyhow::bail!(
                        "another write operation is running (pid {pid}); try again after it finishes"
                    );
                }
                tokio::time::sleep(LOCK_RETRY_DELAY).await;
            }
            Err(LockState::Stale) => {
                let _ = std::fs::remove_file(&lock_path);
            }
            Err(LockState::Unknown) => {
                if started.elapsed() >= LOCK_WAIT_TIMEOUT {
                    anyhow::bail!(
                        "could not acquire write lock at {}; remove stale lock file if no znt process is running",
                        lock_path.display()
                    );
                }
                tokio::time::sleep(LOCK_RETRY_DELAY).await;
            }
        }
    }
}

#[derive(Debug)]
enum LockState {
    HeldBy(i32),
    Stale,
    Unknown,
}

fn try_acquire(lock_path: &Path) -> Result<WriteLockGuard, LockState> {
    if let Some(parent) = lock_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    match OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(lock_path)
    {
        Ok(mut file) => {
            let pid = std::process::id();
            let _ = writeln!(file, "{pid}");
            Ok(WriteLockGuard {
                path: lock_path.to_path_buf(),
            })
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            let mut pid_buf = String::new();
            if OpenOptions::new()
                .read(true)
                .open(lock_path)
                .and_then(|mut file| file.read_to_string(&mut pid_buf))
                .is_err()
            {
                return Err(LockState::Unknown);
            }

            let pid = pid_buf.trim().parse::<i32>().ok();
            match pid {
                Some(pid) if is_process_running(pid) => Err(LockState::HeldBy(pid)),
                Some(_) => Err(LockState::Stale),
                None => Err(LockState::Unknown),
            }
        }
        Err(_) => Err(LockState::Unknown),
    }
}

fn is_process_running(pid: i32) -> bool {
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::try_acquire;

    #[test]
    fn acquires_and_releases_lock_file() {
        let temp = tempfile::tempdir().expect("tempdir should create");
        let lock_path = temp.path().join(".zenith/lake.write.lock");

        let guard = try_acquire(&lock_path).expect("lock should acquire");
        assert!(lock_path.is_file());
        drop(guard);
        assert!(!lock_path.exists());
    }
}
