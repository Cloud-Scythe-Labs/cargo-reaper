# Introduction

`cargo-reaper` is a Cargo plugin designed to streamline the development of REAPER extension plugins with Rust.
It serves as a companion for the [`reaper-rs`](https://github.com/helgoboss/reaper-rs) library,
which provides Rust bindings and tools for creating REAPER plugins -- including a procedural macro that bootstraps your plugin as a native REAPER extension.

## Motivation

Developing REAPER extension plugins requires intimate knowledge about REAPER and its behavior on each platform that it supports.
This information is somewhat esoteric and not listed in the development docs, making extension plugin development a trial-and-error ordeal.
`cargo-reaper` aims to simplify the learning curve by providing an easy-to-use, intuitive interface for initializing, building,
testing and publishing REAPER extension plugins.

Throughout this book are references to REAPER specific terminology, most of which you may safely ignore since `cargo-reaper`
handles it for you, however, if you wish to know more the [`Glossary`](./appendix/glossary.md) section contains detailed documentation
that may aid you in understanding how `cargo-reaper` works. It is recommended to have a once-over for anyone creating a non-trivial
extension plugin.
