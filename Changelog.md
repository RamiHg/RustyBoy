# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2019-07-12

### Added

- Audio now works! Tested on Mac OS X and Windows. There are still some minor issues regarding some
  unimplemented features, but all-in-all, it works quite well. libsamplerate is used to accurately
  downsample the audio to 48kHz.
- Serializing now correctly serializes the cartridge, as well as its RAM contents.

### Changed

- Bincode instead of JSON for the serialization format. Orders of magnitude faster (duh).

### Fixed

- Fixed major bug with interrupt handling and CB-prefix instructions. Games were randomly crashing
  for no apparent reason. After many nights of debugging, I realized that I was servicing interrupts
  in between CB opcode decoding (due to how CB instructions are implemented)! Needless to say, games
  run flawlessly now.

## [1.0.0] - 2019-06-17

Initial release!
