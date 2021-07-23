# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0]

### Added
- `analysis_end` function on System to replace `system__wide_hyper_period` as the end of analysis, 
  this takes the largest offset into account
- `fixed_` functions for actual execution to use aggregated higher priority actual execution instead of aggregated higher
  priority constrained demand, the original behaviour is preserved under the `original_` prefix

### Changed
- renamed some functions to contain the prefix `original_` 

### Fixed 
- Readme copy paste error

## [0.1.1]

### Fixed 
- Readme formatting and links

## [0.1.0]

### Added
- Initial Implementation


[Unreleased]: https://git.informatik.uni-kiel.de/stu201758/rta-for-fps-rs/-/tree/master
[0.1.0]: https://github.com/Skgland/Response-Time-Analysis-for-Fixed-Priority-Servers/tree/v0.1.0
[0.1.1]: https://github.com/Skgland/Response-Time-Analysis-for-Fixed-Priority-Servers/tree/v0.1.1
[0.2.0]: https://github.com/Skgland/Response-Time-Analysis-for-Fixed-Priority-Servers/tree/v0.2.0