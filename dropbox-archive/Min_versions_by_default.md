# Min versions by default

There’s an argument that libraries and apps want the same version strategy, regardless of the strategy itself. If I write library X, I would ideally want to use the dependencies versions that an app would use, to make sure the library actually works fine with them. To put this another way, if libraries use the minimum version, and apps use maximum version, an app might get broken if a minor update of the dependency of the library introduces a bug, and the library hits it. 


Alex’s thoughts:

- Minimum version resolution as an *option* sounds great to me, I’m mostly just talking about the idea of having it on by *default* 
- It’s fundamentally impossible to be perfect here. There’s no way to *guarantee* that Cargo.toml is actually correct. Our goal is to have Cargo.toml be “more correct” in many situations, such as accidentally relying on a newer API. This naturally leads to the conclusion to me that hard errors are probably out here, but rather just warnings and heuristics.
- If we were to have libraries choose minimum versions by default this creates a sort of “schism” between application developers and library developers. These are now distinct modes where dependency management is quite different. For example as I’m working on a library everything could be working but when I switch to the application to plug that library in everything could stop working. I may have not realized that the behavior of a newer version was subtly different of some dep and was getting an older version on the library. This is of course true for the reverse as well, working on a lib could work and during integration it doesn’t b/c the transitive deps are different.
- As a library author I personally want to be working against the updated versions of libraries. I, myself, am an “application” developer when I’m working on a library, especially when writing tests and such. My own CI often wants lots of bug fixes! I don’t think it’s clear cut that the minimum resolution is the right choice 100% of the time here.
- In practice the real problem that seems most likely is “you accidentally used a feature from a newer version”. Changing the defaults in Cargo’s graph resolution seems like a too-big hammer for this problem, whereas an optional flag for CI and publication seems like a good fit.
- Overall “min by default” seems backwards from the defaults we want to encourage. We don’t encourage you to minimize your rustc version by default, why would we want you to minimize your dep versions? This is definitely a “this just feels wrong” thought but it seems odd to go the other way from what seems to be the best practice of “use up to date libraries”.

Prerequisites before even considering this change:


- Cargo needs to be able to distinguish between applications and libraries
- Commands like `cargo update` need to modify Cargo.toml, otherwise rewriting everything is unlikely to be ergonomically viable.

Questions:


- What happens on publish for:
    - packages that are *both* a library and a binary? ex: bindgen
    - packages that are *only* binaries? ex: cargo plugins
- This doc is mostly talking about the current state of version resolution, but we got into this because of public/private dependencies. How do these thoughts fit into a future world where public/private dependencies exist?
- npm also heavily trusts semver, do they have this problem?

