# cargo schema version (RFC PR#1953)
Thread: https://github.com/rust-lang/rfcs/pull/1953
Rendered: https://github.com/joshtriplett/rfcs/blob/cargo-schema-version/text/0000-cargo-schema-version.md

**Background: versioning cargo & rust dependencies**


- Rust/cargo are backwards compatible, but not forwards compatible. Old versions will fail to compile crates that depend on newer versions.
- No way to specify what your minimum Rust/cargo version is; if you aren’t updating to latest stable we give you very little help.

This RFC proposes to require crates to specify the version of cargo they depend on:

**Major problems with this RFC:**


- It proposes we introduce possibly as many as **four** versions:
    - A version for the cargo.toml manifest format
    - A version for cargo itself
    - A version for rustc
    - An epoch version
- It proposes a bizarre syntax for declaring manifest version, specifically to guarantee that existing cargos fail to parse new manifests:
    [package.1.0.0]
    name = "foobar"
    ...

**Discussion thread:**

Some of us have pushed back on these problems, the RFC authors seem sympathetic to changing the syntax to be more expected. Authors seem more conflicted about not versioning cargo and Rust separately.

We’ve determined that epochs cannot be derived from rustc versions, so the minimum number of versions possible is two:


- A release version (1.18 etc)
- An epoch version (2017, etc)

**Other questions:**


- What version does `cargo new` initiate with?
- If you depend on a feature from newer version than your toml states (but which your toolchain has), do we warn, error, or do nothing?


**Conclusion:**


- Add a version to the manifest — default to the version being used to publish
- Warn when compiling a crate that depends on a higher version of rust than you are
- Flag to limit resolution to only crates with compatible rust versions

