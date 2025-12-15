# Cargo build integration plan, 2017-08-03
We have two customers we’re trying to help:


- People who have very custom build setups — distributed building, multiple machines, custom determinism solutions, etc, existing build tools that manage dependencies, native deps etc


- People who have some build system *concerns*, e.g. want to control network usage
    - Willing to allow Cargo to “run the show”, but want more control over certain aspects
    - Want to control a relatively small % of pretty common thing
        - Mirroring, caching, environment hashes, native deps

The needs are very similar, but the first customer needs *everything at once*, whereas the second one generally needs just a couple things that are commonly needed, and are not constrained by some huge build system. The ways to help them early on look quite different, though we want them to converge in the end.

We take a two-pronged approach.

# Prong 1: build plans

We can understand `c``argo build` in terms of several steps (note, this is finer-grained than before!):

| **Step**              | **Conceptual output**                                                                                                                                             | **Additional concerns**             |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| Dependency resolution | Lock file                                                                                                                                                         | Mirrors, offline/local, native deps |
| Build configuration   | Cargo settings per crate in graph                                                                                                                                 | Profiles                            |
| Build lowering        | A series of steps that must be run in sequence, that may include:<br><br>- Planned rustc invocations (may be multiple per crate)<br>- binaries that should be run | Build scripts, plugins              |
| Build execution       | Compiled artifacts                                                                                                                                                | Caching                             |

Each of these outputs is the conceptual input into the next step.


    $ cargo compile --extern other-crate=/path/to/artifacts # analogy

Interpretation of Aturon’s CSS analogy:


    body {
        openssl: /over/here;
        
        .serde {
            feature: json;
            features: macros;
        }
        
        .ring {
            openssl: /somewhere/else;
        }
    }

Some build systems are very strict about when dependencies can be added; that is, all dependencies must be added before the compilation stage

Reasons build scripts can’t be run in with dependency resolution:


1. Build scripts have ordinary dependencies, want to follow the ordinary dependency resolution process
2. Different outputs for different platforms
# Prong 2: extensibility points

(Note: we should talk about “extensibility points” versus “Cargo as a library”, and the “[midlayer mistake](https://lwn.net/Articles/336262/)”.)


- multiple registries/alternative registries - “how do you obtain crates”
    - Mirrors of crates.io (build offline, avoid network access)
        - Either as-needed or pre-fetching everything
        - Vendored for a particular package/build, or systematically integrated into a packaging system (e.g. Linux distributions)
        - Always using a mirror
        - Having multiple mirrors available for redundancy if one is unavailable
    - Modified versions of crates.io crates
        - Out of scope for **this** discussion, good to discuss as a standalone topic
    - Internal, proprietary crates
    - A synthesis of some combination of the above
    - Design work needed: API of the index https://github.com/rust-lang/rfcs/pull/2006#issuecomment-313484860
- making vendoring more first-class/coherent (works fairly reliably)
- caching
    - monorepo caching story — i.e. want to cache into the monorepo
    - local machine caching that’s shared among all projects
    - sccache — distributed caching
- expressing an “environment hash”
    - used to inform the cache
    - includes Rust version, isolation environment, OS cfg, native dependencies
    - relevant to nix’s concerns
- First-class native dependencies (MOST IMPORTANT TO ALEX)
    - replace the build script with a pointer to the native deps
    - Josh suggested separating vendored non-Rust source from the -sys crate that wants it, and supporting downloading it *separately* from crates.io
        - YK: I think this is fine but it involves getting into the space of debhelper, which is fine but a big project. I think Josh has the experience here though 😃 
    - `link=`
# The long game

TLDR: merge these two paths!


# Action item:
- Roadmap eRFC before RFI period
    - Broad strokes that enable experimentation
- Make sure we state clearly that solutions for Prong 2 need to take Prong 1 into account and vice versa
- Blog post recapping Cargo Roadmap RFC after accepted
# FB recap (wycats)
- FB is using a fair amount of Rust already
- The setup they’re using:
    - There’s a repo with a global Cargo.toml listing crates from crates.io
    - If you want to add a dep, you add it to that global Cargo.toml and do a `cargo update`
    - Then those deps are available to FB-internal projects
    - Analogous to having a workspace for all of FB
- They current have a strong split between internal and external deps
    - wycats argued, and convinced them, that there’s not an *intrinsic* split here
- Constraints:
    - Big takeaway was: `cargo build` should not be able to reach outside of the current directory unless you specifically tell it about other locations
        - i.e. could have dependencies on files “invisible” to the larger build system
        - wycats argued successfully that `--extern` should cover this
- Then discussion around whether internal projects should have a `Cargo.toml` file
    - They agreed that there are ways to “model” having a `Cargo.toml` that doesn’t hurt their other goals
    - There are two competing goals here:
        - Buck should be the sole source of truth for building, which may include a “prepare” step that involves running Cargo to generate Buck descriptions
            - They are very worried about having to “sync” Cargo and Buck descriptions of a build
        - They want things like RLS to work, which currently depend on `Cargo.toml`
    - Solution proposed by sid0:
        - Create a new key in `Cargo.toml` that says, please get the info you’d normally get from a plugin instead, e.g. one that reads a Buck file and creates a json blob equivalent to the structure you get after reading a normal `Cargo.toml`
        - Thus, all normal Cargo subcommands will work, and will just delegate the manifest step
    - Proposal by wycats
        - Instead have Cargo generate a Buck file whenever a subcommand is used, much like how we generate lockfiles today
        - This was less appealing to FB but seems to satisfy the constraints
            - Josh: there seems to be a *very* strong cultural norm that you should write Buck files

