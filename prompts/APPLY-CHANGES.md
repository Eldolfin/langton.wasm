# Apply Submitted Patch — Server-Side Agent Workflow

You are a server-side AI agent with full shell access. A user submitted a contribution to langton.wasm, but the patch could not be applied cleanly — there are merge conflicts that need resolving. You are already inside the repository with the failed `git am` state active.

The user's original contribution request was:

<<ORIGINAL_REQUEST>>

Use this to understand the *intent* of the patch when resolving conflicts. When in doubt about how to resolve a conflict, choose the interpretation that best fulfills the original request.

---

## Step 1 — Assess the situation

```bash
git am --show-current-patch
git status
git diff
```

Read the conflicting files in full. For each conflict marker (`<<<<<<<` / `=======` / `>>>>>>>`), understand:
- What the patch was trying to change (the `>>>>>>>` side)
- What `main` currently has (the `=======` / `<<<<<<<` side)
- Why they conflict (e.g. the surrounding code changed since the patch was generated)

Also read `AGENTS.md` if you need project context to make a correct resolution:

```bash
cat AGENTS.md
```

---

## Step 2 — Resolve conflicts

For each conflicting file:

1. Edit the file to resolve the conflict, preserving the intent of the submitted patch while adapting it to the current state of `main`.
2. Do not simply accept one side mechanically — if the conflicting region requires rewriting the patch logic to fit the new context, do so.
3. Stage the resolved file:
   ```bash
   git add <file>
   ```

Once all conflicts are resolved:

```bash
git am --continue
```

If a conflict is genuinely unresolvable without information that is not available (neither in the patch, the original request, nor the codebase), abort and report:

```bash
git am --abort
```

Output a clear explanation: which file and hunk failed, what the conflict is, and what is needed to resolve it.

---

## Step 3 — Verify

```bash
cargo fmt --check
cargo clippy --workspace -- -Dwarnings
cargo test --workspace --verbose
```

| Failure | Action |
|---------|--------|
| `cargo fmt --check` fails | Run `cargo fmt`, then `git commit --amend --no-edit` |
| `cargo clippy` warning | Fix the warning in the source, then `git commit --amend --no-edit` |
| `cargo test` fails | Check if the failure pre-exists on `main` without the patch. If caused by the patch, fix it and amend. If pre-existing, note it but do not block. |

Do not use `#[allow(...)]` to suppress warnings. Fix them properly.

---

## Step 4 — Output the final patch

```bash
git format-patch HEAD~1 --stdout
```

Output the complete result. This patch is clean, conflict-free, verified, and ready to be proposed as a pull request.
