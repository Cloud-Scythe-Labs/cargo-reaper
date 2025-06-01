# cargo-reaper-link

## NAME
cargo-reaper-link -- Manually symlink extension plugin(s) to REAPER's `UserPlugins` directory.

## SYNOPSIS
`cargo-reaper link` [_path_]...

## DESCRIPTION
Manually symlink one or more extension plugins to REAPER's `UserPlugins` directory.

This may be useful in circumstances where finer grain control is necessary between building
and symlinking the plugin, for instance, in CI or when using build tools like Nix or Docker.

> By default [`cargo-reaper-build`](./build.md) will symlink extension plugins automatically, unless specified otherwise.

## OPTIONS

`-h` </br>
`--help` </br>
  <dd>Print help information.</dd>

## EXAMPLES

1. Create a symlink from an absolute path to a compiled REAPER extension plugin to REAPER's `UserPlugins` directory.
```sh
cargo reaper link /absolute/path/to/target/release/reaper_my_plugin.{so|dylib|dll}
```

2. Create a symlink from a relative path to a compiled REAPER extension plugin to REAPER's `UserPlugins` directory, using shell scripting (Linux and MacOS only).
```sh
cargo reaper link $(realpath target/release/reaper_my_plugin.*)
```

> REAPER extension plugins are dynamically linked libraries, which have differing extension names depending on their target platform.
> Below is a list of platforms and their corresponding extension names, though in most cases, a regex catchall will suffice (`reaper_my_plugin.*`).
>
> - Linux -- `.so`
> - Darwin (MacOS) -- `.dylib`
> - Windows -- `.dll`
