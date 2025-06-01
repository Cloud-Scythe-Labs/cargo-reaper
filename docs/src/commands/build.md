# cargo-reaper-build

## NAME
cargo-reaper-build -- Compile REAPER extension plugin(s).

## SYNOPSIS
`cargo-reaper build` [_options_] [_cargo_build_args_]...

## DESCRIPTION
Compiles, renames and symlinks REAPER extension plugins, forwarding trailing arguments to the [`cargo-build`](https://doc.rust-lang.org/cargo/commands/cargo-build.html#options) invocation.

The resulting `target` is renamed to its corresponding key specified in the [`cargo-reaper` configuration file](../configuration-file.md).
Symlinks to plugins in REAPER's `UserPlugins` directory are managed automatically, unless specified otherwise. This ensures that a new plugin
that is built with the `release` profile, does not fail to be symlinked if a symlink with the same name already exists for the `debug` profile.

If for whatever reason symlinking fails, and the build command is unable to remove a stale symlink, use [`cargo-reaper-clean`](./clean.md).

## OPTIONS

`--no-symlink` </br>
  <dd>Prevent symlinking extension plugin(s) to the <code>UserPlugins</code> directory.</dd>

`-h` </br>
`--help` </br>
  <dd>Print help information.</dd>

## EXAMPLES

1. Build a package or workspace containing a REAPER extension plugin, and all of its dependencies.
```sh
cargo reaper build
```

2. Build a package or workspace containing a REAPER extension plugin and all of its dependencies, but do not create symlinks to the `UserPlugins` directory.
```sh
cargo reaper build --no-symlink
```

3. Build only the specified package in a workspace containing a REAPER extension plugin with optimizations for x86_64 Windows.
```sh
cargo reaper build -p reaper_my_plugin --lib --release --target x86_64-pc-windows-msvc
cargo reaper build -- -p reaper_my_plugin --lib --release --target x86_64-pc-windows-msvc
```

> Note that arguments passed to the `cargo-build` invocation must be trailing. These may be passed directly, or as positional arguments.
