[![built with nix](https://builtwithnix.org/badge.svg)](https://builtwithnix.org)

<h1 align="center"><i>cargo-reaper</i></h1>

<p align="center"><img src="https://raw.githubusercontent.com/Cloud-Scythe-Labs/cargo-reaper/refs/heads/master/assets/rea-corro.svg" alt="Corro the Unsafe Rust Urchin" width="200"/></p>

<p align="center">A Cargo plugin for developing REAPER extension and VST plugins with Rust.</p>

`cargo-reaper` is a convenience wrapper around Cargo that adds a post-build hook to streamline REAPER extension development. It automatically renames the compiled plugin to include the required `reaper_` prefix and symlinks it to REAPER’s `UserPlugins` directory.

By default, Cargo prefixes dynamic libraries with `lib`, which REAPER does not recognize. Manually renaming the plugin and keeping the `UserPlugins` directory up-to-date can be tedious -- `cargo-reaper` takes care of all that for you, across all supported platforms.

## reaper-rs

`cargo-reaper` is made to be a companion for [`reaper-rs`](https://github.com/helgoboss/reaper-rs) which is a rust library for writing REAPER plugins that includes an extension plugin bootstrap proc macro.

## Getting Started

### Extension Plugins

To initialize, build and run your first `cargo-reaper` extension plugin:

```sh
cargo reaper new reaper_hello_world_extension
cargo reaper run
```

### VST Plugins

> \[!WARNING\]
> Currently the `reaper-rs` library supports using the `vst` crate which is deprecated. In an effort to maintain support for `reaper-rs`,
> `cargo-reaper` can initialize VST plugins, but it does not add any conveniences like it does for extension development.

To initialize and build a VST plugin:

```sh
cargo reaper new --template vst reaper_vst_plugin
cargo build
```

For more information on how to use `cargo-reaper` [read the docs](https://cloud-scythe-labs.github.io/cargo-reaper/), or [start a discussion](https://github.com/Cloud-Scythe-Labs/cargo-reaper/discussions).

Please be sure to check that your issue has not already been resolved before opening a new discussion or issue.
