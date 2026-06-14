# 0.7.4

## Features

- Added `#[derive(ComponentBuilder)]` and the documented `#[derive(FormField)]`
  alias for generating component constructors and fluent setters with required
  fields, optional setters, defaults, skipped fields, and renamed setters.

## Fixes

- Required constructor fields now accept `impl Into<T>`, matching the README
  `FormField` example for ID-like fields.

# 0.6.0

## New

- Added a markdown editor as a demo for gpui-toolkit.
- Added support for missing macro fields needed by the new design / builder pattern.

## Fixes

- Fixed an animation bug that could trigger a crash.
- Various engine restart-strategy fixes carried alongside this crate's macro updates.

## Changes

- Made the themes uniform across the UI kit.
- Split the autoeq UI out of the UI Kit so the macros stay focused on generic widget generation.
- Reorganised crates to match the layout published on crates.io for easier downstream consumption.
