use anyhow::{Context as _, bail};
use gix::bstr::ByteSlice as _;
use gix::refs::transaction::PreviousValue;
use gix::{ObjectId, Repository};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;

// FIXME: if gix is only used here, replace with command use instead
pub fn clone_repo(ssh_url: &str, out_path: impl AsRef<Path>) -> anyhow::Result<gix::Repository> {
    let should_interrupt = AtomicBool::new(false);

    let mut prepare = gix::prepare_clone(ssh_url, out_path.as_ref())?;

    let (mut checkout, _fetch_outcome) =
        prepare.fetch_then_checkout(gix::progress::Discard, &should_interrupt)?;

    let (repo, _checkout_outcome) =
        checkout.main_worktree(gix::progress::Discard, &should_interrupt)?;

    Ok(repo)
}

pub fn fetch_main(repo_path: &Path) -> anyhow::Result<()> {
    let output = Command::new("git")
        .args(["fetch", "origin"])
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git fetch failed: {stderr}");
    }
    Ok(())
}

/// Create a worktree from main at `worktree_path` with a unique branch name.
/// Returns a `WorktreeGuard` that removes the worktree on drop.
pub fn create_worktree(
    repo_path: &Path,
    worktree_path: &Path,
    branch: &str,
) -> anyhow::Result<WorktreeGuard> {
    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            branch,
            worktree_path.to_str().context("non-UTF8 path")?,
            "origin/main",
        ])
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git worktree add failed: {stderr}");
    }
    Ok(WorktreeGuard {
        repo_path: repo_path.to_owned(),
        worktree_path: worktree_path.to_owned(),
        branch: branch.to_owned(),
    })
}

pub struct WorktreeGuard {
    repo_path: PathBuf,
    worktree_path: PathBuf,
    branch: String,
}

impl Drop for WorktreeGuard {
    fn drop(&mut self) {
        let _ = Command::new("git")
            .args([
                "worktree",
                "remove",
                "--force",
                self.worktree_path.to_str().unwrap_or(""),
            ])
            .current_dir(&self.repo_path)
            .output();
        let _ = Command::new("git")
            .args(["branch", "-D", &self.branch])
            .current_dir(&self.repo_path)
            .output();
    }
}

pub fn push_branch(repo_path: &Path, branch: &str) -> anyhow::Result<()> {
    let output = Command::new("git")
        .args(["push", "origin", branch])
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git push failed: {stderr}");
    }
    Ok(())
}

/// Apply a `git format-patch` patch using `git am`.
/// Unlike `git apply`, `git am` creates a commit and handles patch metadata.
pub fn apply_patch(work_dir: &Path, patch: &str) -> anyhow::Result<()> {
    let normalized = patch.replace("\r\n", "\n");
    let patch = normalized.as_str();
    let mut child = Command::new("git")
        .args(["am", "--3way", "-"])
        .current_dir(work_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    child
        .stdin
        .as_mut()
        .context("failed to open stdin for git am")?
        .write_all(patch.as_bytes())?;

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let _ = Command::new("git")
            .args(["am", "--abort"])
            .current_dir(work_dir)
            .output();
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git am failed: {stderr}")
    }
    Ok(())
}

/// Rewrite the author of the current HEAD commit and move the branch to the new commit.
/// Analogous to `git commit --amend --author="name <email>" --no-edit`.
///
/// `new_name` and `new_email` are optional — fields left as `None` are preserved
/// from the original commit. Passing both as `None` is a no-op that still produces
/// an identical new commit (same SHA), so the branch move is effectively skipped.
///
/// Returns the new commit id.
pub fn amend<F>(
    repo: &Repository,
    new_name: Option<&str>,
    new_email: Option<&str>,
    commit_message_transform: F,
) -> anyhow::Result<ObjectId>
where
    F: for<'a> FnOnce(&'a str) -> String,
{
    // Resolve HEAD → commit.
    let head = repo.head()?;
    let old_id = head
        .id()
        .context("HEAD is unborn (no commits yet)")?
        .detach();
    let original = repo.find_object(old_id)?.try_into_commit()?;

    // Owned, mutable copy.
    let mut commit: gix::objs::Commit = original
        .decode()
        .context("Failed to decode commit")?
        .to_owned()?;

    if let Some(name) = new_name {
        commit.author.name = name.into();
        commit.committer.name = name.into();
    }
    if let Some(email) = new_email {
        commit.author.email = email.into();
        commit.committer.email = email.into();
    }

    let old_msg = commit.message.to_str_lossy();
    commit.message = commit_message_transform(&old_msg).into();

    // Write the new commit object.
    let new_id = repo.write_object(&commit)?.detach();

    // Move whatever HEAD points at (a branch, or HEAD itself if detached).
    let log_message = {
        let name = new_name.unwrap_or("<unchanged>");
        let email = new_email.unwrap_or("<unchanged>");
        format!("amend: rewrite author to {name} <{email}>")
    };

    match repo.head()?.referent_name() {
        Some(branch_ref) => {
            repo.reference(
                branch_ref.as_bstr().to_str()?,
                new_id,
                PreviousValue::MustExistAndMatch(old_id.into()),
                log_message,
            )?;
        }
        None => {
            repo.reference(
                "HEAD",
                new_id,
                PreviousValue::MustExistAndMatch(old_id.into()),
                log_message,
            )?;
        }
    }

    Ok(new_id)
}
