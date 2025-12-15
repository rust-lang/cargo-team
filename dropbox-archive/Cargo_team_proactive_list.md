# Cargo team proactive list

## Nominated:
- shared branding between crates & trust issues about authorship
    - [sub-crate-namespace proposal](https://twitter.com/hdevalence/status/891858603371409408) - the only namespace proposal carol has been not immediately against. “Cargo could keep the current system of first come, first served names, but allow people to construct sub-names. For instance, someone who registered the 'foo' crate could have 'foo/bar', 'foo/baz', etc”
    - Recently gained org team pages - does this help at all? ex: [rust-lang:libs](https://crates.io/teams/github:rust-lang:libs)
- Release Cargo 1.0: https://github.com/rust-lang/cargo/issues/4211
    - Can we please stabilize the [Cargo versioning proposal](https://github.com/rust-lang/cargo/issues/4682) first? (Is that the right link? No, that’s not the right link, but there’s no right link and the last proposal is moribund.)
- Cargo installing non-Rust-related files, e.g. manpages
    - install.rs proposal from All-Hands in Berlin
- Cargo wrappers: currently, we have ~~two~~ three cargo wrappers (xargo for cross-compilation, wargo for web-assembly stuff, [fargo](https://fuchsia.googlesource.com/fargo/) for fuchsia), and there’s vargo being prepared for neon (https://github.com/neon-bindings/rfcs/pull/4). Should we design some extension points in Cargo for such wrappers? (I (matklad) don’t know anything about this wrappers except that they exist, and that in the ideal world they should not exist 🙂 )
- Private local dependencies (unpublished path dependencies): https://github.com/rust-lang/cargo/pull/4735
## Backlog:
- Write up document on how to think about semver
- Package vs crate
- Make sccache more of a thing
- More work laying out the pillars
- Recruiting/mentoring
    - What is the “shepherd” story for Cargo team?
    - Who do we have our eyes on?
- FAQ/addressing persistent myths
- templates
- [automatic features](https://github.com/rust-lang/rfcs/pull/1787)
- [profile-based features](https://github.com/rust-lang/rfcs/pull/1956)
- path overrides
- independent control over dependency compilation mode
- optional dependencies, allowing cargo to pull in multiple versions of the same crate
    - goal: be able to provide serde support across a range of different serde versions
    - related to automatic features
- rustc version specification
- cargo version https://github.com/rust-lang/cargo/issues/4211
    - should cargo-the-tool version match rust version?
    - should cargo-the-lib be renamed and keep the version?
- [std on crates.io](https://github.com/rust-lang/rfcs/pull/1133)
    - May also need to discuss how a std crate on crates.io could internally use (and hide) Rust unstable features, as the integrated std currently does. Should be possible to use std from crates.io without using nightly
- `cargo clean` is a disaster
- exclusive features: https://github.com/rust-lang/cargo/issues/2980
    - Currently, Cargo features *supposed* to be additive: that is, it should always be safe to activate more features then needed.
    - This is not documented, and is not checked at all, so, de-facto, a lot of published crates do use conflicting features, and some of them really need to (for example, `-sys` crates with different backends). 
- policy for surfacing subcommands from the ecosystem, deciding when to include in Cargo proper, overall vision for CLI



# Discussed
- Quick decision, probably:  [For a package where a library and a binary have the same name, have `cargo doc` ignore the bin](https://github.com/rust-lang/cargo/issues/4341)
    - decision: document only the lib, no warning, make sure `cargo doc --bin` works
    - Provide some messaging and guidance to accompany this decision, help people understand what they should do.
- Cargo docs - [tracking issue](https://github.com/rust-lang/cargo/issues/4040)
    - Would like to have these better before the book comes out so the book can point to them without apologizing
    - A contributor started converting current docs to mdbook, ok to run with that?
    - How to integrate with trains/doc.crates.io/doc.rust-lang.org?
    - Including docs of the pieces of build system integration that we have today!
    - decision:
        - convert to mdbook in cargo repo
        - rust-lang/rust git submodule builds the docs 
        - serve from doc.rust-lang.org
        - redirect (start with reverse, move to the other way)
        - talk with docs team about URL
- npm typosquatting + build script exfilling sensitive env vars issue, how do we address similar problems on crates.io?
    - http://blog.npmjs.org/post/141702881055/package-install-scripts-vulnerability
    - https://twitter.com/o_cee/status/892306836199800836
    - npm might be creating a script to look for names with a small edit distance to other names
    - What are build scripts allowed to do or not?
    - What would be involved in build script sandboxing/whitelisting?
    - Could we address this with the more principled build script thing that might be built as part of the build system integration story?
    - How important is this?
    - Decision/discussion:
        - It’s not just build scripts that’s a problem, any crate could contain any code that does anything
        - We should add a button to make it easy to report a suspicious package
        - We should take a look at npm’s typosquatting analysis when they have that available, see if we can reuse/copy

