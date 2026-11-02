# Welcome to the iocraft contributing guide!

Thanks for your interest in contributing to the project!

Pull requests for bug fixes and feature contributions are greatly appreciated!

## If you've found a bug that you'd like to fix...

Please search the issues first to see if it's a known issue or if there's any relevant discussion.

If the fix would require a large time investment or would have a significant impact on the crate's API, please use an existing or new issue to propose your fix before you begin.

## If you'd like to implement a new feature...

Again, please search the issues first to see if there's any relevant discussion.

If the feature is minor or quick to implement, you can feel free to open a pull request right away. If the feature is more invasive or requires significant investment, start a discussion about it via an issue so we can make sure we're aligned on the right direction to take.

## Changelogs

Don't edit `CHANGELOG.md` files. They're generated automatically from commit messages when a release is made, so manual additions are redundant and cause merge conflicts with other pull requests. Instead, use a [Conventional Commits](https://www.conventionalcommits.org/) style title for your pull request (e.g. `fix(text-input): ...` or `feat: ...`), as it will become the changelog entry.

## Documentation

Doc comments (`///` and `//!`) on public items are the crate's user-facing documentation on [docs.rs](https://docs.rs/iocraft). They should describe how to use the API, not how it's implemented or how it came to be. Before adding to them, ask whether a typical user of the crate needs to know what you're writing:

- **Do** document behavior a user can observe and might reasonably not expect, such as panics, errors, clamping of out-of-range input, or new options.
- **Don't** describe the bug you fixed or reassure readers that it no longer happens. Correct behavior is the default assumption; "leading whitespace is included in the width" or "wide characters are only included once" just narrates a fix.
- **Don't** mention internal details such as locks, borrows, wakers, internal writers, or helper functions. If a maintainer needs to know about it, use a regular `//` comment next to the code instead.
- **Don't** restate what the type signature already says, such as trait bounds.

Most bug fixes require no documentation changes at all. Your commit message and pull request description are the right places to explain what you fixed and why.

## Thank you!

❤️❤️❤️
