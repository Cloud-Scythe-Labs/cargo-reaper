# cargo-reaper

## NAME
cargo-reaper -- A Cargo plugin for developing REAPER extension plugins with Rust.

## SYNOPSIS
`cargo-reaper` _command_

## DESCRIPTION
`cargo-reaper` is a convenience wrapper around Cargo that adds a post-build hook to streamline REAPER extension development. It automatically renames the compiled plugin to include the required `reaper_` prefix and symlinks it to REAPER’s `UserPlugins` directory.

By default, Cargo prefixes dynamic libraries with `lib`, which REAPER does not recognize. Manually renaming the plugin and keeping the `UserPlugins` directory up-to-date can be tedious -- `cargo-reaper` takes care of all that for you, across all supported platforms.

## COMMANDS

Each command is documented in its own section:

[`cargo-reaper new`](./commands/new.md) </br>
  <dd>Scaffold a new plugin project.</dd>

[`cargo-reaper list`](./commands/list.md) </br>
  <dd>Print plugin information to <code>stdout</code>.</dd>

[`cargo-reaper build`](./commands/build.md) </br>
  <dd>Compile REAPER plugin(s).</dd>

[`cargo-reaper link`](./commands/link.md) </br>
  <dd>Manually symlink plugin(s) to REAPER's <code>UserPlugins</code> directory.</dd>

[`cargo-reaper run`](./commands/run.md) </br>
  <dd>Compile plugin(s) and launch REAPER.</dd>

[`cargo-reaper clean`](./commands/clean.md) </br>
  <dd>Remove generated symlinks and artifacts.</dd>

[`cargo-reaper completions`](./commands/completions.md) </br>
  <dd>Generate shell completions.</dd>

`help` </br>
  <dd>Print help or the help of the given subcommand(s).</dd>

## OPTIONS

`-h` </br>
`--help` </br>
  <dd>Print help (see more with <code>--help</code>).</dd>

`-V` </br>
`--version` </br>
  <dd>Print version information.</dd>
