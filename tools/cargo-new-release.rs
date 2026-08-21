#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
name = "cargo-new-release"
version = "0.1.0"
edition = "2024"

[dependencies]
anyhow = "1.0"
clap = { version = "4.6", features = ["derive"] }
regex = "1.12"
semver = "1.0"
time = { version = "0.3", features = ["formatting", "macros"] }
---
use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use regex::Regex;
use semver::Version;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};
use time::macros::{date, format_description};

const CHANGELOG_PATH: &str = "doc/book/src/CHANGELOG.md";

#[derive(Debug, Parser)]
#[command(name = "cargo-new-release")]
#[command(bin_name = "cargo-new-release")]
#[command(about = "Prepare Cargo's next version and changelog commits")]
struct Cli {
    /// Path to the rust-lang/cargo checkout to update.
    #[arg(long, value_name = "PATH")]
    cargo_repo: PathBuf,

    /// Remote in the Cargo checkout that points to rust-lang/cargo.
    #[arg(long, default_value = "origin", value_name = "REMOTE")]
    cargo_remote: String,

    /// Path to a rust-lang/rust checkout used to inspect the beta submodule.
    #[arg(long, value_name = "PATH")]
    rust_repo: PathBuf,

    /// Remote in the Rust checkout that points to rust-lang/rust.
    #[arg(long, default_value = "origin", value_name = "REMOTE")]
    rust_remote: String,

    /// Local branch to create for the release commits.
    #[arg(long, default_value = "version-bump", value_name = "BRANCH")]
    branch: String,
}

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
fn check_status(cargo_repo: &Path, cargo_remote: &str) -> Result<()> {
    let root = Command::git("rev-parse --show-toplevel")
        .current_dir(cargo_repo)
        .run_stdout()?;
    env::set_current_dir(root)?;
    let status = Command::git("status --porcelain").run_stdout()?;
    if !status.is_empty() {
        bail!("Cargo checkout has uncommitted changes:\n{status}");
    }
    let remote = Command::git(&format!("remote get-url {cargo_remote}")).run_stdout()?;
    if !remote.ends_with("rust-lang/cargo.git") {
        bail!("{cargo_remote} does not appear to be rust-lang/cargo, was: {remote}");
    }
    Ok(())
}

/// Creates the release branch.
fn create_branch(cargo_remote: &str, branch: &str) -> Result<()> {
    if !Command::git(&format!("fetch {cargo_remote} --tags")).run_success()? {
        bail!("failed to fetch {cargo_remote}");
    }
    if Command::git(&format!("show-ref --verify --quiet refs/heads/{branch}")).run_success()? {
        eprintln!("info: replacing {branch} branch");
    }
    eprintln!("info: creating {branch} branch");
    if !Command::git(&format!("checkout -B {branch} {cargo_remote}/master")).run_success()? {
        bail!("failed to create branch");
    }
    Ok(())
}

/// Updates the version in `Cargo.toml` and `Cargo.lock`.
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
    if !Command::new("cargo")
        .args(["update", "--workspace"])
        .run_success()?
    {
        bail!("failed to update Cargo.lock");
    }
    Ok(next_version)
}

/// Commits the version bump.
fn commit_bump(next_version: &Version) -> Result<()> {
    if !Command::git("commit -a -m")
        .arg(format!("chore: bump to {next_version}"))
        .run_success()?
    {
        bail!("failed to commit");
    }
    Ok(())
}

/// Modifies the changelog to include stubs for the given version.
fn prep_changelog(
    next_version: &Version,
    rust_repo: &Path,
    cargo_remote: &str,
    rust_remote: &str,
) -> Result<()> {
    let beta_minor_version = next_version.minor - 2;
    if !Command::git(&format!("fetch {rust_remote} --tags"))
        .current_dir(rust_repo)
        .run_success()?
    {
        bail!("failed to fetch {rust_remote}");
    }
    let last_beta_line = Command::git(&format!("ls-tree {rust_remote}/beta src/tools/cargo"))
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
        Command::git(&format!("show-ref {cargo_remote}/{beta_version}")).run_stdout()?;
    let last_branch_hash = last_branch_line
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow!("Cargo beta ref did not contain a hash"))?;

    if last_beta_hash != last_branch_hash {
        bail!(
            "rust-lang/rust beta uses Cargo {last_beta_hash}, but \
             rust-lang/cargo {cargo_remote}/{beta_version} points to {last_branch_hash}"
        );
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

    let beta_prs = find_prs(
        &changelog,
        beta_hash_start,
        &format!("{cargo_remote}/{beta_version}"),
    )?;

    let added_idx = changelog
        .find("### Added\n")
        .ok_or_else(|| anyhow!("could not find `### Added` in {CHANGELOG_PATH}"))?;
    changelog.insert_str(added_idx, &format_pr_links(&beta_prs));

    fs::write(CHANGELOG_PATH, &changelog)?;
    commit_changelog(beta_minor_version)?;

    let master_prs = find_prs(
        &changelog,
        start_of_beta_short_hash,
        &format!("{cargo_remote}/master"),
    )?;

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
             \n\
             ### Documentation\n\
             \n\
             ### Internal\n\
             \n",
            next_version.minor - 1,
            next_version_date(next_version)?,
            format_pr_links(&master_prs),
        ),
    );
    fs::write(CHANGELOG_PATH, changelog)?;
    commit_changelog(next_version.minor - 1)
}

fn format_pr_links(prs: &[(u32, String, String)]) -> String {
    prs.iter()
        .map(|(number, url, description)| format!("- {description} \n  [#{number}]({url})\n"))
        .collect()
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
fn commit_changelog(minor_version: u64) -> Result<()> {
    if !Command::git("commit -a -m")
        .arg(format!("docs(changelog): 1.{minor_version}.0 update"))
        .run_success()?
    {
        bail!("failed to commit changelog");
    }
    Ok(())
}

fn next_version_date(next_version: &Version) -> Result<String> {
    let first = date!(2015 - 05 - 15);
    let next_days = ((next_version.minor - 1) * 42) as i64;
    let next_date = first + time::Duration::days(next_days - 1);
    Ok(next_date.format(format_description!("[year]-[month]-[day]"))?)
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    check_status(&cli.cargo_repo, &cli.cargo_remote)?;
    create_branch(&cli.cargo_remote, &cli.branch)?;
    let next_version = bump_version_toml()?;
    commit_bump(&next_version)?;
    prep_changelog(
        &next_version,
        &cli.rust_repo,
        &cli.cargo_remote,
        &cli.rust_remote,
    )?;
    eprintln!(
        "Review and edit {CHANGELOG_PATH} for nightly 1.{}.0 and beta 1.{}.0, then amend the changelog commit.",
        next_version.minor - 1,
        next_version.minor - 2,
    );
    eprintln!(
        "Prepared local branch `{}`. Push it manually when ready.",
        cli.branch
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        exit(1);
    }
}
