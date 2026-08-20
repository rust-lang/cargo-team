#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
name = "cargo-new-release"
version = "0.1.0"
edition = "2024"

[dependencies]
anyhow = "1.0"
dialoguer = "0.9"
regex = "1.12"
semver = "1.0"
time = { version = "0.3", features = ["formatting", "macros"] }
---
use anyhow::{anyhow, bail, Context, Result};
use dialoguer::Confirm;
use regex::Regex;
use semver::Version;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::process::{exit, Command, Stdio};
use time::macros::{date, format_description};

const CHANGELOG_PATH: &str = "src/doc/src/CHANGELOG.md";

trait CommandExt {
    fn git(args: &str) -> Command;
    fn run_stdout(&mut self) -> Result<String>;
    fn display_args(&self) -> String;
    fn run_success(&mut self) -> Result<bool>;
}

impl CommandExt for Command {
    fn git(args: &str) -> Command {
        let mut command = Command::new("git");
        command.args(args.split_whitespace());
        command
    }

    fn run_stdout(&mut self) -> Result<String> {
        self.stdout(Stdio::piped());
        let output = self.output().with_context(|| {
            format!(
                "failed to spawn `{} {}`",
                self.get_program().to_string_lossy(),
                self.display_args()
            )
        })?;
        if !output.status.success() {
            bail!(
                "failed to run `{} {}`: {}",
                self.get_program().to_string_lossy(),
                self.display_args(),
                output.status
            );
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    fn display_args(&self) -> String {
        self.get_args()
            .map(OsStr::to_string_lossy)
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn run_success(&mut self) -> Result<bool> {
        let status = self.status().with_context(|| {
            format!(
                "failed to spawn `{} {}`",
                self.get_program().to_string_lossy(),
                self.display_args()
            )
        })?;
        if !matches!(status.code(), Some(0 | 1)) {
            bail!(
                "failed to run `{} {}`: {}",
                self.get_program().to_string_lossy(),
                self.display_args(),
                status
            );
        }
        Ok(status.success())
    }
}

/// Checks that the repo is ready to go.
fn check_status() -> Result<()> {
    let root = Command::git("rev-parse --show-toplevel").run_stdout()?;
    env::set_current_dir(root)?;
    if !Command::git("diff-index --quiet HEAD .").run_success()? {
        eprintln!("Working tree has changes.");
        Command::git("status --porcelain").run_success()?;
        if !Confirm::new()
            .with_prompt("Do you want to continue?")
            .default(false)
            .interact()?
        {
            exit(1);
        }
    }
    let upstream = Command::git("config remote.upstream.url").run_stdout()?;
    if !upstream.ends_with("rust-lang/cargo.git") {
        bail!("upstream does not appear to be rust-lang/cargo, was: {upstream}");
    }
    let origin = Command::git("config remote.origin.url").run_stdout()?;
    if !origin.ends_with("/cargo.git") {
        bail!("origin does not appear to be cargo, was: {origin}");
    }
    Ok(())
}

/// Creates the `version-bump` branch.
fn create_branch() -> Result<()> {
    if !Command::git("fetch upstream --tags").run_success()? {
        bail!("failed to fetch upstream");
    }
    if Command::git("show-ref --verify --quiet refs/heads/version-bump").run_success()? {
        eprintln!("info: replacing version-bump branch");
    }
    eprintln!("info: creating version-bump branch");
    if !Command::git("checkout -B version-bump upstream/master").run_success()? {
        bail!("failed to create branch");
    }
    if !Command::git("config branch.version-bump.remote origin").run_success()? {
        bail!("failed to set remote origin");
    }
    if !Command::git("config branch.version-bump.merge refs/heads/version-bump").run_success()? {
        bail!("failed to set branch merge");
    }
    Ok(())
}

/// Updates the version in `Cargo.toml`.
fn bump_version_toml() -> Result<Version> {
    let mut manifest = fs::read_to_string("Cargo.toml").context("failed to read Cargo.toml")?;
    let version_start = manifest
        .find("\nversion = \"")
        .ok_or_else(|| anyhow!("could not find package version in Cargo.toml"))?
        + 12;
    let version_len = manifest[version_start..]
        .find('"')
        .ok_or_else(|| anyhow!("could not find end of package version"))?;
    let version = Version::parse(&manifest[version_start..version_start + version_len])?;
    if version.major != 0 {
        bail!("expected a 0.x Cargo version, found {version}");
    }
    let next_version = Version::new(0, version.minor + 1, 0);
    manifest.replace_range(
        version_start..version_start + version_len,
        &next_version.to_string(),
    );
    fs::write("Cargo.toml", manifest)?;
    Ok(next_version)
}

/// Waits for the user to manually validate.
fn wait_for_inspection() -> Result<()> {
    eprintln!("Check for tests or rustc probing (usually target_info.rs) that can be updated.");
    if !Confirm::new()
        .with_prompt("Ready to commit?")
        .default(true)
        .interact()?
    {
        exit(1);
    }
    Ok(())
}

/// Commits the version bump.
fn commit_bump(next_version: &Version) -> Result<()> {
    if !Command::git("commit -a -m")
        .arg(format!("Bump to {next_version}"))
        .run_success()?
    {
        bail!("failed to commit");
    }
    Ok(())
}

/// Modifies the changelog to include stubs for the given version.
fn prep_changelog(next_version: &Version, rust_repo: &str) -> Result<()> {
    let beta_minor_version = next_version.minor - 2;
    if !Command::git("fetch upstream --tags")
        .current_dir(rust_repo)
        .run_success()?
    {
        bail!("failed to fetch rust upstream");
    }
    let last_beta_line = Command::git("ls-tree upstream/beta src/tools/cargo")
        .current_dir(rust_repo)
        .run_stdout()?;
    let mut parts = last_beta_line.split_whitespace();
    if parts.next() != Some("160000") || parts.next() != Some("commit") {
        bail!("unexpected Cargo submodule entry: {last_beta_line}");
    }
    let last_beta_hash = parts
        .next()
        .ok_or_else(|| anyhow!("Cargo submodule entry did not contain a hash"))?;
    if parts.next() != Some("src/tools/cargo") || parts.next().is_some() {
        bail!("unexpected Cargo submodule entry: {last_beta_line}");
    }

    let beta_version = format!("rust-1.{beta_minor_version}.0");
    let last_branch_line =
        Command::git(&format!("show-ref upstream/{beta_version}")).run_stdout()?;
    let last_branch_hash = last_branch_line
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow!("Cargo beta ref did not contain a hash"))?;

    if last_beta_hash != last_branch_hash {
        eprintln!(
            "warning: rust-lang/rust beta branch hash {last_beta_hash} does not equal \
             rust-lang/cargo upstream/{beta_version} hash {last_branch_hash}"
        );
        eprintln!(
            "This may happen if changes are pushed to {beta_version} shortly after the beta \
             branch was created. Please carefully inspect whether that happened."
        );
        if !Confirm::new()
            .with_prompt("Do you want to continue?")
            .default(true)
            .interact()?
        {
            exit(1);
        }
    }
    let start_of_beta_short_hash = &last_beta_hash[..8];

    let changelog = fs::read_to_string(CHANGELOG_PATH)
        .with_context(|| format!("failed to read {CHANGELOG_PATH}"))?;
    let head_re = Regex::new(r"([a-f0-9]+)\.\.\.HEAD")?;
    let matches = head_re.captures_iter(&changelog).collect::<Vec<_>>();
    if matches.len() != 2
        || matches[0].get(0).unwrap().as_str() != matches[1].get(0).unwrap().as_str()
    {
        bail!("expected two matching `HASH...HEAD` links in {CHANGELOG_PATH}");
    }
    let beta_hash_start = matches[0].get(1).unwrap().as_str();
    let mut changelog = head_re
        .replace_all(&changelog, format!("{beta_hash_start}...{beta_version}"))
        .into_owned();

    let master_prs = find_prs(&changelog, start_of_beta_short_hash, "upstream/master")?;
    let beta_prs = find_prs(
        &changelog,
        beta_hash_start,
        &format!("upstream/{beta_version}"),
    )?;

    let added_idx = changelog
        .find("### Added\n")
        .ok_or_else(|| anyhow!("could not find `### Added` in {CHANGELOG_PATH}"))?;
    changelog.insert_str(added_idx, &format_pr_links(&beta_prs));

    if !changelog.starts_with("# Changelog\n") {
        bail!("{CHANGELOG_PATH} did not start with `# Changelog`");
    }
    changelog.insert_str(
        12,
        &format!(
            "\n## Cargo 1.{} ({})\n\
             [{start_of_beta_short_hash}...HEAD](https://github.com/rust-lang/cargo/compare/{start_of_beta_short_hash}...HEAD)\n\
             \n\
             {}\n\
             \n\
             ### Added\n\
             \n\
             ### Changed\n\
             \n\
             ### Fixed\n\
             \n\
             ### Nightly only\n\
             \n",
            next_version.minor - 1,
            next_version_date(next_version)?,
            format_pr_links(&master_prs),
        ),
    );
    fs::write(CHANGELOG_PATH, changelog)?;

    let master_urls = master_prs
        .iter()
        .map(|(_, url, _)| url.as_str())
        .collect::<Vec<_>>();
    open_browser(&master_urls)?;
    eprintln!(
        "Update the nightly version 1.{}.0 and come back when finished.",
        next_version.minor - 1
    );
    if !Confirm::new()
        .with_prompt("Ready to continue?")
        .default(true)
        .interact()?
    {
        exit(1);
    }

    let beta_urls = beta_prs
        .iter()
        .map(|(_, url, _)| url.as_str())
        .collect::<Vec<_>>();
    open_browser(&beta_urls)?;
    eprintln!("Update the beta version 1.{beta_minor_version}.0 and come back when finished.");
    if !Confirm::new()
        .with_prompt("Ready to commit?")
        .default(true)
        .interact()?
    {
        exit(1);
    }
    Ok(())
}

fn format_pr_links(prs: &[(u32, String, String)]) -> String {
    prs.iter()
        .map(|(number, url, description)| format!("- {description} \n  [#{number}]({url})\n"))
        .collect()
}

fn open_browser(urls: &[&str]) -> Result<()> {
    if !Command::new("/Applications/Firefox.app/Contents/MacOS/firefox")
        .arg("-url")
        .args(urls)
        .run_success()?
    {
        bail!("failed to open Firefox");
    }
    Ok(())
}

fn find_prs(changelog: &str, start: &str, end: &str) -> Result<Vec<(u32, String, String)>> {
    let log = Command::git(&format!("log --first-parent {start}...{end}")).run_stdout()?;
    let commits = commits_in_log(&log)?;
    let (duplicates, new): (Vec<_>, Vec<_>) = commits
        .into_iter()
        .partition(|(pr, _, _)| changelog.contains(&format!("[#{pr}]")));
    for (pr, _, _) in duplicates {
        eprintln!("skipping PR #{pr}, already documented");
    }
    Ok(new)
}

/// Returns `(PR number, PR URL, PR description)` tuples.
fn commits_in_log(log: &str) -> Result<Vec<(u32, String, String)>> {
    let commit_re = Regex::new("(?m)^commit ")?;
    let merge_re = Regex::new(r"(?:Auto merge of|Merge pull request) #([0-9]+)|\(#([0-9]+)\)$")?;
    commit_re
        .split(log)
        .filter(|commit| !commit.trim().is_empty())
        .filter_map(|commit| {
            let hash = commit.split_whitespace().next()?;
            let mut lines = commit
                .lines()
                .filter(|line| !line.trim().is_empty() && line.starts_with(' '))
                .map(str::trim);
            let first = lines.next()?;
            let captures = match merge_re.captures(first) {
                Some(captures) => captures,
                None => {
                    eprintln!("could not find a PR number in line: {first}\nhash: {hash}");
                    return None;
                }
            };
            let (capture, description) = match (captures.get(1), captures.get(2)) {
                (Some(capture), _) => (capture, lines.next().unwrap_or_default().to_owned()),
                (_, Some(capture)) => {
                    let mut title = first.to_owned();
                    let range = (capture.range().start - 2)..=capture.range().end;
                    title.replace_range(range, "");
                    (capture, title.trim_end().to_owned())
                }
                (None, None) => unreachable!(),
            };
            let number = match capture.as_str().parse::<u32>() {
                Ok(number) => number,
                Err(error) => return Some(Err(error.into())),
            };
            Some(Ok((
                number,
                format!("https://github.com/rust-lang/cargo/pull/{number}"),
                description,
            )))
        })
        .collect()
}

/// Commits the changelog update.
fn commit_changelog(next_version: &Version) -> Result<()> {
    if !Command::git("commit -a -m")
        .arg(format!("Update changelog for 1.{}", next_version.minor - 2))
        .run_success()?
    {
        bail!("failed to commit changelog");
    }
    Ok(())
}

/// Pushes the branch and opens the new pull request page.
fn create_pr(next_version: &Version) -> Result<()> {
    if !Command::git("push").run_success()? {
        bail!("failed to push");
    }
    let origin = Command::git("remote get-url origin").run_stdout()?;
    let user_re = Regex::new(r"([a-zA-Z0-9-]+)/cargo")?;
    let username = &user_re
        .captures(&origin)
        .ok_or_else(|| anyhow!("could not determine GitHub username from {origin}"))?[1];
    open_browser(&[&format!(
        "https://github.com/{username}/cargo/pull/new/version-bump"
    )])?;
    eprintln!("title:\nBump to {next_version}, update changelog");
    Ok(())
}

fn next_version_date(next_version: &Version) -> Result<String> {
    let first = date!(2015 - 05 - 15);
    let next_days = ((next_version.minor - 1) * 42) as i64;
    let next_date = first + time::Duration::days(next_days - 1);
    Ok(next_date.format(format_description!("[year]-[month]-[day]"))?)
}

fn run() -> Result<()> {
    let rust_repo = env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("expected path to rust repo as first argument"))?;
    check_status()?;
    create_branch()?;
    let next_version = bump_version_toml()?;
    wait_for_inspection()?;
    commit_bump(&next_version)?;
    prep_changelog(&next_version, &rust_repo)?;
    commit_changelog(&next_version)?;
    create_pr(&next_version)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        exit(1);
    }
}
