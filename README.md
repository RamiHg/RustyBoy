# RustyBoy

RustyBoy is a cycle-accurate Gameboy emulator written to be as close to the real hardware as possible.

While it does not yet have all the bells and whistles of a user-friendly emulator, it has enough features to 

The emulator is designed to mimic the environment of FPGA development. It is therefore not written for speed or
efficiency (but still runs pretty fast).

## Getting Started

RustyBoy works on any target supported by Rust. The audio backend works on Linux, Mac OS X, and Windows.

### Prerequisites

cmake is required in all platforms to build the 3rd-party audio backend (libsoundio). Install using your favorite package manager: `sudo apt/brew/scoop install cmake`.

#### Linux

pkg-config and PulseAudio are required for audio playback on Linux. To install
(on Debian-based systems):

```sh
sudo apt install pkg-config pulseaudio
```

### Usage

```bash
cargo run --release -- path_to_rom.gb
```

## [1.1.0] What's New

Full [changelog here](Changelog.md).

Audio now works! Tested on Mac OS X and Windows. There are still some minor issues regarding some
unimplemented features, but all-in-all, it works quite well. libsamplerate is used to accurately
downsample the audio to 48kHz.

If you are running into issues getting the audio dependencies to compile, simply remove "audio" from
the default feature set in soc/Cargo.toml.

Fixed a pretty gnarly bug regarding interrupt servicing.



RustyBoy currently only supports MBC1 and MBC3 cartridges.

## Implementation

### CPU

RustyBoy's CPU is based on a microcode specification that is completely written in a
[Google Sheets document](https://docs.google.com/spreadsheets/d/1kMCDI1IlQtenE8m_Q8PhgFFZF4yk-pS-j0KrA0_e-DM/edit).
The document describes each opcode's t-cycle execution at the micro-code level.

The [microcode csv](soc/instructions.csv) is then read by the [asm compiler](soc/src/cpu/asm), which
verifies and compiles the microcode into its [final structure](soc/src/cpu/micro_code.rs). This
microcode is what is actually used **both** by the Rust emulator, **and** the FPGA CPU.

This has the interesting side-effect that the [CPU control unit](soc/src/cpu/control_unit.rs) is
relatively simple - almost 260 lines. Most of the heavy lifting is in the data!

### GPU

Most demoscene and video game ROMs that fully utilize the GPU rely on behavior that is accurate to
the T (no pun intended). Having a perfectly accurate CPU without an equally accurate GPU is like
installing a race car engine in the body of a Lada - it's just not going to look very impressive.

Unfortunately, most GPU behavior is undocumented. There are even slight edge-case differences
between different revisions of the same model.

But due to recent [heroic efforts](https://www.youtube.com/watch?v=HyzD8pNlpwI) by researchers, I
was able to put together [something](soc/src/gpu.rs) that is fairly accurate. Watching that video
should give you a good understanding of about 90% of the GPU's inner workings.

## Status

At this point, I consider the emulator to be feature-complete (except audio). It's (almost)
perfectly cycle accurate; at least in the areas that I care about.

It passes all Blargh, all (but one) MooneyeGB, and almost all Wilbert Pol tests. See the
[complete test status](docs/test_details.md) for a more detailed list of all passing/failing tests.

Of course, passing unit tests is all fine and dandy. The real fun is being able to run demo-scene
ROMs (and video games). oh.gb and pocket.gb run almost flawlessly. Here is a montage of my favorite
parts:

<p align="center">
    <image src="docs/rustyboy.gif" />
</p>

## License

RustyBoy is currently released as GPLv3, because I see no reason why anyone would derive from it. If
you'd like a more permissive license, email me!
