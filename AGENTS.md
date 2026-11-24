# Guidance for AI agents

Read [CONTRIBUTING.md](CONTRIBUTING.md) before making changes. In particular:

- **Never edit `CHANGELOG.md` files.** They're generated from commit messages at release time. Manual entries are redundant and cause merge conflicts.
- **Be very conservative with public doc comments.** Most bug fixes need no doc changes. Don't add sentences describing the fix, the absence of the bug, or implementation details (locks, borrows, internal helpers, etc.). Only document behavior that a user of the crate can observe and needs to know about. Put notes for maintainers in regular `//` comments, and put summaries of your work in the commit message or pull request description.
