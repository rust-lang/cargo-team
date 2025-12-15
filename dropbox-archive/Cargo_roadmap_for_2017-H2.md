# Cargo roadmap for 2017-H2

# Status as of 2017-10-16
## Review of Sept goals
[x] Multiregistry RFC in FCP by 18th
[ ] Multiregistry implementation in Cargo landed
    - Waiting on test cases for boats’s PR
    - Another PR from cswindle
    - Other improvements waiting til boats’ PR is in; impl period issues coming!
[x] Land pub/priv RFC by 18th
[ ] Get started on Cargo side of pub/priv implementation
    - aturon to talk to boats about this
[x] Landed eRFC for build system integration
[ ] Landed (e)RFC for metabuild
    - Cargo schema versioning worked out at RustFest
        - Should never have to write a schema version yourself; do it all based on detection of features used, give good error messages if a newer Cargo is needed
        - Designed so that old Cargo should be able to read new schemas
        - No worse than doing nothing
        - Josh to upload RFC
    - Metabuild RFC
        - Worked through interface with Alex
        - How to deal with multiple crates using metabuild
        - First version of RFC will be `metabuild = [list of crate names]` and will disallow also having a `build.rs` file
        - TBD: error-handling strategy
[x] Land mdBook version of Cargo docs
[x] Create labels in cargo’s issue tracker for candidate issues for impl period and issues that are ready for the impl period
## Planning
- Multiple registries
- Pub/priv
- Metabuild
- Build system integration
- Cargo docs
    - matklad to look into cargo doc interaction with importing extra Rust code
- Profiles/features revision
# Status as of 2017-08-31
## Goals for end of Sept
- Multiregistry RFC in FCP by 18th
- Multiregistry implementation in Cargo landed
- Land pub/priv RFC by 18th
- Get started on Cargo side of pub/priv implementation
- Landed eRFC for build system integration
- Landed (e)RFC for metabuild
- Land mdBook version of Cargo docs
- Create labels in cargo’s issue tracker for candidate issues for impl period and issues that are ready for the impl period
## Checkin
- Multiple registries
    - Draft RFC almost done
        - Posted ASAP
        - In FCP prior to Sept 18
    - Implementation in progress
        - Landed by end of Sept
- Pub/priv
    - in FCP any day now
- Metabuild
- Build system integration
- Security policies, TOS, SLA
- Cargo docs
    - Cookbook-style docs
    - Reference
    - Guide
- Mentoring
- Issue tracker is in bad shape
- Automatic features
    - Blocked on looking at features more broadly
    - Not gonna do this this year
- On the radar: profiles
    - Strong interaction with build system integration
# Original plan
- Multiple registries
- Pub/priv
- Build system integration
- Security policies, TOS, SLA
- Cargo docs
    - Cookbook-style docs
    - Reference
    - Guide
- Mentoring
- Issue tracker is in bad shape
- Automatic features
- On the radar: profiles
    - Also, possibly optimizing specific deps
    - YK: I’d love to chat quickly about this to give you the starting point for how we got where we are today, but I completely agree it’s kind of a stagnated MVP

