# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.1](https://github.com/ccbrown/iocraft/compare/iocraft-v0.6.0...iocraft-v0.6.1) - 2025-01-08

### Fixed

- check if stdin is terminal (#59)

## [0.6.0](https://github.com/ccbrown/iocraft/compare/iocraft-v0.5.3...iocraft-v0.6.0) - 2024-12-30

### Added

- [**breaking**] rename `Box` to `View` to avoid conflict (#56)

## [0.5.3](https://github.com/ccbrown/iocraft/compare/iocraft-v0.5.2...iocraft-v0.5.3) - 2024-12-28

### Fixed

- eliminate undesired impact of transparent components on layout (#53)

## [0.5.2](https://github.com/ccbrown/iocraft/compare/iocraft-v0.5.1...iocraft-v0.5.2) - 2024-12-25

### Added

- add try_ methods to State, document/reduce panics (#49)

### Fixed

- improve component recycling algorithm (#51)

### Other

- add notes to State::try_ methods

## [0.5.1](https://github.com/ccbrown/iocraft/compare/iocraft-v0.5.0...iocraft-v0.5.1) - 2024-12-20

### Added

- add write function to State (#45)

### Fixed

- rename extend function to avoid std conflicts (#46)

### Other

- rust 1.83 clippy fixes (#47)
- use core instead of std where possible

## [0.5.0](https://github.com/ccbrown/iocraft/compare/iocraft-v0.4.1...iocraft-v0.5.0) - 2024-12-10

### Added

- make async functions send+sync (#38)

## [0.4.1](https://github.com/ccbrown/iocraft/compare/iocraft-v0.4.0...iocraft-v0.4.1) - 2024-12-05

### Added

- enable "std" feature for taffy ([#35](https://github.com/ccbrown/iocraft/pull/35))

## [0.4.0](https://github.com/ccbrown/iocraft/compare/iocraft-v0.3.2...iocraft-v0.4.0) - 2024-11-01

### Other

- fix minor typo

## [0.3.2](https://github.com/ccbrown/iocraft/compare/iocraft-v0.3.1...iocraft-v0.3.2) - 2024-10-04

### Added

- add button component

## [0.3.1](https://github.com/ccbrown/iocraft/compare/iocraft-v0.3.0...iocraft-v0.3.1) - 2024-09-30

### Added

- improve state api so that deadlocks are harder to create, add docs

## [0.3.0](https://github.com/ccbrown/iocraft/compare/iocraft-v0.2.3...iocraft-v0.3.0) - 2024-09-30

### Added

- convenience methods for creating terminal event types
- fullscreen mouse events, calculator example

### Other

- seal hooks, update docs, rm deprecated fn

## [0.2.3](https://github.com/ccbrown/iocraft/compare/iocraft-v0.2.2...iocraft-v0.2.3) - 2024-09-27

### Added

- add position, inset, and gap style props
- allow margins to be negative

### Other

- add test for negative margin

## [0.2.2](https://github.com/ccbrown/iocraft/compare/iocraft-v0.2.1...iocraft-v0.2.2) - 2024-09-26

### Fixed

- explicitly check for keyboard enhancement support before enabling
- make emoji with vs16 space correctly on more platforms

## [0.2.1](https://github.com/ccbrown/iocraft/compare/iocraft-v0.2.0...iocraft-v0.2.1) - 2024-09-26

### Other

- add windows to ci ([#20](https://github.com/ccbrown/iocraft/pull/20))
- use std::io::IsTerminal
- eliminate use of examples symlinks

## [0.2.0](https://github.com/ccbrown/iocraft/compare/iocraft-v0.1.2...iocraft-v0.2.0) - 2024-09-25

### Added

- add use_async_handler hook
- add mock_terminal_render_loop api

### Other

- add more docs, ratatui shoutout, and non_exhaustive attrs

## [0.1.2](https://github.com/ccbrown/iocraft/compare/iocraft-v0.1.1...iocraft-v0.1.2) - 2024-09-24

### Other

- add a few more tests
- expand documentation, add many more doc examples

## [0.1.1](https://github.com/ccbrown/iocraft/compare/iocraft-v0.1.0...iocraft-v0.1.1) - 2024-09-23

### Other

- doc improvements, add example images
- release ([#10](https://github.com/ccbrown/iocraft/pull/10))

## [0.1.0](https://github.com/ccbrown/iocraft/releases/tag/iocraft-v0.1.0) - 2024-09-23

### Fixed

- fix crate dependencies for examples
- fix doc include path resolution

### Other

- explicitly specify iocraft-macros version
- release ([#9](https://github.com/ccbrown/iocraft/pull/9))
- add package descriptions, repositories, and readmes
- key prop, docs, and tests
- documentation pass
- minor simplification
- props docs
- add docs
- add more tests
- improve test coverage
- add fullscreen example
- use_context
- refactor hook logic out of macro
- use_state
- redo hooks mechanism
- use_async refactor, spawn method
- minor refactor
- refactor terminal a bit for testability
- unicode fixes
- add form example
- rename render -> draw
- add a few more tests ([#8](https://github.com/ccbrown/iocraft/pull/8))
- add lots o tests ([#7](https://github.com/ccbrown/iocraft/pull/7))
- add ci ([#1](https://github.com/ccbrown/iocraft/pull/1))
- text underline
- text alignment
- text wrapping
- rm mouse events, avoid problematic cursor saving/restoring
- add mouse events
- typo fix
- handle emoji/different unicode character widths correctly
- rename UseFuture -> UseAsync
- tweaks to input handling
- complete first pass at docs for all public types
- do a pass at about half the docs
- clean up public api
- add license, fix up exports
- more powerful context, mutable props/context
- simplify
- simplify
- input handling and example
- eliminate render loop flickering
- system context
- small refactors, add progress bar example
- refactor stdio hooks
- polish
- cleanup
- non-static content provider props (but not value yet)
- non-static props!
- rm one more send
- less send
- simplify
- way less cloning
- context
- make handles clone
- use_stdout, use_stderr
- simplify
- tests, fix edge cases
- add tests
- convenience/Display functions, add tests
- pretty table
- text weight
- canvas rendering
- finish table example
- iterate on style support, start table example
- rename project

## [0.1.0](https://github.com/ccbrown/iocraft/releases/tag/iocraft-v0.1.0) - 2024-09-23

### Fixed

- fix crate dependencies for examples
- fix doc include path resolution

### Other

- add package descriptions, repositories, and readmes
- key prop, docs, and tests
- documentation pass
- minor simplification
- props docs
- add docs
- add more tests
- improve test coverage
- add fullscreen example
- use_context
- refactor hook logic out of macro
- use_state
- redo hooks mechanism
- use_async refactor, spawn method
- minor refactor
- refactor terminal a bit for testability
- unicode fixes
- add form example
- rename render -> draw
- add a few more tests ([#8](https://github.com/ccbrown/iocraft/pull/8))
- add lots o tests ([#7](https://github.com/ccbrown/iocraft/pull/7))
- add ci ([#1](https://github.com/ccbrown/iocraft/pull/1))
- text underline
- text alignment
- text wrapping
- rm mouse events, avoid problematic cursor saving/restoring
- add mouse events
- typo fix
- handle emoji/different unicode character widths correctly
- rename UseFuture -> UseAsync
- tweaks to input handling
- complete first pass at docs for all public types
- do a pass at about half the docs
- clean up public api
- add license, fix up exports
- more powerful context, mutable props/context
- simplify
- simplify
- input handling and example
- eliminate render loop flickering
- system context
- small refactors, add progress bar example
- refactor stdio hooks
- polish
- cleanup
- non-static content provider props (but not value yet)
- non-static props!
- rm one more send
- less send
- simplify
- way less cloning
- context
- make handles clone
- use_stdout, use_stderr
- simplify
- tests, fix edge cases
- add tests
- convenience/Display functions, add tests
- pretty table
- text weight
- canvas rendering
- finish table example
- iterate on style support, start table example
- rename project
