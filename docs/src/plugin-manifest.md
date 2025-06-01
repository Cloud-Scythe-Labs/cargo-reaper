# Plugin Manifest

There are a few necessities for `cargo-reaper` to recognize an extension plugin that is declared in a [configuration file](./configuration-file.md).

1. The cargo manifest must be a [_package_](https://doc.rust-lang.org/cargo/appendix/glossary.html#package).
2. The package must include a [_library target_](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#configuring-a-target).
3. The library target must include a [`name`](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-name-field) field.
4. The library target must include [`cdylib`](https://doc.rust-lang.org/reference/linkage.html#r-link.cdylib) in the [`crate-type`](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-crate-type-field) field.

The above should be true whether the project is a single package, workspace with multiple packages or workspace package.

## Package Manifest

An example of a single package manifest and its corresponding [configuration file](./configuration-file.md).

```toml
# Cargo.toml
[package]
name = "my_package"
version = "0.1.0"
edition = "2024"

[lib]
name = "my_extension_plugin"
crate-type = ["cdylib"]
```
```toml
# reaper.toml
[extension_plugins]
reaper_my_plugin = "./."
```

## Workspace Manifest

Examples of acceptable workspace patterns and their corresponding [configuration file](./configuration-file.md)s.

### Workspace with Multiple Package Manifests

An example of a virtual workspace that does not contain a package attribute, but consists of multiple members,
each of which being a package manifest with the [plugin manifest criteria](#plugin-manifest).

```toml
# Cargo.toml
[workspace]
resolver = "2"
members = ["crates/*"]
```
```toml
# reaper.toml
[extension_plugins]
reaper_my_plugin_1 = "./crates/my_plugin_1"
reaper_my_plugin_2 = "./crates/my_plugin_2"
```

### Workspace Package Manifest

An example of a workspace package manifest. The library target [`path`](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-path-field) field may be necessary in some cases, depending on the
workspace layout. For brevity, the default path is specified below.

```toml
# Cargo.toml
[workspace.package]
name = "my_workspace_package"
version = "0.1.0"
edition = "2024"

[lib]
name = "my_extension_plugin"
path = "./src/lib.rs"
crate-type = ["cdylib"]
```
```toml
# reaper.toml
[extension_plugins]
reaper_my_plugin = "./."
```
