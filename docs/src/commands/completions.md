# cargo-reaper-completions

## NAME
cargo-reaper-completions -- Generate shell completion scripts.

## SYNOPSIS
`cargo-reaper completions` _shell_

## DESCRIPTION
Generate shell completions for the `cargo-reaper` command line application.

The following shells are supported:
- [Bash](#bash)
- [Elvish](#elvish)
- [Fish](#fish)
- [PowerShell](#powershell)
- [Zsh](#zsh)

## OPTIONS

`-h` </br>
`--help` </br>
  <dd>Print help information.</dd>

## EXAMPLES

Below are examples of how to generate completion scripts for the supported shells.
Note that this does not include instructions for sourcing the scripts or configuring your shell to enable completions.
Please refer to your shell’s documentation for details.

### Bash

```sh
cargo-reaper completions bash > /usr/share/bash-completion/completions/cargo-reaper.bash
```

### Elvish

```sh
cargo-reaper completions elvish > ~/.elvish/lib/cargo-reaper.elv
```

### Fish

```sh
cargo-reaper completions fish > /usr/share/fish/vendor_completions.d/cargo-reaper.fish
```

### PowerShell

```sh
cargo-reaper completions powershell > $HOME/Documents/PowerShell/cargo-reaper.ps1
```

### Zsh

```sh
cargo-reaper completions zsh > /usr/share/zsh/site-functions/_cargo-reaper
```

