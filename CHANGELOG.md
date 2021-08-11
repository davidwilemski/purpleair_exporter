# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2021-08-11
### Added
- Add instantaneous PM2.5 AQI estimate.

## [0.2.0] - 2021-08-10
### Added
- Support comma-separated list of sensor IDs in PURPLEAIR_SENSOR_IDS.

### Changed
- Use Rust log and env_logger crates.
- Add a changelog.

## [0.1.0] - 2021-08-10
### Changed
Initial release

### Added 
- Support scraping Purple Air JSON API for sensor IDs (based on `PURPLEAIR_SENSOR_IDS` environment variable). Supports a single ID or a pipe-separated list (e.g. "123|456").
- Exports Prometheus metrics on port 3000 at `/metrics`

[Unreleased]: git@github.com:davidwilemski/purpleair_exporter/compare/0.3.0...HEAD
[0.3.0]: git@github.com:davidwilemski/purpleair_exporter/compare/0.2.0...0.3.0
[0.2.0]: git@github.com:davidwilemski/purpleair_exporter/compare/0.1.0...0.2.0
[0.1.0]: git@github.com:davidwilemski/purpleair_exporter/releases/tag/0.1.0
