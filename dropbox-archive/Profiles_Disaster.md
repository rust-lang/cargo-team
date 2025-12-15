# Profiles Disaster


# Summary

The “profiles” feature of Cargo has some really great sides to it, but it also has accumulated some problems other time. The core one is that there are too many different profiles (dev, release, test, bench, doc), and almost nobody understands which profile is activated when. 

We may want to do something about profiles sooner rather then later because they effectively
block some features, and because “fixing” the profiles probably can’t be done by extending the current system. 


# Problems In Depth

Which profile is used for `cargo test`? More than one, actually: `cargo test` will build the `[lib]` crate of the package with `dev` profile, the `[test]` crates with `test` profile (and link it with the `dev` `[lib]`), and then it will build `[lib]` again in `test` profile for unit tests! Logically, `cargo test --release` builds everything in `bench` profile.

That is, if you want do implement a Cargo feature which does something great in the `X` profile, you are out of luck, because “being in `X` profile” is a rather unpredictable condition! The examples of such features are:


- Optimizing dependencies even in `dev` profiles: https://github.com/rust-lang/cargo/issues/1359
- Activating features based on profiles: https://github.com/rust-lang/rfcs/pull/1956
- Custom profiles: https://github.com/rust-lang/cargo/issues/2007

It also seems to me that there are some problems on the implementation side of things: Cargo sometimes uses profiles to find out what operation it is doing right now, and this somewhat ties together Cargo’s implementation details and user visible profiles (a relevant issue is https://github.com/rust-lang/cargo/issues/4140). 


# Today’s Profiles, The Good Parts

There are however some absolutely great things about the current system:

- you don’t need to specify raw compiler flags, 
- `--release` flag is simple to understand and works great in practice,
- the `dev`/`release` behavior you get by default is most reasonable. 


# Way Forward

Honestly, I don’t know what is the way forward here. One can imagine a more general `profiles 2.0` system, which allows to specify arbitrary mappings from crates to compiler’s flags, and which you can opt-into instead of the current profiles. 

There’s one more though that I’ve got while writing this up. It seems to me that if we could just drop **all** existing profiles, except the `dev` and `release` (which correspond to `--relase` flag exactly), then we’d have a very lean and simple to understand system, which should be easy to extend to allow per-dependency customization and such. 


# Meeting notes 


- Profiles should only modify flags passed to rustc invocations
- Which rustc invocations happen should be controlled by some other mechanism
- Cargo implementation details that currently rely on profiles should read the profile config once and check the relevant configuration options once instead of the profiles
- Are historical reasons for how this system came to be still valid today?
- Are there better defaults we could choose instead of what we have now?
    - esp since we have `cargo check` now (where the profile is moot)
    - `-o 1` optimizations by default
    - add `cargo debug`
    - `cargo doc` should be built on top of `cargo check` so profile is moot
- Custom profiles
- Profile inheritance
- `--release` should be shorthand for `--profile=release`
- can’t set `panic=abort` for test mode
- which of the current profile options should be global, and which should be controllable per library (once we support that)?
- can’t currently say from the toplevel which dependencies should be compiled as which type (rlib, dylib, etc)
- everything is terrible and a big ball of spaghetti tangled in yarn

How badly do we need custom profiles right now?

- not at all
- but for backwards compatibility?
- don’t want to close the door on them though
- tying features to profiles

We don’t want to add more stuff to profiles until profiles are designed in a better way…


# Action plan


- matklad to develop a plan to deprecate any profile aside from debug, dev, release and create an RFC to get feedback on usecases we don’t know about
    - dev and test should be the same 99% of the time today
- Then plaster those profiles all over

THERE ARE THREE PROFILES

![](https://d2mxuefqeaa7sj.cloudfront.net/s_4BFC3EE12E69A60E3594A1ED1D6CCDC2F265CC0EA38A85CE960BB64829CF78C9_1499968503679_hrus_ex_picards_4_lights_dd.jpg)














Some profiles related issues:


- https://github.com/rust-lang/cargo/issues/4251 (my doctests work only with -O3, how do I set this via profiles?)
-  https://github.com/rust-lang/cargo/issues/4240 (Guess, what profile is used by `cargo bench` `--``no-run`? `--release` of course!)



# Profiles 2.0 RFC
# Summary

Deprecate and remove all Cargo profiles except for dev and release in order to have a simple and solid foundation for making profiles more powerful.

# Motivation

The “profiles” feature of Cargo allows crate authors to control certain compiler options, like optimization level or presence of debug assertions. It has really great sides to it, but it also has accumulated some problems other time. The core one is that there are too many different profiles (dev, release, test, bench, doc), and it is difficult to undestand which profile is activated for a particular Cargo invocation. For example, `cargo test` will build the lib crate of the package with dev profile, the test crates with test profile (and link it with the dev lib), and then it will build lib again in test profile for unit tests! In contrast, `cargo test --release` builds everything in bench profile.

This situation is problematic for two reasons:


- It's difficult to use non-default profiles, because knowing which profile should be modified is tricky.
- Some Cargo features which depend on profiles are blocked:
    - [#1359](https://github.com/rust-lang/cargo/issues/1359) optimizing dependencies by default even in dev profile
    - Using different optimization flags for different dependencies and workspace members
    - [#1956](https://github.com/rust-lang/rfcs/pull/1956) enabling features based on the current profile
    - [#2007](https://github.com/rust-lang/cargo/issues/2007) ability to define custom profiles
# Detailed design
## Core issues

There are two reasons why current system can be described as "surprising" and "complex":


- Different profiles may be used simultaneously for a single Cargo command, as happens with `cargo test` example.
- Profiles try to encode a high level Cargo operation, like testing, or benchmarking, or documenting, which does not work really great because `--release` flag makes sense for almost all operations. This impedance mismatch is demonstrated by `cargo test --release`, which uses the bench profile.
## Profiles 2.0

It is possible to create a much simpler system, Profiles 2.0. It consists only of two profiles, dev and release, and the release profile corresponds precisely to the `--release` flag. That is, `cargo test` builds all local crates and dependencies in the dev profile, and `cargo test --release` uses the release profile for everything. The exception is `cargo bench`, which uses the release profile by default.

This proposal can be naturally extended to support more features. For example, supporting custom profiles can be implemented as `--profile name` argument, which makes `--release` flag a synonym for `--profile release`. Note that this is not really feasible with profiles 1.0, because `--release` does not always mean the release profile.

It is also becomes possible to customize profile options per package withing a single profile. Strawman syntax for Cargo.toml looks like this:


    [profiles.dev]
    opt-level = 1
    
    [profiles.dev.dependencies]
    opt-level = 2
    
    [profiles.dev.package.foo]
    opt-level = 3
    debug_assertions = false

Note that this RFC itself does not propose to implement any extensions.

## Road to Profiles 2.0

Because profiles are relevant only for the leaf crate (that is, nothing on crates.io can be affected by changes in profiles), and because profiles affect only non-functional properties of code, we probably can move to Profiles 2.0 rather aggressively. Namely, it should be feasible to just switch to Profiles 2.0, adding warnings if any non-release non-dev profile is mentioned in Cargo.toml.

# How We Teach This

Cargo documentation and the Rust Book must be updated to not mention profiles other then dev and release. A warning should be produce if an old profile is detected in Cargo.toml.

# Drawbacks

The big drawback is that this is formally a breaking change. This is ameliorated by several facts though:


- Dependencies on crates.io can't be affected.
- Profiles affect almost exclusively non-functional properties of code.
- It is expected that in practice all profiles are close to either dev or release anyway.
# Alternatives

One alternative is to implement Profiles 2.0 using another name in Cargo.toml, and deprecate, but do not change current implementation. Another alternative is to wait until epochs or similar mechanism is implemented, to make the switch more opt-in.

The main advantage of these alternatives is that the old system stays intact, and all old code works exactly the same. The main disadvantage of these alternatives is that the old system must be supported indefinitely, which places a significant burden on Cargo implementation.

# Unresolved questions
- What current uses of profiles are not covered by this proposal?
- What is the optimal path to Profiles 2.0?
- Profiles include `rpath` and `panic` settings, which probably don't belong there. How these settings work with Profiles 2.0?
- Should we change the defaults for profiles? Perhaps the dev profile should use `--opt-level 1`?
- Do we need only two profiles? Perhaps we need a profile per optimization level?

