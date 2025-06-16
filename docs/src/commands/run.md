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

`-o` _path_ </br>
`--open` _path_ </br>
`--open-project` _path_ </br>
  <dd>Open a specific REAPER project file.</dd>

`--no-build` </br>
  <dd>Do not build plugin(s) before running REAPER.</dd>

`-t` _duration_ </br>
`--timeout` _duration_ </br>
  <dd>The amount of time to wait before closing REAPER, in human-readable format (e.g. 10s, 2m, 1h).</dd>

`--stdin` _stdio_ </br>
  <dd>Configuration for the child process’s standard input (stdin) handle.</dd>

`--stdout` _stdio_ </br>
  <dd>Configuration for the child process’s standard output (stdout) handle.</dd>

`--stderr` _stdio_ </br>
  <dd>Configuration for the child process’s standard error (stderr) handle.</dd>

`-h` </br>
`--help` </br>
  <dd>Print help information.</dd>

## ADDITIONAL LINUX OPTIONS

The following options require `xserver` to be configured and have `Xvfb` and `xdotool` installed.
These options are intended to enable testing in headless environments and to make it easier to
assert the state an extension plugin reaches.

`--headless` </br>
  <dd>Run REAPER in a headless environment.</dd>

`-D` _display_ </br>
`--display` _display_ </br>
  <dd>The virtual display that should be used for the headless environment. Can also be passed with the <code>DISPLAY</code> environment variable, e.g. <code>DISPLAY=:99</code>.</dd>

`-w` _title_ </br>
`--locate-window` _title_ </br>
  <dd>Locate a window based on its title and exit with status code 0 if found.</dd>

`--keep-going` </br>
  <dd>Continue until the specified timeout, even after a window is located.</dd>

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

3. Run REAPER in a headless environment through `Xvfb` on Linux, and attempt to locate a window.
```sh
DISPLAY=:99 cargo-reaper run --headless \ # open REAPER on Xvfb display 99
  --no-build \                            # but don't build any plugins
  --open /path/to/my_project.RPP \        # open a pre-saved project (maybe with state that affects how the plugin behaves)
  --locate-window "error: expected ..." \ # and locate an error window and exit successfully
  --timeout 15s \                         # but only run REAPER for a maximum of 15 seconds
  --keep-going \                          # and don't exit until the timeout is reached (even if the window is found)
  --stdout null \                         # do not print Xvfb or REAPER info to stdout
  --stderr null                           # do not print Xvfb or REAPER errors to stderr
```

> TIP: The above assumes the extension plugin is already installed, skipping the build phase.
> This can be particularly useful since it doesn't require configuring a rust toolchain in order
> to build the plugin prior to testing it.
