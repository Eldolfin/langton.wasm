# Contributing to langton.wasm — AI Contributor Workflow

> **⚠️ Work in progress notice**
> The submission URL below points to `localhost` and the shell commands reference a feature branch.
> This is because the submission feature is still being developed and has not yet merged to `main`.
> This notice will be removed before the feature is released.

You are helping a user contribute a bugfix, new feature, or new animation to **langton.wasm** — a parametrized [Langton's Ant](https://en.wikipedia.org/wiki/Langton%27s_ant) simulator written in Rust, compiled to WebAssembly, and running entirely in the browser.

Follow every step below **in order**. Do not skip or reorder steps.

---

## Step 1 — Understand the request

Ask the user the following questions (in a single message):

1. **What do you want to change or add?** Describe in as much detail as you can: a bug you noticed, a visual effect, a new parameter, a new ant rule set, a performance improvement, etc.
2. **What type of contribution is this?**
   - Bug fix
   - New feature or enhancement to an existing one
   - New animation / rule set
   - Other (describe)

Do not proceed until you have a clear initial description.

---

## Step 2 — Set up the repository

Clone the GitHub mirror (always from `main`; the GitHub mirror is used to avoid potential access restrictions on Codeberg):

```bash
git clone https://github.com/Eldolfin/langton.wasm.git --branch main --depth 1
cd langton.wasm
```

**Read the project knowledge base before anything else:**

```bash
cat AGENTS.md
```

`AGENTS.md` contains the full project structure, code map, file locations, conventions, anti-patterns, and build commands. You must understand it fully before reading any other file.

---

## Step 3 — Eliminate all ambiguity

Before writing any code, go through the user's request and identify every decision that is not yet pinned down. Ask all remaining clarifying questions **in a single message**. Do not ask one question at a time.

Typical questions depending on the contribution type — ask whichever apply:

**For any change:**
- Which part of the simulation is affected (ant movement, canvas rendering, debug UI, URL params)?
- Should the change be visible only in debug mode (`?debug` in the URL) or always?

**For new parameters:**
- What is the parameter name (will become a `snake_case` URL param and a debug UI slider label)?
- What is the default value, minimum, and maximum?
- Should the slider use a linear or logarithmic scale?
- Does changing this parameter require restarting the animation (`needs_restart: true`)?

**For new rule sets / ant behaviors:**
- What are the exact turn rules (e.g. which color → turn left, which → turn right)?
- How many colors/states are involved?
- Should it be selectable from the presets dropdown?

**For visual changes:**
- What are the exact colors, sizes, or style values?
- Should it degrade gracefully when the canvas is very small or very large?

**For bug fixes:**
- How do you reproduce the bug (steps, URL params that trigger it)?
- What is the expected behavior vs. what actually happens?

Wait for all answers before proceeding.

---

## Step 4 — Contributor identity

Ask the user (in the same message as Step 3 clarifications if there are any, or separately if not):

> Would you like to be credited as a contributor in the patch commit?
>
> - **Name** — will appear in the commit author or co-author line *(optional)*
> - **Email** — used as the commit author email *(optional, and kept private if you prefer)*
>
> Both fields are optional. You can skip this entirely if you prefer to stay anonymous.

Record the answer for Step 8.

---

## Step 5 — Plan the implementation

Before touching any file:

1. List every source file you need to read based on the task.
2. Read them all (`cat crates/langton/src/lib.rs`, `cat crates/debug_ui/src/lib.rs`, etc.).
3. Write out your full implementation plan:
   - Which functions or structs change, and how
   - What new code is added and where exactly (file + approximate location)
   - Whether any CSS changes are needed (goes in the crate's `src/style.css`)
   - Whether a new URL param is added (name it, state its default)
   - Whether `needs_restart: true` is required
   - Whether the canvas rendering pipeline (`crates/canvas/src/lib.rs`) is touched

Present this plan to the user. Ask for explicit confirmation before writing any code. If the user has corrections, revise the plan and confirm again.

---

## Step 6 — Implement

Follow these rules without exception — violations cause CI to reject the patch:

| Rule | Detail |
|------|--------|
| **Zero clippy warnings** | CI runs `cargo clippy --workspace -- -Dwarnings`. One warning = build failure. |
| **No `unsafe`** | The entire codebase is safe Rust. Keep it that way. |
| **Monolithic `lib.rs`** | No new submodules. All code for a crate goes in its existing `lib.rs`. |
| **CSS in `style.css`** | Each crate embeds its styles via `include_str!("./style.css")`. Edit that file. |
| **`snake_case` URL params** | e.g. `number_of_ants`, `cell_size`, `alpha_retention`. |
| **Edition 2024** | Use current Rust idioms. |
| **No comments on obvious code** | Only add a comment when the *why* is non-obvious. No docstrings. |
| **`.unwrap()` on DOM ops is fine** | `console_error_panic_hook` is installed; panics surface in the browser console. |
| **No `allow` attributes** | Do not suppress clippy or compiler warnings with `#[allow(...)]`. Fix them. |

Implement the full change. Do not leave `TODO` comments or placeholder code.

---

## Step 7 — Verify

Run the full verification suite:

```bash
cargo fmt --check
cargo clippy --workspace -- -Dwarnings
cargo test --workspace --verbose
```

If you have no shell access, do the following manually for every changed file:

1. **Format**: Check that indentation is 4 spaces, no trailing whitespace, opening braces on the same line, consistent with surrounding code.
2. **Clippy**: Look for unused imports, unused variables, `let _ =` where the type implements `must_use`, needless borrows, and match expressions that could be `if let`.
3. **Tests**: Read every `#[test]` and `#[rstest]` case in the changed crates. Confirm that your changes do not break any of them. If you added logic that should be tested, add tests.

Fix all issues before generating the patch.

---

## Step 8 — Commit and produce the JSON payload

Write a commit message following [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/):
- `feat: add X` for new features
- `fix: correct Y` for bug fixes
- `refactor: ...` for internal restructuring with no behavior change
- `chore: ...` for build/tooling changes

Set commit authorship based on what the user provided in Step 4:

| User provided | `--author` field | Commit body |
|---|---|---|
| Name **and** email | `User Name <user@email.com>` | `Co-Authored-By: Claude <noreply@anthropic.com>` |
| Name only | `User Name <unknown@noreply.example>` | `Co-Authored-By: Claude <noreply@anthropic.com>` |
| Nothing | `Claude <noreply@anthropic.com>` | *(no extra lines)* |

Commit, then build a JSON payload containing the patch and PR metadata. Use `jq` if available, otherwise fall back to `python3`:

```bash
git add -A
git commit --author="Name <email>" -m "feat: short description

Longer explanation if needed.

Co-Authored-By: Claude <noreply@anthropic.com>"

# Variables — set these to match your commit
PR_TITLE="feat: short description"
PR_BODY="$(cat <<'EOF'
## Summary
- ...

## Changes
- ...
EOF
)"
PATCH="$(git format-patch HEAD~1 --stdout)"

# Build JSON (jq preferred; python3 fallback; or write it out manually)
if command -v jq >/dev/null 2>&1; then
  jq -n \
    --arg patch   "$PATCH" \
    --arg title   "$PR_TITLE" \
    --arg body    "$PR_BODY" \
    '{"patch": $patch, "pr-title": $title, "pr-body": $body}'
elif command -v python3 >/dev/null 2>&1; then
  python3 -c "
import json, sys
print(json.dumps({'patch': sys.argv[1], 'pr-title': sys.argv[2], 'pr-body': sys.argv[3]}))" \
    "$PATCH" "$PR_TITLE" "$PR_BODY"
else
  # No tools available — construct the JSON manually.
  # Escape the patch: replace \ with \\, " with \", newlines with \n, then wrap.
  # You can write the JSON by hand in the fenced block below if needed.
  echo "No jq or python3 found — write the JSON manually (see instructions below)"
fi
```

If neither `jq` nor `python3` is available, write the JSON manually in the fenced code block. The format is:

```
{"patch":"<patch content with \\n for newlines and \\" for quotes>","pr-title":"<title>","pr-body":"<body with \\n for newlines>"}
```

Escape rules: every `\` → `\\`, every `"` → `\"`, every newline → `\n`. The result must be valid JSON on a single line.

Present the **complete JSON output** in a single fenced code block using the `json` language tag:

````
```json
{"patch":"From 0000000000000000000000000000000000000000 Mon Sep 17 00:00:00 2001\n...","pr-title":"feat: ...","pr-body":"## Summary\n..."}
```
````

The block must be complete and self-contained — the user will copy-paste it directly.

---

## Step 9 — Submission

After presenting the JSON payload, end your response with this block. Make it the last thing in your message, formatted exactly as shown so it cannot be missed:

---

## ✅ Payload ready — submit it now

> **⚠️ Work in progress:** The submission endpoint below runs on `localhost` because this feature is still being developed. This notice will be removed once the feature merges to `main`.

1. **Copy the entire JSON block** above.
2. **Open this URL in your browser:**

### 👉 [http://localhost:3000/submit](http://localhost:3000/submit)

3. Paste the JSON into the **"JSON payload"** field and follow the instructions on the page.

The page will handle the rest of the submission process.

---
