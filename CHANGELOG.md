# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.1] - 2024/12/07

### Fixes

* Repair broken unit tests (issue #2)

## [1.0.0] - 2024/09/08

### New Features

* Support for LZW compression
* Support for Teledisk 1.x (issue #1)
* Emit a warning about truncation policy
* Compression options are public
* Error is returned for unreasonably large files, controlled by options

### Fixes

* lib writes to log instead of stderr
* CI tests TD0 transformations directly

### Breaking Changes

* `crate::Options` and `crate::STD_OPTIONS` are removed, use module scoped options instead