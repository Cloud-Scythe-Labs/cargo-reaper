# Glossary

## Default Global Installation Path

This is the system-wide path where the REAPER binary executable is installed.
This is different for each supported platform:

- Linux:
  - A global default is not predictable since Linux does not have a canonical package manager. Instead, `cargo-reaper` uses the
  [`which`](https://crates.io/crates/which) crate's [`which::which_global`](https://docs.rs/which/7.0.3/which/fn.which_global.html)
  function to determine its location.
- Darwin (MacOS) -- `/Applications/REAPER.app`
- Windows:
  - x86 (32bit) -- `C:\Program Files (x86)\REAPER\reaper.exe`
  - x86_64 (64bit) -- `C:\Program Files\REAPER (x64)\reaper.exe`
  - aarch64 (ARM) -- `C:\Program Files\REAPER (ARM64)\reaper.exe`

## Extension Plugin

A C/C++ [dynamically linked library](#dynamically-linked-library) that when placed in REAPER's [`UserPlugins`](#user-plugins) directory, is loaded as part of the REAPER
application thread on launch, adding additional functionality to the program.

## Dynamically Linked Library

A compiled collection of code and data that is loaded into a program at runtime, rather than being statically included in the final executable
during compilation.

REAPER [extension plugins](#extension-plugin) are dynamically linked libraries, which have differing extension names depending on their target platform:

- Linux -- `.so`
- Darwin (MacOS) -- `.dylib`
- Windows -- `.dll`

## User Plugins

The `UserPlugins` directory is the file system location created the first time REAPER is launched that the REAPER application thread loads [extension plugins](#extension-plugin) from.
This is different for each supported platform:

- Linux -- `~/.config/REAPER/UserPlugins`
- Darwin (MacOS) -- `~/Library/Application\ Support/REAPER/UserPlugins`
- Windows -- `%APPDATA%\REAPER\UserPlugins`

## Plugin Manifest

A plugin manifest, in the context of `cargo-reaper`, is the same as a [Cargo manifest](https://doc.rust-lang.org/cargo/appendix/glossary.html#manifest), which contains
a [package](https://doc.rust-lang.org/cargo/appendix/glossary.html#package) with a [library target](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#configuring-a-target) of [`crate-type`](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-crate-type-field) [`cdylib`](https://doc.rust-lang.org/reference/linkage.html#r-link.cdylib).

See the [`Plugin Manifest`](../plugin-manifest.md) section for detailed information on configuring a `cargo-reaper` plugin manifest.
