# Cargo build system integration

# 2017-07-27

You’re facebook, you’re trying to write an internal Rust crate.

- You would write a Cargo.toml as usual.
- You would have a way to specify dependencies on non-Rust projects provided by bazel.
- You would be able to specify a dependency on crates that ultimately come from crates.io.
    - Want to satisfy that dependency with a locally mirrored/vendored crate when appropriate.
- When you run `cargo test`, bazel can provide what it’s responsible for in order for cargo to be able to run tests.
- This could be in the form of a tool called `bazel-cargo`

Alex McArther:

- Want to have the same folder structure that other bazel projects use, cargo’s is different
- Want to use protobuf to use something that isn’t a Rust crate but generates an rlib
    - Could this be a crate? This gets onerous with multiple protobuf crates depending on each other. Could groups of protobufs be part of one crate?
        - This doesn’t feel “first class”
        - Harder to interop with things that aren’t Rust crates
    - Would it help if you had a thing like bindgen that could “import” the protobuf protocol from Cargo, so that this becomes a “pull” rather than a “push”?
    - “Build scripts cannot describe their own dependencies”

Yehuda “Yehjdja” Katz:

- “Two ways to do this, don’t have a preference between them”
    - Allow Bazel users to override the build scripts
    - Enumerate the steps of `cargo test` in a declarative way so a Bazel wrapper can substitute what it needs to substitute

Josh Triplett:

- Dependency resolution goes through bazel
- Output directory configurable
    - to integrate with caching? What’s the rationale?
    - cargo’s download of crates - want to make sure it gets versions under version control
    - Want to make vendoring and offline use of cargo more first-class
- General desire to let people swap out “external” components (ex: openssl, crates) with their own (internal) components
- Not hitting the network is a primary use case
- Want reproducibility - don’t want to hit the network, want to use an exact version
- Some users just don’t want to use the cargo executable
    - What’s the rationale?
    - Let’s separate this concern from integrating Cargo with other things.

Alex McArther:

- Would bazel integration call cargo once or multiple times?
    - bazel expects to use low-level pieces deterministically
    - like calling gcc multiple times
    - Multiple top-level projects exist
    - Might be interesting to call Cargo once per library crate, and link them together, more like C libraries in binary form, but that’d be a major change, and might not be the primary concern here.
    - Bazel would have a “cargo library” or “cargo binary” build rule that specifies their dependencies, and each dependency is its own bazel rule bubble

Aaron Turon:

- May want Cargo to be able to generate a “build plan”, feed that to bazel, and let bazel handle dependencies, compiling each crate once.

Yehuda Katz:

- Two crates (A&B), each in a subdirectory in the same directory (monorepo)
- Crate A depends on crate B
- People don’t want Crate A to be responsible for building crate B
- Different dependencies on a crate may use different features
- Want cargo to handle most things, but have an escape hatch for custom requirements

Three different senses of “Use cargo”

- Development workflow
- Compiling single libraries/compilation units
- Compiling applications

Aaron: these are listed in reverse priority order (most important first) in terms of making incremental progress in a less-constrained space
Yehuda: these are all important

Three “tiers” of Cargo functionality: 

1. Built-in (dependency resolution, crate locations/downloading)
2. Declarative (Cargo.toml dependencies, metadata of crates)
3. Programmable (build.rs). 
- Most “common” things at the “programmable” level need to be moved up to become semantic declarative steps.
    - Example: pkgconfig, protobuf
- Some things at the “built-in” level need to be moved down to become semantic declarative steps.
    - Example: dependency resolution (from bazel instead), crate location

What direction is more incremental?

- Action item: Aaron and Yehuda to write down the pieces (Josh is interested too)

Aaron is not frustrated, he just REALLY WANTS TO SPIKE OUT SOMETHING TECHNICAL GEEZ


----------
# Notes from 2017-07-20
# Issues raised on the [roadmap issue](https://github.com/rust-lang/rust-roadmap/issues/12)
- [Configuring linkage](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-277816996) can be a pain
    - *Firstly, to provide that flag I need to "cargo rustc" instead of "cargo build", and to do that I need to detect all the binary/lib targets and build them one at a time. I really wish I could just "cargo build" and have cargo sort out the details for me.*
- [Discovering output artifacts](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-277816996) is harder than it could be
- For build systems like Bazel, [need the ability for Rust projects to act as Bazel deps](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-277840577) [*and*](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-277840577) [to depend on Bazel deps](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-277840577)
    - For common “native deps”, need the -sys crates to pull from the corresponding Bazel dep
    - Some work done on the [Bazel side](https://github.com/bazelbuild/rules_rust/issues/2)
        - [A tool](https://github.com/acmcarther/cargo-raze) to generate Bazel BUILD files from Cargo.toml
            - `*cargo raze*` *gives you the best of both worlds: rust library downloading + resolution courtesy of Cargo with the power and scalability of Bazel.*
        - [A tool](https://github.com/acmcarther/rules_cargo) to invoke Cargo directly
- Cargo’s exports of environment variables can [cause problems](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-278088373)
- `build.rs` files
    - [*build.rs files seem antithetical*](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-278088373) *to bazel's "hermetic ethos", since they can do pretty much anything. I think this will become less of an issue in the very near future since common usecases such as serde are being resolved with the stabilization of macros 1.1.*
    - Particularly a problem for native deps, see above
- Interest in [JVM integration](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-278096012)
- [Still the “downloads things at build time” canard](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-278193719)
    - Alex raised the [vendoring approach](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-278199633)
- Integration with [caching/build management](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-278193719)
    - May be addressable by working toward a single-crate compilation mode
    - *Do you mean* [*Cargo might be able to make use of Buck's cache?*](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-278213103) *That poses lots of problems, not least because its unclear how Cargo would be able to compute the correct key. Buck's cache is indexed by both the immediate dependency (the source file contents), but also the keys of all its dependencies, with the goal of being able to skip as much of the dependency graph as possible. Cargo wouldn't have access to the information needed to either lookup or insert blobs into the cache.*
- [Version management/mono-version-mono-repo issues](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-279811597)
- [Cargo exerting too much control/too deeply assuming it “controls the world”](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-309326738)
# Design constraints/desires
- Integration should allow for easy access to as much of crates.io as possible
- Cargo workflows and Cargo-based tools should be available when working on a crate — think `cargo doc`, the RLS, etc
# jsgf’s [breakdown of Cargo roles](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-279489242)

More generally, you could consider splitting cargo into several distinct parts:


- crate dependency management
    - where you’re getting dependencies from
    - semver and resolution
- a build planner
    - what do we do for all of the dependencies that we now have
- a build execution engine
    - caching
    - specific ways you’re invoking rustc

YK: I don’t love this breakdown, but it’s a good starting point. It’s missing some details about workflows (how does “cargo test” fit into this breakdown?). It also misses the predictability/determinism role that Cargo plays (it fits into “build planner” in an abstract sense, but the details of how it decides what to build are not purely abstractable, nor purely overlapping with other build tools in an obvious way)



> the details of how it decides what to build are not purely abstractable, nor purely overlapping with other build tools in an obvious way

CN: why not?
YK: Let’s discuss via voice, but you’re right that my point is not obvious.

    YK: Ok, let me try. For example, Cargo provides a “conservative updating” system for transitive dependencies. As far as we know, Buck/Bazel people are willing to use Cargo for private transitive dependencies, but Buck doesn’t provide its own facilities for conservative updating. TLDR the Cargo.lock is doing some useful things even in the context of Bazel owning top-level dependencies.

YK: More generally, I think there are two approaches we could take here:

1. dump out a plan, to be imported by bazel
2. make Cargo more generally extensible (cache extensibility, native dep extensibility, various forms of mirroring including vendoring, etc.)

YK: I think both of these are important directions, but more people will be able to take advantage of (2) in my view, and I think we’re under-investing in it at the moment. But I think we need to invest in (1) at the same time.

YK: The benefit of (2) is that you can do things like “drop in sccache” or “drop in Google’s shared caching system” or “cache into the monorepo” in a way that doesn’t necessarily disturb other extensibility aspects.

YK: Worth noting: `cargo test` does a bunch of stuff: inline tests, integration tests (separate crate), build examples, doctests. So it’s not so trivial to reimplement it, and it’s not even really something bazel/buck want to be doing.

The planner would build up a graph of actions ("I need to build X from A, B, C because Y needs it"). In the normal (current) mode of operation the build execution engine would walk the graph and perform each action, possibly relying on cached state.

However, that action graph could also be turned into a set of rules for another build system (Buck, Bazel, etc) to perform the execution, including managing its own cache. 

Crate dependency management spans both to some extent - if you have a dependency on a crate, then you can take an action like download it or check vendored sources, then embed that crate's action graph into this one (sharing any common subgraphs).

Dependencies on non-Rust code could also be handled in the action graph, where the execution engine is responsible for resolving things like "I need openssl". In the current standalone cargo mode, this would still be "invoke autoconf from build.rs", but it could be implemented as "depend on the standard 3rd party openssl".

# Stakeholders/potential contributors
- @jsgf - Facebook, integration with Buck
- @acmcarther - explored [several avenues](https://github.com/bazelbuild/rules_rust/issues/2#issuecomment-303911828) for [integration](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-278088373) with Bazel, seems [open to several options](https://github.com/bazelbuild/rules_rust/issues/2#issuecomment-285191343)
- @davidzchen - [Bazel Rust Rules author](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-277870332)
- @softprops - [interested in helping](https://github.com/bazelbuild/rules_rust/issues/2#issuecomment-303908322)
- @jpakkane - [author of Meson build system](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-277933472), working on multi-language build integration
- @tupshin - interest in [JVM integration](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-278096012)
- @cardoe - Gentoo’s [cargo-build](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-278995178)
- @sholsapp - [Gradle integration](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-279021984)
- @firstyear - [autotools](https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-309326738)
- @joshtriplett - distro integration, declarative native deps
- firefox devs - potential contributors - @froydnj and others
# Meeting Notes


- Rust is basically not usable unless you have access to the crates ecosystem. Therefore, any integration strategy needs to make it easy to incorporate crates from crates.io.
- We’re adding a lot of tools and workflows to the Rust development experience that depend on cargo (ex: RLS, cargo doc, custom subcommands, etc). Any integration strategy shouldn’t prevent developers from using these tools.
- We want to avoid the impression that cargo isn’t a “serious” build tool, and that when you’re ready to get serious, you should use something like bazel
- How far can we get with Cargo.toml being the source of truth for a top-level project?
- In an ideal world:
    - cargo and Cargo.toml handle the Rust code, what they’re good at
    - build systems take care of c++ projects and other external dependencies that they’re good at
    - Render unto Caesar that which is Caesar’s
    - Doesn’t completely solve the problem in workflows that need to know about everything
        - Ex: Developer using Buck wants to run `cargo doc` or `cargo test`. How does that work? This could involve running a command like `bazel prepare` before `cargo test` will work; seems fine.
        - Bazel should know how to build things, and cargo might also know how to build things via `build.rs`, but cargo should also know how to ask bazel where a built artifact is and know to use that instead
- Less ideal:
    - Trying to make build system rules from a Cargo.toml
    - Trying to add a way to specify build system rules in Cargo.toml
- “Library approach” - more declarative way of specifying building than build.rs
    - Could allow people to override pieces of the build process
    - Overriding has to be in .cargo/config to enable changing build processes once and have the same policy across the entire monorepo
    - How do we get crate authors to use this rather than build.rs?

Today you can:

- Specify `links` in Cargo.toml
    - http://doc.crates.io/build-script.html#the-links-manifest-key
- Specify in `.cargo/config` to use the linked thing from somewhere else
    - http://doc.crates.io/build-script.html#overriding-build-scripts
- Is this enough?
    - For native deps?
    - For internal deps?
- It’s a little rough:
    - not fine grained
    - have to conform to how links works with cargo
    - `cargo metadata` doesn’t surface `links` at all
- This needs documented, like everything else with cargo


- Incremental steps
    - Make `cargo metadata` better

AK: I think I’ve understood one fundamental mismatch between Cargo’s and Bazel/Buck monorepo view of the world. Cargo builds (“knows”) a single top level package or workspace. Bazel builds monorepo as a whole, it “knows” about all Cargo projects. So it seems that in Caro+Bazel integration, we want Cargo to be able to produce a *single* lockfile/build plan for the whole monorepo (conceptually, in reality it might be split into several files), to make sure that external dependencies are locked to a single version, and  that internal packages have correct dependencies (That is, if A → C and B → C, then A and B share exactly the same artifacts for C, even if otherwise they are completely different (non-workspaced) Cargo packages).

AK: here are some thoughts on how we can slice “Bazel calls Cargo to build Rust code” into pieces with progressively increased complexity.


## Scenario 1. 

There’s a big monorepo with gazzilion lines of C++. Inside this monorepo, there is a single bit of Rust. This bit is **a single Cargo workspace**, which **doesn’t depend on any native code** (no internal dependencies, and no build scripts for openssl), **uses crates.io dependencies**, and **produces artifacts, used by other Bazel targets**, like binaries, static or shared libraries. This scenario corresponds to “Let’s rewrite a small, focused internal library in Rust”. 

I would say that Cargo already handles Scenario 1 today: namely, you define Bazel rule, which uses source code of the workspace as a single dependency, and calls Cargo to produce the artifacts. 

Concerns:


- Builds are not hermetic because Cargo hits the Internet (https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-278193719).
    This is handled by combination of vendoring and `--frozen`.


- You need `cargo rustc` to pass flags to the compiler to tweak dynamic libraries (
    https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-277816996)
    Needs investigation, but maybe we must expose more knobs here.


- Static libraries always link jemaloc and stuff (https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-278299047)
    This seems bad. Suppose our Cargo workspace produces to static libraries, and then we want to link them both with our C++ binary. Does this leads to linking errors due to duplicate symbols? 


- I don’t know where output artifacts are (https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-277816996)
    There’s a `--message-format=json`, which will print location of artifacts *during cargo build*. It may be more convenient to know artifact locations upfront, we might add `--output` cli option for staticlib, cdylib and bin targets.


## Scenario 2.1

A single workspace, which may depend on **internal native dependencies.** 

Looks like this scenario is more or less handled as well? On Bazel’s side, you declare that your Cargo projects depends on C++ artifacts, and instruct Bazel to put them into known location. On Cargo’s side, you write `build.rs` which takes those artifacts. This might need `bazel prepare` command to make it possible to Cargo directrly, and not via Bazel. 


## Scenario 2.2

A single workspace, which may depend on internal or **external native dependencies** (that is, you would like to vendor rust bindings to openssl). The solution here is the same as for 2.1, except that you need to *override* build scripts of dependencies from crates.io. If I understood @Alex C correctly, this is already possible. This might not be pretty, *but* there are small number of openssl-sys -like crates on crates.io, and we can imagine “The build.rs library” which makes possible to write `build.rs` which can be configured from outside (via env vars, for example), to get native deps from where Bazel puts them. 



## Intermission

So looks like, **if** there’s **a single workspace**, we almost have perfect integration between Bazel and Cargo (there are some rough edges, and someone has to write this integration of course 😃 ). Note that single workspace implies that all Cargo code is build on a single machine. That is, we can’t say “let’s make the whole monorepo a single Cargo workspace” and call it a day. 


## Scenario 3

We have at least **two independent workspaces**. That is, crates from one workspace don’t depend on crates from other workspace. 

Here things get really interesting! First, we can apply solutions from previous scenarios directly. The problem with this approach is that there are two vendor directories, one for each workspace, which is bad, because a) no single place to audit external deps b) duplicated work during build. 

Can we share the vendor directory between both workspaces? I think we can’t really, because this implies that both workspaces must reside in the same filesystem and must be build on a single machine, and we want to be able to distribute Rust compilations. A hypothetical solution here is to have Bazel targets for each crate file of each vendored crate. Then, in the Bazel rule for a workspace, we can specify dependencies on certain crate files and make Cargo use thouse .crates 👋 👋 

This allows to have distributed Rust builds with a single place for vetting external dependencies, however duplicated works during build remains. It can be plugged by sccache to some extent, but of course this is much worse then using Bazel’s artifact caching. 


## Scenario 4

The final boss, several workspaces with interdependencies. 

Just some vague thoughts here 😞 In Bazel, we can specify that one workspace depends on the source code of another workspace. This makes distributing Rust builds impossible. As in scenario 3, we can specify dependency on `.crate` file, and transfer `.crate` files between workspaces via Bazel. This again allows for distribution, but does nothing about duplicated work during building, which we can again try to cover this hole by sccache. 


