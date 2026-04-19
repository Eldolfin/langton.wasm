# Apply Submitted Patch — Server-Side Agent Workflow

You are a server-side AI agent with full shell access. A user has submitted a git patch through the contribution form. Your job is to apply it cleanly onto `main`, resolve any conflicts, verify it passes all checks, and produce a clean final patch ready to be proposed as a pull request.

---

## Step 1 — Set up the environment

```bash
git clone https://github.com/Eldolfin/langton.wasm.git --branch main
cd langton.wasm
```

Read the project knowledge base:

```bash
cat AGENTS.md
```

---

## Step 2 — Apply the patch

Save the received patch content to `incoming.patch`, then apply it with 3-way merge support:

```bash
git am --3way incoming.patch
```

**If `git am` succeeds with no conflicts**, skip to Step 3.

**If `git am` fails with conflicts:**

1. Inspect what failed:
   ```bash
   git am --show-current-patch
   git diff
   ```
2. Read the conflicting files in full. Understand what the patch intended vs. what `main` currently contains.
3. Resolve each conflict, preserving the intent of the submitted patch. When the patch intent and the current `main` state are incompatible, prefer correctness over mechanical application — adapt the change to fit the current code.
4. Stage resolved files and continue:
   ```bash
   git add <resolved-files>
   git am --continue
   ```
5. If a conflict cannot be resolved without missing context, abort and output a clear error:
   ```bash
   git am --abort
   ```
   Then explain exactly which file and hunk failed, what the conflict is, and what information would be needed to resolve it.

---

## Step 3 — Verify

Run the full check suite:

```bash
cargo fmt --check
cargo clippy --workspace -- -Dwarnings
cargo test --workspace --verbose
```

For each failure:

| Failure | Action |
|---------|--------|
| `cargo fmt --check` fails | Run `cargo fmt`, then `git commit --amend --no-edit` |
| `cargo clippy` warning | Fix the warning in the source, then `git commit --amend --no-edit` |
| `cargo test` fails | Determine if the failure pre-exists on `main` (run `git stash`, test, `git stash pop`). If caused by the patch, fix it and amend. If pre-existing, note it but do not block the patch. |

Do not use `#[allow(...)]` to suppress clippy warnings. Fix them properly.

---

## Step 4 — Output the final patch

```bash
git format-patch HEAD~1 --stdout
```

Output the complete result. This patch is clean, verified, and ready to be opened as a pull request.
