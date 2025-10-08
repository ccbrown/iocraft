# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.14](https://github.com/ccbrown/iocraft/compare/iocraft-v0.7.13...iocraft-v0.7.14) - 2025-10-08

### Fixed

- avoid bg color overflowing at eol ([#143](https://github.com/ccbrown/iocraft/pull/143))
- End synchronized update on StdTerminal drop ([#140](https://github.com/ccbrown/iocraft/pull/140))

## [0.7.13](https://github.com/ccbrown/iocraft/compare/iocraft-v0.7.12...iocraft-v0.7.13) - 2025-09-28

### Added

- use_ref, use_effect, and imperative TextInput control ([#136](https://github.com/ccbrown/iocraft/pull/136))
- additional state convenience methods

### Fixed

- underflow under certain absolute positioning circumstances ([#138](https://github.com/ccbrown/iocraft/pull/138))

## [0.7.12](https://github.com/ccbrown/iocraft/compare/iocraft-v0.7.11...iocraft-v0.7.12) - 2025-09-20

### Fixed

- purge terminal on vertical overflow ([#134](https://github.com/ccbrown/iocraft/pull/134))
- make TextInput ignore modified keys ([#132](https://github.com/ccbrown/iocraft/pull/132))

## [0.7.11](https://github.com/ccbrown/iocraft/compare/iocraft-v0.7.10...iocraft-v0.7.11) - 2025-08-20

### Added

- automatically append newline as needed for use_output ([#124](https://github.com/ccbrown/iocraft/pull/124))
- add `print` methods for stdout without newlines ([#122](https://github.com/ccbrown/iocraft/pull/122))

## [0.7.10](https://github.com/ccbrown/iocraft/compare/iocraft-v0.7.9...iocraft-v0.7.10) - 2025-06-20

### Fixed

- TextInput initial value scroll offset ([#105](https://github.com/ccbrown/iocraft/pull/105))

## [0.7.9](https://github.com/ccbrown/iocraft/compare/iocraft-v0.7.8...iocraft-v0.7.9) - 2025-05-07

### Fixed

- add gnome to list of bad vs16 terminals ([#101](https://github.com/ccbrown/iocraft/pull/101))

## [0.7.8](https://github.com/ccbrown/iocraft/compare/iocraft-v0.7.7...iocraft-v0.7.8) - 2025-04-29

### Added

- add fragment component and use_const hook ([#98](https://github.com/ccbrown/iocraft/pull/98))

## [0.7.7](https://github.com/ccbrown/iocraft/compare/iocraft-v0.7.6...iocraft-v0.7.7) - 2025-04-25

### Fixed

- don't let multiline input scroll horizontally ([#96](https://github.com/ccbrown/iocraft/pull/96))

### Other

- rewrite text input, add cursor and multiline support ([#92](https://github.com/ccbrown/iocraft/pull/92))
- implement text wrapping to be more robust for advanced cases ([#95](https://github.com/ccbrown/iocraft/pull/95))
- fix doc typo
- add UseMemo hook ([#93](https://github.com/ccbrown/iocraft/pull/93))

## [0.7.6](https://github.com/ccbrown/iocraft/compare/iocraft-v0.7.5...iocraft-v0.7.6) - 2025-04-04

### Other

- fix UseAsyncHandler docs typo

## [0.7.5](https://github.com/ccbrown/iocraft/compare/iocraft-v0.7.4...iocraft-v0.7.5) - 2025-04-03

### Fixed

- allow use_terminal_events handlers to mutate

### Other

- lint fix

## [0.7.4](https://github.com/ccbrown/iocraft/compare/iocraft-v0.7.3...iocraft-v0.7.4) - 2025-03-27

### Fixed

- don't erase last col for fullscreen tuis ([#84](https://github.com/ccbrown/iocraft/pull/84))

## [0.7.3](https://github.com/ccbrown/iocraft/compare/iocraft-v0.7.2...iocraft-v0.7.3) - 2025-03-26

### Added

- add italic text ([#82](https://github.com/ccbrown/iocraft/pull/82))

### Fixed

- don't underline leading whitespace center/right aligned text ([#81](https://github.com/ccbrown/iocraft/pull/81))

### Other

- add MixedText component ([#79](https://github.com/ccbrown/iocraft/pull/79))

## [0.7.2](https://github.com/ccbrown/iocraft/compare/iocraft-v0.7.1...iocraft-v0.7.2) - 2025-03-20

### Fixed

- don't error if keyboard enhancement check times out

## [0.7.1](https://github.com/ccbrown/iocraft/compare/iocraft-v0.7.0...iocraft-v0.7.1) - 2025-03-18

### Other

- Fix for overflow when scrolling out of bounds. ([#72](https://github.com/ccbrown/iocraft/pull/72))

## [0.7.0](https://github.com/ccbrown/iocraft/compare/iocraft-v0.6.4...iocraft-v0.7.0) - 2025-03-15

### Added

- fully implement overflow property, add scrolling example ([#70](https://github.com/ccbrown/iocraft/pull/70))

### Fixed

- negative top/left positions

### Other

- polish up docs regarding key prop ([#66](https://github.com/ccbrown/iocraft/pull/66))

## [0.6.4](https://github.com/ccbrown/iocraft/compare/iocraft-v0.6.3...iocraft-v0.6.4) - 2025-02-19

### Other

- re-arrange element macro docs to work around docs.rs bug

## [0.6.3](https://github.com/ccbrown/iocraft/compare/iocraft-v0.6.2...iocraft-v0.6.3) - 2025-02-19

### Fixed

- make properties named "key" a compile-time error

## [0.6.2](https://github.com/ccbrown/iocraft/compare/iocraft-v0.6.1...iocraft-v0.6.2) - 2025-01-20

### Fixed

- move reset to before the final newline

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
