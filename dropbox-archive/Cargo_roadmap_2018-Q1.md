# Cargo roadmap 2018-Q1

# Topics to consider
- pub/priv
    - still no progress on implementation
    - this should really land before the next epoch
    - wycats considers this most pressing
- build system integration
    - mshal (the “[Tup](http://gittup.org/tup/)” guy) — patch to produce a build plan
        - basically working, modulo a new feature/constraint in Cargo
        - up-to-date [PR](https://github.com/rust-lang/cargo/pull/4734), needs to be reviewed
    - what’s the next spike
    - extensibility
- profiles revisions
    - high priority — connected to build system integration, FF needs
    - wycats: would like to see targeted mitigations in parallel with profile rethink
    - [Manish has an RFC for custom profiles](https://github.com/rust-lang/rfcs/pull/2282), need to look at it
- metabuild
    - status unknown
    - wycats: some concerns with the current design, need to be worked through
    - wycats: in particular, unsure about taking such an incremental approach
    - aturon: probalby want to focus first on the “please skip the build script and use this instead”
- features revisions
    - wycats: need to tread very carefully, the union rule is very useful
    - not clear that this is super important
- custom registries
    - a couple people are actively using the feature, but want more feedback prior to stabilization
    - small issues/ergonomic concerns still being worked through
    - need to be able to alias a crate name in `Cargo.toml` — was part of the modules RFC
- cargo/rustup integration
    - (would need input from dev tools as to what we should be considering for this as the cargo team, maybe nothing!)
- cargo/xargo merger
    - (imo low priority but would get us a lot of brownie points to pull it off)
- 

