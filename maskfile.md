# Tasks

## setup

> sets the project up and checks if everything is ok to start developing

**OPTIONS**
* allow
    * flags: -y --yes
    * desc: Allows direnv to run

```bash
if hash nix 2> /dev/null; then
  if [ ! -v name ]; then
    echo "Building nix environment..."
    nix develop --build
  fi
  if hash direnv 2> /dev/null; then
    if [ -v name ]; then
      echo "Already in direnv environment"
    else
      if [ -v allow ]; then
        direnv allow
      else
        echo "Starting 'nix develop'..."
        nix develop
      fi
    fi
  else
    echo "Starting 'nix develop'..."
    nix develop
  fi
else
  echo "You don't have Nix, you need to setup the environment by yourself. :("
fi
cargo build
cargo doc
```

## build

> Builds project

```bash
cargo build
```

## test

> Tests my project

```bash
cargo test
```

## lint

> Lints the project with clippy

Using a separate `target-dir` so it does not conflict with rust-analyzer running in VSCode.

**OPTIONS**
* watch
    * flags: -w --watch
    * desc: Runs in watch mode with Bacon

```bash

if [ -v watch ]; then
  bacon clippy -- --all-features --target-dir ./target/clippy/
else
  cargo clippy --all-features --target-dir ./target/clippy/
fi
```
