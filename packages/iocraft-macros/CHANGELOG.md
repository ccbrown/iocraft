# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.3](https://github.com/ccbrown/iocraft/compare/iocraft-macros-v0.2.2...iocraft-macros-v0.2.3) - 2025-03-15

### Added

- fully implement overflow property, add scrolling example ([#70](https://github.com/ccbrown/iocraft/pull/70))

### Other

- polish up docs regarding key prop ([#66](https://github.com/ccbrown/iocraft/pull/66))

## [0.2.2](https://github.com/ccbrown/iocraft/compare/iocraft-macros-v0.2.1...iocraft-macros-v0.2.2) - 2025-02-19

### Other

- re-arrange element macro docs to work around docs.rs bug

## [0.2.1](https://github.com/ccbrown/iocraft/compare/iocraft-macros-v0.2.0...iocraft-macros-v0.2.1) - 2025-02-19

### Fixed

- make properties named "key" a compile-time error

## [0.2.0](https://github.com/ccbrown/iocraft/compare/iocraft-macros-v0.1.8...iocraft-macros-v0.2.0) - 2024-12-30

### Added

- [**breaking**] rename `Box` to `View` to avoid conflict (#56)

## [0.1.8](https://github.com/ccbrown/iocraft/compare/iocraft-macros-v0.1.7...iocraft-macros-v0.1.8) - 2024-12-10

### Added

- make async functions send+sync (#38)

## [0.1.7](https://github.com/ccbrown/iocraft/compare/iocraft-macros-v0.1.6...iocraft-macros-v0.1.7) - 2024-11-01

### Added

- support generic type parameters for component macro ([#33](https://github.com/ccbrown/iocraft/pull/33))

### Fixed

- keep components send + sync

## [0.1.6](https://github.com/ccbrown/iocraft/compare/iocraft-macros-v0.1.5...iocraft-macros-v0.1.6) - 2024-10-04

### Added

- add button component

## [0.1.5](https://github.com/ccbrown/iocraft/compare/iocraft-macros-v0.1.4...iocraft-macros-v0.1.5) - 2024-09-27

### Added

- add position, inset, and gap style props

## [0.1.4](https://github.com/ccbrown/iocraft/compare/iocraft-macros-v0.1.3...iocraft-macros-v0.1.4) - 2024-09-26

### Other

- eliminate use of examples symlinks

## [0.1.3](https://github.com/ccbrown/iocraft/compare/iocraft-macros-v0.1.2...iocraft-macros-v0.1.3) - 2024-09-25

### Added

- add use_async_handler hook
- add mock_terminal_render_loop api

## [0.1.2](https://github.com/ccbrown/iocraft/compare/iocraft-macros-v0.1.1...iocraft-macros-v0.1.2) - 2024-09-24

### Other

- add a few more tests

## [0.1.1](https://github.com/ccbrown/iocraft/compare/iocraft-macros-v0.1.0...iocraft-macros-v0.1.1) - 2024-09-23

### Other

- release ([#9](https://github.com/ccbrown/iocraft/pull/9))

## [0.1.0](https://github.com/ccbrown/iocraft/releases/tag/iocraft-macros-v0.1.0) - 2024-09-23

### Fixed

- fix crate dependencies for examples
- fix doc include path resolution

### Other

- add package descriptions, repositories, and readmes
- key prop, docs, and tests
- documentation pass
- props docs
- improve test coverage
- use_context
- simplify
- refactor hook logic out of macro
- use_state
- redo hooks mechanism
- add form example
- rename render -> draw
- add a few more tests ([#8](https://github.com/ccbrown/iocraft/pull/8))
- add lots o tests ([#7](https://github.com/ccbrown/iocraft/pull/7))
- use codecov ([#4](https://github.com/ccbrown/iocraft/pull/4))
- add ci ([#1](https://github.com/ccbrown/iocraft/pull/1))
- add mouse events
- complete first pass at docs for all public types
- clean up public api
- add license, fix up exports
- more powerful context, mutable props/context
- input handling and example
- eliminate render loop flickering
- system context
- small refactors, add progress bar example
- polish
- non-static props!
- context option
- better error
- way less cloning
- context
- simplify
- tests, fix edge cases
- text weight
- finish table example
- iterate on style support, start table example
- rename project
