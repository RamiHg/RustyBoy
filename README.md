# RustyBoy

RustyBoy is a cycle-accurate Gameboy emulator. It is designed to be a design guide and verification
tool in the development of a complete Gameboy system in an FPGA.

The emulator is designed to mimick the environment of FPGA development. It's not written for speed
or efficiency; it barely runs at 2x speed even with full optimizations!

## Usage

```bash
cargo run --release -- path_to_rom.gb
```

RustyBoy currently only supports MBC1 and MBC3 cartidges.

## Implementation

RustyBoy's CPU is based on a microcode specification that is completely written in a Google Sheets
document ([instructions.csv](soc/instructions.csv))! The document describes the microcode-level
process for each instruction at each T-cycle.

The microcode csv is then read by the [asm compiler](soc/src/cpu/asm), which verifies and compiles
compiles the instructions into microcode. This microcode is what is actually used **both** by the
Rust emulator, **and** the FPGA CPU.

This has the interesting side-effect that the [CPU control unit](soc/src/cpu/control_unit.rs) is
relatively simple - almost 260 lines. Most of the heavy lifting is in the data!

## Status

At this point, I consider the emulator to be feature-complete. It's (almost) perfectly cycle
accurate; at least in the areas that I care about.

It passes all Blargh, all (but one) MooneyeGB, and almost all Wilbert Pol tests. See the [complete
test status](docs/test_details.md) for a more detailed list of all passing/failing tests.

Of course, passing unit tests is all fine and dandy. The real fun is being able to run demo-scene
ROMs (and video games). oh.gb and pocket.gb run almost flawlessly. Here is a montage of my favorite
parts:

<p align="center">
    <image src="docs/rustyboy.gif" />
</p>


## License

RustyBoy is currently released as GPLv3, because I see no reason why anyone would derive from it.
If you'd like a more permissive license, email me!
