# Changelog

All notable changes to Olenro will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Olenro is based on CC Switch (MIT). The full upstream history prior to the fork
is preserved in [CHANGELOG.upstream.md](./CHANGELOG.upstream.md).

## [1.0.0] - 2026-06-01

First official release of Olenro as an independent project, based on CC Switch
(MIT, upstream v3.15.0). Inherits the full CC Switch 3.15.0 feature set. See
[docs/release-notes/v1.0.0-en.md](docs/release-notes/v1.0.0-en.md) for details.

### Changed
- Rebranded from CC Switch fork to Olenro.
- Made the application identity fully independent: bundle identifier
  `com.olenro.desktop`, configuration directory `~/.olenro`, and deep-link
  scheme `olenro://`.
- Neutralized affiliate/referral tracking parameters in provider preset URLs.
- Switched auto-updater endpoint to the project's own release channel
  (github.com/lianhr12/olenro).

### Added
- One-time automatic import from a legacy `~/.cc-switch` configuration on first
  launch, so existing CC Switch users migrate without manual steps.

### Compatibility
- Legacy `ccswitch://` deep links are still parsed alongside the new `olenro://`
  scheme.
