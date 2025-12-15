# Unstable Cargo features

## Possible constraints
- We want an experience like the compiler that gives a lot of clarity around stability
- Can only use unstable Cargo features on the nightly channel
## Open questions
- crates.io integration
    - Do we allow publishing crates that use nightly features?
        - Josh: to start we shouldn’t let you use nightly Cargo features on crates.io crates
        - Think of this as an “app” feature, or purely for custom registries
        - Can then allow them when we can make it much more obvious that there’s a nightly dependency
- Do we need an RFC?
    - Josh: may want this for discussion around crates.io publication
    - Josh: Would like to discuss versioning and similar before we *stabilize* any nightly Cargo features; implementation should proceed in the interim, as long as we sort out versioning before we declare anything stable. After all, nightly features are allowed to break or change incompatibly, including the mechanism for nightly features themselves.
    - Portable build script stuff may want experimentation
## Strawman

MVP:


    [package]
    name = "..."
    nightly-cargo-features = ["..."]


- Only possible on nightly channel of Rust
- Can’t publish to crates.io if you use this
    - Will be enforced on the server side of crates.io (old versions of cargo won’t disable publish with a nonworking `nightly-cargo-features` key, or you could go around cargo, so this needs to be validated server-side)
    - Anyone running their own crates.io server could turn this off for their server (even though we don’t officially support custom registries yet)
- Follow rustc convention for unstable CLI options?
    - `-Z unstable-options --build-plan`

