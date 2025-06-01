# cargo-reaper-run

## NAME
cargo-reaper-run -- Run REAPER extension plugin(s).

## SYNOPSIS
`cargo-reaper run` [_options_] [_cargo_build_args_]...

## DESCRIPTION
Compile extension plugins and open REAPER.

This is effectively shorthand for running [`cargo-reaper-build`](./build.md) then opening REAPER, which will make
changes to your extension plugins take immediate effect.

`cargo-reaper-run` will attempt to use the REAPER binary executable on `$PATH` if it's available, otherwise falling
back to the platform specific default global installation path. If for some reason the default installation is not
working, please see the options below for manually specifying a path to a REAPER binary executable.

## OPTIONS

`-e` _path_ </br>
`--exec` _path_ </br>
  <dd>Override the REAPER executable file path.</dd>

`-h` </br>
`--help` </br>
  <dd>Print help information.</dd>

## EXAMPLES

1. Build a package or workspace containing a REAPER extension plugin and all of its dependencies, and open REAPER.
```sh
cargo reaper run
```

2. Build only the specified package in a workspace containing a REAPER extension plugin with optimizations for x86_64 Windows, and open REAPER.
```sh
cargo reaper run -p reaper_my_plugin --lib --release --target x86_64-pc-windows-msvc
cargo reaper run -- -p reaper_my_plugin --lib --release --target x86_64-pc-windows-msvc
```

> Note that arguments passed to the `cargo-build` invocation must be trailing. These may be passed directly, or as positional arguments.
