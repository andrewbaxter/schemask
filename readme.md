Schemask is a data validation specification intended for:

- Data validation
- Safety (i.e. clear expectations, conventions, few footguns)
- Documentation
- Code generation

In contrast, JSONSchema can describe all sorts of crazy data specifications, but because of that flexibility it makes it very hard to read specifications, it's easy to miss holes in the validation, and using it to generate general serializers in most languages is more or less impossible (it may be possible for some domains that restrict themselves to a limited subset).

Unlike JSONSchema, Schemask is not indended to be shoehorned into existing data: the idea is that new software targets schemask and adapts its data to the restrictions of schemas, and in exchange gains easy interoperability with multiple ecosystems.

This repo is:

- A library for doing common tasks (validating data, generating language interfaces)
- A command line utility exposing the same

Schemask is intended to describe data in multiple formats/languages so the following description isn't intended to be json-specific, but json is used in examples as a representative encoding.

# How to use it

If you're writing a program that takes JSON, publish a schemask spec JSON file. If you're using rust, this can be generated using the `#[derive(Schemask)]` macro. Use `schemask generate-markdown` to generate a markdown description of your data format you can publish for users to refer to.

If you're writing a program that needs to produce such JSON, use `schemask generate-typescript` or `schemask generate-rust` to generate type definitions for the schema. Use those in your program to produce conformant JSON.

If you're using rust, you can skip the CLI and generate the types inline at compile time with the `from_schemask!` macro, giving it a path to a schemask spec file relative to the current source file:

```rust
// Expands to structs/enums (with serde attributes) matching `player.json`.
schemask::from_schemask!("schemas/player.json");
```

The generated types carry the serde attributes needed to round-trip against the schema: `#[serde(deny_unknown_fields)]` on records, `#[serde(rename = "...")]` when the idiomatic Rust name differs from the wire key, and optional-field skipping. The file is tracked as a build dependency, so edits trigger a recompile.

# Overview

A schemask schema is wrapped in a single-key object naming the schemask version, currently always `v1`:

```json
{ "v1": { "bindings": {}, "default": null } }
```

The inner object has the following fields:

- Bindings: a list of names and associated types
- A default root type: which binding to start with when matching data, if none is specified

All keys in a schemask specification are snake case, and unknown keys are rejected.

Schemask defines the following types and examples of their matching JSON:

- Any: Matches anything
- Null: `null`
- String: `"hello"`
- Const string (`"magic"`): matches exactly the one string, `"magic"`
- Bool: `true` `false`
- Int: `7` (rejects floats)
- Float: `1.34` (any number)
- Ref (`binding`): Anything - this matches against the type in the binding with the specified binding name
- Option (TYPE): Matches `TYPE` or `null`. If options are nested, the inner option matches `{"element": TYPE | null}` instead.
- Set (TYPE): Matches an array, like `["a", "b", "c"]` if TYPE is `string`. Duplicates are rejected. Order doesn't matter.
- List (TYPE): Matches an array, like `["a", "b", "c"]` if TYPE is `string`. Order is significant.
- String map (TYPE): Matches an object, like `{k: v}` where `v` matches TYPE.
- Tuple `[TYPE1, TYPE2, ...]`: Matches an array where each element can have a different type specification (like `["a", 4]`)
- Tagged union `{KEY1: TYPE1 | KEY2: TYPE2}`: Matches an object that contains exactly one of the listed `KEY` `VALUE` pairs. A variant whose type is null also matches the bare string `"KEY"`, which is how such a variant is written.
- Record `{KEY1: TYPE1, KEY2: TYPE2}`: Matches an object that has every listed key and a matching value, and no others. A key whose type is an option may be omitted instead of being set to null.

For perhaps slightly more rigor, there's a json encoding of a schemask specification for schemask, available by running the cli: `schemask schemask-schema`.

# The CLI

In `source/` do `cargo build`. This will produce a `schemask` binary in `target/debug/`.

# The library

Do `cargo add schemask`.

Generate schemas with `#[derive(Schemask)] struct MyType;`, then `MyType::schemask()`.

Validate schemas with `schemask::validate(schema, data)`.
