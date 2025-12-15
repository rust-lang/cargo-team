# Multiple registries

Diff between mirrors and private registries

- Fallback to mirrors implies SHA checksums must match
    - If that is the case, cargo may transparently fall back
- Private registry does not need to match
    

Strawman:

- Cargo.toml has `[registries]` section, you can add a registry to it (e.g. `linkedin = linkedin.com/crates`
- In a dependency, you can have `registry` source, eg `registry = linkedin`
- Specify the registry format.

Interface:
New top level section: [registries]

- name-of-registry = url

In dependencies:
{name: cratename, registry = name-of-registry}

Need to create a spec for crates.io-index format

- Downside of abstracting behind a library: people won't keep it up to date
    - Then we need to support arbitrarily old formats
- If we create a spec for the current structure,
    - we can version it to evolve it
    - there are already lines in the crates.io-index
- We should try to avoid requiring updates to the registry

Spec:

- it should be a git repo
- with a certain directory structure
- containing files
- top-level config.json containing "dl" specifying where to find tarballs
- we should support in cargo.toml either specifying location of crates.io-like host/url or registry url
    - so that people can just put their custom registry on github and use it
- other registry urls that crates are allowed to depend on in this registry


constraint: cargo must be able to resolve dependencies offline
spec would not include API of crates.io like publish/search for now
follow up rfc someday - cargo publish and cargo search enabled
we should standardize crates.io’s JSON api someday, not needed right now

if crates.io looks at a .cargo/config in any way, we should stop doing that? check

levels of running a registry:

- make a git repo by hand, no api server necessary
- tool (in cargo) that reads from cargo.toml, gives a hash that’s what needs to go in the git repo
    - lower than cargo publish: `cargo update-registry`, `publish-metadata`, etc, just adds metadata to an index, does not upload the package anywhere
- good crates-io-like UI

