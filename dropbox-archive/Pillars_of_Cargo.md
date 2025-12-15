# Pillars of Cargo

## Pillars of Cargo

[Blog post](https://blog.rust-lang.org/2016/05/05/cargo-pillars.html)

“Cargo is the compiler for most users” 

- we want all of the standard workflows to work out of the box.
- we want to give people the ability to step outside of these conventions, but through a layered approach, rather than through knobs
    - Knobs create a large number of combinations that are hard to understand/predict
    - Layered approach is more of a modular/abstraction approach
        - e.g. `cargo build` `--``release` is a standard workflow, but you can customize things like the optimization level underneath. But other tools/people can still use the standard workflow, without knowing about that customization.
            - That’s as opposed to adding a top-level flag
        - `build.rs` is another example
    - Orthogonality: design separate primitives not knobs (also see [Matz on Orthogonal vs. Harmonious](http://www.artima.com/intv/rubyP.html))
    - Interesting examples
        - link flags :-1: vs. profiles :+1:
        - crate type field in Cargo.toml :-1:
        - Providing a hash for us to use in our caching :+1: (primitive)


1. Building, testing, and running projects [applications] should be predictable across environments and over time.
    
    > Once a project successfully compiles on one machine, subsequent compiles across machines and environments will use exactly the same source code.
        
    In particular, the lock file contains a complete “fingerprint” of the build when all deps come from a crate index
    
2. To the extent possible, indirect dependencies should be invisible to application authors.
    
    > if the change you made was unrelated to another dependency, it shouldn’t change.
        
3. Cargo should provide a shared workflow for the Rust ecosystem that aids the first two goals.
    
    > Cargo defines a common set of conventions and workflows that operate precisely the same way across the entire Rust ecosystem on all platforms
        
    > By standardizing what it means to build and configure a package, Cargo can apply all of these configuration choices to your direct dependencies and indirect dependencies.
        
4. As a rule, Cargo attempts to minimize the effects of intentional changes to direct dependencies
5. Cargo makes managing dependencies easier (no objections) and consistent (many objections)
6. We want to encourage maintaining dependencies with their current versions, rather than diverging to local versions that drift with changes that aren’t upstreamed and that don’t update with upstream changes.


## 
## Rust should integrate easily into large build systems

From https://github.com/rust-lang/rust-roadmap/issues/12#issuecomment-279788670:

When you use cargo to build a project, it’s responsible for these parts, which could be separated into different phases:


- crate dependency management
- a build planner
- a build execution engine


## Negapillars of cargo

Don’t want cargo to turn into maven or make.


