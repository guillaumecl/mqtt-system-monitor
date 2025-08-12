# Changelog

<!-- next-header -->

## [Unreleased] - ReleaseDate

### Breaking changes

- The temperature ID is used instead of the label

## [1.2.0] - 2025-08-09

### Added

- Allow the registration of several network interfaces sensors.

### Fixed

- The systemd service now wants default.target to be able to be used on user sessions
- Fix availability sensor connectivity class

## [1.1.1] - 2025-08-09

### Added

- Availability sensor, which tells Home Assistant when the device is connected/disconnected.
- The entity ID can now have spaces or uppercase letters and is converted in snake case

## [1.1.0] - 2025-08-07

### Added

- Documentation for the library
- Memory usage sensor

### Fixed

- Allow cleanup when interrupting with Ctrl-C or SIGINT. This will allow sending end messages later

## [1.0.5] - 2025-08-05

### Added

- Changelog system using cargo release

### Fixed

- Only report component devices that are actually configured.


## [1.0.3] - 2025-08-03

Initial version


<!-- next-url -->
[Unreleased]: https://github.com/guillaumecl/mqtt-system-monitor/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/guillaumecl/mqtt-system-monitor/compare/v1.1.1...v1.2.0
[1.1.1]: https://github.com/guillaumecl/mqtt-system-monitor/compare/v1.1.0...v1.1.1
[1.1.0]: https://github.com/guillaumecl/mqtt-system-monitor/compare/v1.0.5...v1.1.0
[1.0.5]: https://github.com/guillaumecl/mqtt-system-monitor/compare/v1.0.3...v1.0.5
[1.0.3]: https://github.com/guillaumecl/mqtt-system-monitor/releases/tag/v1.0.3
