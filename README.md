# cargo-reaper

`cargo-reaper` is a convenience wrapper around Cargo that adds a post-build hook to streamline REAPER extension development. It automatically renames the compiled plugin to include the required `reaper_` prefix and symlinks it to REAPER’s `UserPlugins` directory.

By default, Cargo prefixes dynamic libraries with `lib`, which REAPER does not recognize. Manually renaming the plugin and keeping the `UserPlugins` directory up-to-date can be tedious -- `cargo-reaper` takes care of all that for you, across all supported platforms.

## reaper-rs

`cargo-reaper` is made to be a companion for [`reaper-rs`](https://github.com/helgoboss/reaper-rs) which is a rust library for writing REAPER plugins that includes an extension plugin bootstrap proc macro.

## Getting Started

To initialize, build and run your first `cargo-reaper` extension plugin:

```sh
cargo reaper new reaper_hello_world_extension
cargo reaper run
```

For more information on how to use `cargo-reaper` run `cargo reaper --help`, or [start a discussion](https://github.com/Cloud-Scythe-Labs/cargo-reaper/discussions).

Please be sure to check that your issue has not already been resolved before opening a new discussion or issue.
