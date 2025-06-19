# Platform Support

Any platform which is both supported by Rust and REAPER should work.

> **Important**: [_**extension plugins built for Windows must be built with MSVC**_](https://www.reaper.fm/sdk/plugin/plugin.php).

The following are the canonical sources of truth for REAPER platform support:

- [REAPER Download](https://www.reaper.fm/download.php)
- [REAPER Extensions SDK](https://www.reaper.fm/sdk/plugin/plugin.php)

Which can then be cross referenced against the Rust target list:

- [Rust Platform Support](https://doc.rust-lang.org/rustc/platform-support.html)

Or via `rustc` from the terminal:
```sh
rustc --print target-list
```
