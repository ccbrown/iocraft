# Copilot review instructions

Follow [CONTRIBUTING.md](../CONTRIBUTING.md) and [AGENTS.md](../AGENTS.md). When reviewing pull requests, enforce the rules below.

## Blocking issues

Violations of these rules are **blocking**. Do not present them as nits, suggestions, optional, or "consider". State plainly that the change must be fixed before the pull request can be merged, and comment on every occurrence, not just the first.

### Edits to `CHANGELOG.md`

Any change to a `CHANGELOG.md` file outside of the automated release pull requests is blocking. Changelogs are generated from commit messages at release time.

### Doc comments that don't belong

Doc comments (`///` and `//!`) are user-facing documentation on docs.rs. Flag any added or modified doc comment that:

- Describes a bug fix, or reassures the reader that a bug no longer happens. Correct behavior is assumed.
- Mentions implementation details such as locks, borrows, wakers, internal writers, private types, or helper functions.
- Restates what the signature already says, such as trait bounds or parameter types.
- Contains maintainer notes, including `XXX:` or `TODO:` notes. These belong in regular `//` comments.

Examples of doc comment lines that were previously removed for these reasons:

- `/// A wide character is included once; its continuation columns do not introduce spaces into the extracted text.`
- `/// Leading Unicode whitespace contributes to the aligned display width.`
- `/// Bold and light replace each other rather than combining terminal attributes.`
- `/// Property structs may use defaulted type and const generic parameters.`

Acceptable doc additions describe behavior a user can observe and might not expect: panics, errors, clamping of out-of-range input, or new options. Most bug fixes need no doc changes at all. When flagging, suggest deleting the line or moving maintainer-relevant content to a `//` comment, and point out that explanations of the fix belong in the pull request description.

### Comments that narrate the change

Flag `//` comments that only make sense in the context of this pull request, such as:

- Describing what was changed or fixed (e.g. "now uses...", "fixed so that...", "previously...", "instead of...").
- Referencing the pull request, the review, an issue discussion, or instructions given to an AI agent.
- Restating what the next line of code obviously does.

Comments should explain non-obvious intent or pitfalls for future readers of the code as it stands. `// XXX:` comments for important pitfalls are encouraged.

## Non-blocking guidance

- Bug fixes should usually come with a regression test. If one is missing, ask for it.
- Don't comment on formatting, clippy lints, or broken intra-doc links. CI enforces `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo doc` with `-D warnings`.
- Keep new code consistent with the surrounding code's naming, idioms, and comment density.
