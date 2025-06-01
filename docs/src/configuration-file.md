# Configuration File

`cargo-reaper` requires a configuration file in the project root that conforms to the following
naming convention (in precedence order):

- `reaper.toml`
- `.reaper.toml`

> This file is created automatically for projects initialized by [`cargo-reaper-new`](./commands/new.md).

## Declaring Extension Plugins

`cargo-reaper` expects a key-value pair mapping of reaper extension plugins, where the key is the finalized name of the
plugin and the value is the path to a directory containing a cargo manifest. All other information `cargo-reaper` needs
is gathered from the manifest file.

A minimal `reaper.toml`, for a single cargo package could look like the following:

```toml
[extension_plugins]
reaper_hello_world_extension = "./."
```

> _**Important**_: REAPER requires that extension plugins be prefixed by `reaper_`, otherwise REAPER will not recognize it.
>
> `cargo-reaper` will throw an error and refuse to compile if an extension plugin listed does not meet this condition.
