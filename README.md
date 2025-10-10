# Ragu

Ragu is a minimal Rust port of the popular PureScript package manager, [Spago](https://github.com/purescript/spago).

It is designed to be a drop-in replacement for Spago, with a focus on performance and simplicity.

## ⚠️ Early Development Stage

This project is in the early stages of development and is not yet ready for production use.

This was built with our project in mind and hasn't been thoroughly tested in other environments.

It was developed over a few days by a single developer (+ some vibe-ing), so there are several rough edges that need refinement.

Rust is relatively new to me so sorry for any smelly or non-idiomatic code...

## Reasoning

Spago is an excellent package manager for PureScript, but it is written in JavaScript and has performance issues on large workspaces. This caused significant challenges at my current company [Oxford Abstracts](https://www.oxfordabstracts.com/).

We had seen impressive performance improvements from [Rust in other areas of PureScript](https://github.com/purefunctor/purescript-analyzer) and it seemed like an ideal fit for a simple port of Spago.

## Features

Most of the key day-to-day features of Spago are supported:

- Build purs projects
- Read spago.yaml packages
- Read workspace configurations
- Install packages from git package sets
- Install packages from git repos directly
- Install packages from local paths
- Install packages from the registry

Heavy caching is implemented for packages and package sets, so slow commands typically only need to be run once.

## Key differences

There are several key differences from Spago:

- No package publishing
- No version ranges, only package sets or manually specified github packages.
- No `-p` - all nested workspace work handled by navigating to the workspace directory and running commands from there.
- Some new commands:
  - `ragu workspace` - list all local packages
  - `ragu circular-deps` - check for circular dependencies in the workspace
  - `ragu check-deps` - check for broken dependencies in the workspace and suggest fixes
  - `ragu imports` - analyze imports in source files and categorize them
  - `ragu modules` - analyze modules in source files (can be grouped or filtered by package, etc.)

The `check-deps` equivalent in Spago took around 20 minutes to run on our workspace. It now takes under a second and can be run in CI.

The same is true of `circular-deps`, which we now run along with `check-deps` to ensure dependencies in our (stupidly large) workspace remain healthy.

## Usage

```bash
ragu help
```

## Replacing Spago locally

```bash
cargo install --path .
```

Then alias `spago` to `ragu` if you want it to interact with exsiting tooling that expects `spago sources`.
