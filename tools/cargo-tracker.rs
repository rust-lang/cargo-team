#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
name = "cargo-tracker"
version = "0.1.0"
edition = "2021"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
anyhow = "1.0.76"
reqwest = { version = "0.11.23", features = ["blocking", "json"] }
serde_json = "1.0.108"
time = { version = "0.3.31", features = ["parsing"] }
tracing = "0.1.40"
tracing-subscriber = { version = "0.3.18", features = ["env-filter"] }
---
//! Updates the [Cargo status tracker](https://github.com/orgs/rust-lang/projects/47).
//!
//! Run from the repository root:
//!
//! ```console
//! cargo +nightly -Zscript tools/cargo-tracker.rs
//! ```
//!
//! * Requires `GITHUB_TOKEN` or an authenticated `gh` CLI with project write access.
//! * Running the script immediately mutates the project board.

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, HeaderValue, USER_AGENT};
use serde_json::json;
use std::collections::HashMap;
use std::process::Command;
use time::Date;
use tracing::{debug, info, trace};

const PROJECT_ID: &str = "PVT_kwDOAFLeec4AaJzI";

fn main() -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
        .from_env()?;
    tracing_subscriber::fmt()
        .with_timer(tracing_subscriber::fmt::time::Uptime::default())
        .with_env_filter(filter)
        .init();
    info!("starting");
    sync()?;
    info!("finished https://github.com/orgs/rust-lang/projects/47");
    Ok(())
}

#[derive(Debug)]
struct FieldInfo {
    id: String,
    /// For a single select field, the key is the name of the option, and the
    /// value is the ID of that option.
    options: HashMap<String, String>,
}

struct Tracker {
    client: Client,
    auth: HeaderValue,
    rfcbot: serde_json::Value,
    team_members: Vec<String>,
    items: Vec<serde_json::Value>,
    /// Key is the field name, value is information about the field.
    field_ids: HashMap<String, FieldInfo>,
}

impl Tracker {
    fn graphql(&self, query: &str, vars: serde_json::Value) -> Result<serde_json::Value> {
        graphql(&self.client, &self.auth, query, vars)
    }

    /// Returns whether or not the given field is already set to the given value.
    fn value_already_set(
        &self,
        item_id: &str,
        field_name: &str,
        field_kind: &str,
        new_value: &str,
    ) -> bool {
        // Determine if this value is already set.
        if let Some(item) = self
            .items
            .iter()
            .find(|item| item["id"].as_str().unwrap() == item_id)
        {
            if let Some(node) = item["fieldValues"]["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|value| {
                    value["field"]["name"]
                        .as_str()
                        .map(|name| name == field_name)
                        .unwrap_or(false)
                })
            {
                if node[field_kind]
                    .as_str()
                    .map(|node_value| node_value == new_value)
                    .unwrap_or(false)
                {
                    debug!("skipping item {item_id} value is already {new_value}");
                    return true;
                }
            } else if field_kind == "text" && new_value == "" {
                // GitHub treats an empty string as not set.
                return true;
            }
        }
        false
    }
}

fn sync() -> Result<()> {
    let client = Client::new();
    let auth = load_github_auth()?;
    let items = load_items(&client, &auth)?;
    let field_ids = get_field_ids(&client, &auth)?;
    let tracker = Tracker {
        client,
        auth,
        rfcbot: load_rfcbot()?,
        team_members: load_team_members()?,
        items,
        field_ids,
    };
    update_fcp_items(&tracker)?;
    add_missing_fcp(&tracker)?;
    sync_nominated(&tracker)?;
    Ok(())
}

fn load_github_auth() -> Result<HeaderValue> {
    let token = match std::env::var("GITHUB_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            let output = Command::new("gh").args(&["auth", "token"]).output()?;
            if !output.status.success() {
                bail!(
                    "failed to get GitHub token from `gh`:\n\
                    {}\n\
                    {}\n",
                    String::from_utf8(output.stdout).unwrap(),
                    String::from_utf8(output.stderr).unwrap()
                );
            }
            String::from_utf8(output.stdout).unwrap()
        }
    };
    let mut auth = HeaderValue::from_maybe_shared(format!("token {}", token.trim())).unwrap();
    auth.set_sensitive(true);
    Ok(auth)
}

fn load_team_members() -> Result<Vec<String>> {
    let team_data: serde_json::Value =
        reqwest::blocking::get("https://team-api.infra.rust-lang.org/v1/teams.json")?
            .error_for_status()?
            .json()?;
    Ok(team_data["cargo"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["github"].as_str().unwrap().to_string())
        .collect())
}

fn load_rfcbot() -> Result<serde_json::Value> {
    Ok(reqwest::blocking::get("https://rfcbot.rs/api/all")?
        .error_for_status()?
        .json()?)
}

fn graphql(
    client: &Client,
    auth: &HeaderValue,
    query: &str,
    vars: serde_json::Value,
) -> Result<serde_json::Value> {
    trace!("running query:\n{query}\nvars:\n{vars:#?}");
    let req = client
        .post("https://api.github.com/graphql")
        .header(USER_AGENT, "cargo-tracker")
        .header(AUTHORIZATION, auth)
        .json(&serde_json::json!({
            "query": query,
            "variables": vars,
        }));
    let req = req.build()?;
    let result: serde_json::Value = client.execute(req)?.error_for_status()?.json()?;
    if let Some(errors) = result["errors"].as_array() {
        let messages: Vec<_> = errors
            .iter()
            .map(|err| err["message"].as_str().unwrap_or_default())
            .collect();
        bail!("graphql failed: {}", messages.join("\n"));
    }
    Ok(result)
}

/// Loads all project items.
fn load_items(client: &Client, auth: &HeaderValue) -> Result<Vec<serde_json::Value>> {
    // TODO: Paginate to support more than 100
    // TODO: Does this include archived items?
    debug!("loading all project items");
    let mut items = graphql(
        client,
        auth,
        r#"query($projectId:ID!) {
            node(id: $projectId) {
                ... on ProjectV2 {
                    items(first: 100) {
                        nodes {
                            id
                            fieldValues(first: 100) {
                                nodes {
                                    ... on ProjectV2ItemFieldTextValue {
                                        text
                                        field {
                                            ... on ProjectV2FieldCommon {
                                                name
                                            }
                                        }
                                    }
                                    ... on ProjectV2ItemFieldDateValue {
                                        id
                                        date
                                        field {
                                            ... on ProjectV2FieldCommon {
                                                name
                                            }
                                        }
                                    }
                                    ... on ProjectV2ItemFieldSingleSelectValue {
                                        id
                                        name
                                        field {
                                            ... on ProjectV2FieldCommon {
                                                name
                                            }
                                        }
                                    }
                                }
                            }
                            content {
                                ... on DraftIssue {
                                    id
                                    title
                                    body
                                }
                                ... on Issue {
                                    id
                                    number
                                    title
                                    repository {
                                        nameWithOwner
                                    }
                                }
                                ... on PullRequest {
                                    id
                                    number
                                    title
                                    repository {
                                        nameWithOwner
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }"#,
        json!({"projectId": PROJECT_ID}),
    )?;
    let serde_json::Value::Array(items) = items["data"]["node"]["items"]["nodes"].take() else {
        bail!("expected an array of items");
    };
    Ok(items)
}

fn update_fcp_items(tracker: &Tracker) -> Result<()> {
    info!("updating {} items", tracker.items.len());
    for item in &tracker.items {
        let item_id = item["id"].as_str().unwrap();
        if let Some(repo) = item["content"].get("repository") {
            let name_with_owner = repo["nameWithOwner"].as_str().unwrap();
            let (owner, repo_name) = name_with_owner.split_once('/').unwrap();
            let number = item["content"]["number"].as_u64().unwrap();
            set_last_activity(tracker, item_id, owner, repo_name, number)?;
            if let Some(fcp) = tracker.rfcbot.as_array().unwrap().iter().find(|fcp| {
                let issue = &fcp["issue"];
                issue["number"].as_u64().unwrap() == number
                    && issue["repository"].as_str().unwrap() == name_with_owner
            }) {
                set_waiting_on(tracker, fcp, item_id)?;
                set_fcp_status(tracker, fcp, item_id)?;
            }
        }
    }
    Ok(())
}

fn add_missing_fcp(tracker: &Tracker) -> Result<()> {
    info!("adding missing items");
    for fcp in tracker.rfcbot.as_array().unwrap() {
        let issue = &fcp["issue"];
        if !issue["labels"]
            .as_array()
            .unwrap()
            .iter()
            .any(|label| label.as_str().unwrap() == "T-cargo")
        {
            continue;
        }
        let number = issue["number"].as_u64().unwrap();
        let repo = issue["repository"].as_str().unwrap();
        // Determine if this is already tracked in an item.
        if !tracker.items.iter().any(|item| {
            if let Some(item_repo) = item["content"]["repository"]["nameWithOwner"].as_str() {
                let item_number = item["content"]["number"].as_u64().unwrap();
                item_number == number && item_repo == repo
            } else {
                false
            }
        }) {
            info!("{repo} {number} is missing, adding it");
            create_item_from_fcp(tracker, &fcp)?;
        }
    }
    Ok(())
}

fn create_item_from_fcp(tracker: &Tracker, fcp: &serde_json::Value) -> Result<()> {
    let issue = &fcp["issue"];
    let repo = issue["repository"].as_str().unwrap();
    let number = issue["number"].as_u64().unwrap();
    let (owner, repo_name) = repo.split_once('/').unwrap();
    let id = issue_or_pr_id(tracker, owner, repo_name, number)?;
    let item_id = create_item_from_id(tracker, &id)?;

    set_fcp_status(tracker, fcp, &item_id)?;

    let created_at = date_from_iso(&fcp["status_comment"]["created_at"])?;
    set_field_value_date(tracker, &item_id, "Proposal started", &created_at)?;

    set_last_activity(tracker, &item_id, owner, repo_name, number)?;
    set_waiting_on(tracker, fcp, &item_id)?;

    Ok(())
}

/// Returns the ID of the newly created project item.
fn create_item_from_id(tracker: &Tracker, id: &str) -> Result<String> {
    let item_result = tracker.graphql(
        r#"mutation($projectId: ID!, $id: ID!) {
            addProjectV2ItemById(input: {projectId: $projectId, contentId: $id}) {
                item {
                    id
                }
            }
        }"#,
        json!({
            "projectId": PROJECT_ID,
            "id": id,
        }),
    )?;
    Ok(item_result["data"]["addProjectV2ItemById"]["item"]["id"]
        .as_str()
        .unwrap()
        .to_string())
}

fn issue_or_pr_id(tracker: &Tracker, owner: &str, repo_name: &str, number: u64) -> Result<String> {
    let id_result = tracker.graphql(
        r#"query($owner:String!, $name:String!, $number:Int!) {
            repository(owner:$owner, name:$name) {
                issueOrPullRequest(number:$number) {
                    ... on PullRequest {
                        id
                    }
                    ... on Issue {
                        id
                    }
                }
            }
        }"#,
        json!({
            "owner": owner,
            "name": repo_name,
            "number": number,
        }),
    )?;
    Ok(id_result["data"]["repository"]["issueOrPullRequest"]["id"]
        .as_str()
        .unwrap()
        .to_string())
}

fn get_field_ids(client: &Client, auth: &HeaderValue) -> Result<HashMap<String, FieldInfo>> {
    debug!("loading all project field IDs");
    let ids = graphql(
        client,
        auth,
        r#"query($projectId: ID!) {
            node(id: $projectId) {
                ... on ProjectV2 {
                    fields(first: 20) {
                        nodes {
                            ... on ProjectV2Field {
                                id
                                name
                            }
                            ... on ProjectV2IterationField {
                                id
                                name
                                configuration {
                                    iterations {
                                        startDate
                                        id
                                    }
                                }
                            }
                            ... on ProjectV2SingleSelectField {
                                id
                                name
                                options {
                                    id
                                    name
                                }
                            }
                        }
                    }
                }
            }
        }
        "#,
        json!({
            "projectId": PROJECT_ID

        }),
    )?;
    let fields = ids["data"]["node"]["fields"]["nodes"].as_array().unwrap();
    let mut result = HashMap::new();
    for field in fields {
        let key = field["name"].as_str().unwrap().to_string();
        let mut value = FieldInfo {
            id: field["id"].as_str().unwrap().to_string(),
            options: HashMap::new(),
        };
        if let Some(options) = field.get("options") {
            for option in options.as_array().unwrap() {
                let opt_key = option["name"].as_str().unwrap().to_string();
                let opt_value = option["id"].as_str().unwrap().to_string();
                value.options.insert(opt_key, opt_value);
            }
        }
        result.insert(key, value);
    }
    Ok(result)
}

fn set_field_value_text(
    tracker: &Tracker,
    item_id: &str,
    field_name: &str,
    text: &str,
) -> Result<()> {
    // Determine if this value is already set.
    if tracker.value_already_set(item_id, field_name, "text", &text) {
        return Ok(());
    }

    info!("setting {item_id} field {field_name} to {text}");

    let field_id = &tracker.field_ids[field_name].id;
    tracker.graphql(
        r#"mutation($projectId:ID!, $itemId:ID!, $fieldId: ID!, $text: String!) {
            updateProjectV2ItemFieldValue(
                input: {
                    projectId: $projectId
                    itemId: $itemId
                    fieldId: $fieldId
                    value: {
                        text: $text
                    }
                }
            ) {
                projectV2Item {
                    id
                }
            }
        }"#,
        json!({
            "projectId": PROJECT_ID,
            "itemId": item_id,
            "fieldId": field_id,
            "text": text,
        }),
    )?;
    Ok(())
}

fn set_field_value_single_select(
    tracker: &Tracker,
    item_id: &str,
    field_name: &str,
    option: &str,
) -> Result<()> {
    if tracker.value_already_set(item_id, field_name, "name", option) {
        return Ok(());
    }

    info!("setting {item_id} field {field_name} to {option}");

    let field_info = &tracker.field_ids[field_name];
    let field_id = &field_info.id;
    let option_id = &field_info.options[option];
    tracker.graphql(
        r#"mutation($projectId:ID!, $itemId:ID!, $fieldId: ID!, $optionId: String!) {
            updateProjectV2ItemFieldValue(
                input: {
                    projectId: $projectId
                    itemId: $itemId
                    fieldId: $fieldId
                    value: {
                        singleSelectOptionId: $optionId
                    }
                }
            ) {
                projectV2Item {
                    id
                }
            }
        }"#,
        json!({
            "projectId": PROJECT_ID,
            "itemId": item_id,
            "fieldId": field_id,
            "optionId": option_id,
        }),
    )?;
    Ok(())
}

fn set_field_value_date(
    tracker: &Tracker,
    item_id: &str,
    field_name: &str,
    date: &Date,
) -> Result<()> {
    let (y, m, d) = date.to_calendar_date();
    let m = m as u8;
    let iso_date = format!("{y}-{m:02}-{d:02}");

    // Determine if this value is already set.
    if tracker.value_already_set(item_id, field_name, "date", &iso_date) {
        return Ok(());
    }

    info!("setting {item_id} field {field_name} to {iso_date}");

    let field_id = &tracker.field_ids[field_name].id;
    tracker.graphql(
        r#"mutation($projectId:ID!, $itemId:ID!, $fieldId: ID!, $date: Date!) {
            updateProjectV2ItemFieldValue(
                input: {
                    projectId: $projectId
                    itemId: $itemId
                    fieldId: $fieldId
                    value: {
                        date: $date
                    }
                }
            ) {
                projectV2Item {
                    id
                }
            }
        }"#,
        json!({
            "projectId": PROJECT_ID,
            "itemId": item_id,
            "fieldId": field_id,
            "date": iso_date,
        }),
    )?;
    Ok(())
}

/// Sets the last date of activity for the issue or PR.
fn set_last_activity(
    tracker: &Tracker,
    item_id: &str,
    owner: &str,
    repo_name: &str,
    number: u64,
) -> Result<()> {
    let mut cursor = None;

    let mut overall = Date::MIN;
    let mut team = None;
    let mut author_date = Date::MIN;
    let mut opened;

    loop {
        let activity = last_activity_page(tracker, owner, repo_name, number, &cursor)?;
        opened = date_from_iso(&activity["createdAt"])?;
        author_date = author_date.max(opened);
        let author = activity["author"]["login"].as_str().unwrap_or("ghost");
        overall = overall.max(date_from_iso(&activity["updatedAt"])?);
        let items = &activity["timelineItems"];
        for item in items["nodes"].as_array().unwrap() {
            let user_dates = user_dates_from_timeline_item(item)?;
            for (user, date) in user_dates {
                overall = overall.max(date);
                let is_team_member = tracker.team_members.iter().any(|m| m == &user);
                if is_team_member {
                    team = Some(team.unwrap_or_else(|| Date::MIN).max(date));
                }
                if user == author {
                    author_date = author_date.max(date);
                }
            }
        }

        if !items["pageInfo"]["hasNextPage"].as_bool().unwrap() {
            break;
        }
        cursor = Some(items["pageInfo"]["endCursor"].as_str().unwrap().to_string());
    }

    set_field_value_date(tracker, item_id, "Last activity", &overall)?;
    if let Some(team) = team {
        set_field_value_date(tracker, item_id, "Last team activity", &team)?;
    }
    set_field_value_date(tracker, item_id, "Last author activity", &author_date)?;
    set_field_value_date(tracker, item_id, "Opened", &opened)?;

    Ok(())
}

/// Returns a single page from the activity timeline of the issue or PR.
fn last_activity_page(
    tracker: &Tracker,
    owner: &str,
    repo_name: &str,
    number: u64,
    cursor: &Option<String>,
) -> Result<serde_json::Value> {
    // Note: It is possible to filter the timeline types via `itemTypes`, but
    // it is a bit tedious to list them all out.
    let mut activity = tracker.graphql(
        r#"query ($owner: String!, $name: String!, $number: Int!, $cursor: String) {
              repository(owner: $owner, name: $name) {
                issueOrPullRequest(number: $number) {
                  ... on PullRequest {
                    id
                    author { login }
                    createdAt
                    updatedAt
                    timelineItems(first: 100, after: $cursor) {
                      nodes {
                        __typename
                        ... on IssueComment {
                          author { login }
                          updatedAt
                        }
                        ... on PullRequestCommit {
                          commit {
                            committedDate
                            author { user {login } }
                          }
                        }
                        ... on PullRequestReview {
                          author { login }
                          updatedAt
                          comments(first: 100) {
                            nodes {
                              author { login }
                              updatedAt
                            }
                          }
                        }
                        ... on ClosedEvent {
                          actor { login }
                          createdAt
                        }
                        ... on ReopenedEvent {
                          actor { login }
                          createdAt
                        }
                        ... on BaseRefForcePushedEvent {
                          actor { login }
                          createdAt
                        }
                        ... on MergedEvent {
                          actor { login }
                          createdAt
                        }
                        ... on BaseRefChangedEvent {
                          actor { login }
                          createdAt
                        }
                        ... on BaseRefDeletedEvent {
                          actor { login }
                          createdAt
                        }
                        ... on CommentDeletedEvent {
                          actor { login }
                          createdAt
                        }
                        ... on ConvertToDraftEvent {
                          actor { login }
                          createdAt
                        }
                        ... on HeadRefDeletedEvent {
                          actor { login }
                          createdAt
                        }
                        ... on HeadRefForcePushedEvent {
                          actor { login }
                          createdAt
                        }
                        ... on HeadRefRestoredEvent {
                          actor { login }
                          createdAt
                        }
                        ... on PullRequestCommitCommentThread {
                          comments(first: 100) {
                            nodes {
                              author { login }
                              updatedAt
                            }
                          }
                        }
                        ... on PullRequestReviewThread {
                          comments(first: 100) {
                            nodes {
                              author { login }
                              updatedAt
                            }
                          }
                        }
                        ... on ReadyForReviewEvent {
                          actor { login }
                          createdAt
                        }
                        ... on RenamedTitleEvent {
                          actor { login }
                          createdAt
                        }
                        ... on ReviewDismissedEvent {
                          actor { login }
                          createdAt
                        }
                        ... on ReviewRequestRemovedEvent {
                          actor { login }
                          createdAt
                        }
                      }
                      pageInfo {
                        hasNextPage
                        endCursor
                      }
                    }
                  }
                  ... on Issue {
                    id
                    author { login }
                    createdAt
                    updatedAt
                    timelineItems(first: 100, after: $cursor) {
                      nodes {
                        __typename
                        ... on ClosedEvent {
                          actor { login }
                          createdAt
                        }
                        ... on CommentDeletedEvent {
                          actor { login }
                          createdAt
                        }
                        ... on IssueComment {
                          author { login }
                          updatedAt
                        }
                        ... on RenamedTitleEvent {
                          actor { login }
                          createdAt
                        }
                        ... on ReopenedEvent {
                          actor { login }
                          createdAt
                        }
                      }
                      pageInfo {
                        hasNextPage
                        endCursor
                      }
                    }
                  }
                }
              }
            }"#,
        json!({
            "owner": owner,
            "name": repo_name,
            "number": number,
            "cursor": cursor,
        }),
    )?;
    Ok(activity["data"]["repository"]["issueOrPullRequest"].take())
}

fn date_from_iso(iso: &serde_json::Value) -> Result<Date> {
    let iso = iso.as_str().unwrap();
    let f = time::format_description::well_known::iso8601::Iso8601::DATE_TIME;
    let d = time::Date::parse(iso, &f).with_context(|| format!("failed to parse {iso}"))?;
    Ok(d)
}

fn user_dates_from_timeline_item(item: &serde_json::Value) -> Result<Vec<(String, Date)>> {
    if let Some(comments) = item.get("comments") {
        let uds = comments["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                (
                    c["author"]["login"].as_str().unwrap_or("ghost").to_string(),
                    date_from_iso(&c["updatedAt"]).expect("date"),
                )
            })
            .collect();
        return Ok(uds);
    }

    let ud = if let Some(updated_at) = item.get("updatedAt") {
        (
            item["author"]["login"]
                .as_str()
                .unwrap_or("ghost")
                .to_string(),
            date_from_iso(updated_at)?,
        )
    } else if let Some(created_at) = item.get("createdAt") {
        (
            item["actor"]["login"]
                .as_str()
                .unwrap_or("ghost")
                .to_string(),
            date_from_iso(created_at)?,
        )
    } else if let Some(commit) = item.get("commit") {
        (
            commit["author"]["user"]["login"]
                .as_str()
                .unwrap_or("ghost")
                .to_string(),
            date_from_iso(&commit["committedDate"])?,
        )
    } else {
        return Ok(vec![]);
    };
    Ok(vec![ud])
}

fn set_waiting_on(tracker: &Tracker, fcp: &serde_json::Value, item_id: &str) -> Result<()> {
    let mut unreviewed: Vec<_> = fcp["reviews"]
        .as_array()
        .unwrap()
        .iter()
        .map(|review| review.as_array().unwrap())
        .filter(|review| !review[1].as_bool().unwrap())
        .map(|review| review[0]["login"].as_str().unwrap())
        .collect();
    unreviewed.sort();
    set_field_value_text(tracker, item_id, "Waiting on", &unreviewed.join(" "))?;
    Ok(())
}

fn set_fcp_status(tracker: &Tracker, fcp: &serde_json::Value, item_id: &str) -> Result<()> {
    let mut status = match fcp["fcp"]["disposition"].as_str().unwrap() {
        "merge" => "FCP merge",
        "close" => "FCP close",
        "postpone" => "FCP postpone",
        d => bail!("unexpected FCP disposition {d}"),
    };
    if !fcp["concerns"].as_array().unwrap().is_empty() {
        status = "FCP blocked";
    }
    set_field_value_single_select(tracker, &item_id, "Status", status)?;
    Ok(())
}

fn sync_nominated(tracker: &Tracker) -> Result<()> {
    // Get all project items that are in the nominated status.
    info!("synching nominated items");
    let nominated_items: Vec<_> = tracker
        .items
        .iter()
        .filter(|item| {
            item["fieldValues"]["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| {
                    field["field"]["name"].as_str().unwrap_or("") == "Status"
                        && field["name"] == "Nominated"
                })
        })
        .filter(|item| {
            // don't include draft issues
            item["content"].get("number").is_some()
        })
        .map(|item| {
            (
                item["id"].as_str().unwrap(),
                item["content"]["id"].as_str().unwrap(),
            )
        })
        .collect();
    // For each item, verify it still has a nominated label. Archive if not.
    for (item_id, content_id) in &nominated_items {
        let labels = tracker.graphql(
            r#"query($id: ID!) {
                node(id: $id) {
                    ... on Issue {
                        labels(first: 100) {
                            nodes {
                                name
                            }
                        }
                    }
                    ... on PullRequest {
                        labels(first: 100) {
                            nodes {
                                name
                            }
                        }
                    }
                }
            }"#,
            json!({"id": content_id}),
        )?;
        if !labels["data"]["node"]["labels"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|label| {
                matches!(
                    label["name"].as_str().unwrap(),
                    "I-nominated-to-discuss" | "I-cargo-nominated"
                )
            })
        {
            // No longer nominated, archive it.
            archive(tracker, item_id)?;
        }
    }
    // Add new nominated to discuss.
    for (repo, label) in [
        ("cargo", "I-nominated-to-discuss"),
        ("rust", "I-cargo-nominated"),
        ("rfcs", "I-cargo-nominated"),
    ] {
        let nominated = tracker.graphql(
            r#"query($repo: String!, $label: String!) {
          repository(owner: "rust-lang", name: $repo) {
            label(name: $label) {
              issues(first: 100, states:[OPEN]) {
                nodes {
                  id
                }
              }
              pullRequests(first: 100, states:[OPEN]) {
                nodes {
                  id
                }
              }
            }
          }
        }"#,
            json!({"repo": repo, "label": label}),
        )?;
        let issues = nominated["data"]["repository"]["label"]["issues"]["nodes"]
            .as_array()
            .unwrap();
        let prs = nominated["data"]["repository"]["label"]["pullRequests"]["nodes"]
            .as_array()
            .unwrap();
        for node in issues.iter().chain(prs) {
            let id = node["id"].as_str().unwrap();
            if !nominated_items.iter().any(|(_item_id, cid)| cid == &id) {
                info!("adding nominated item {id}");
                let item_id = create_item_from_id(tracker, id)?;
                set_field_value_single_select(tracker, &item_id, "Status", "Nominated")?;
            }
        }
    }
    Ok(())
}

fn archive(tracker: &Tracker, item_id: &str) -> Result<()> {
    info!("archiving item {item_id}");
    tracker.graphql(
        r#"mutation ($projectId: ID!, $id: ID!) {
          archiveProjectV2Item(input: {projectId: $projectId, itemId: $id}) {
            clientMutationId
          }
        }"#,
        json!({
            "projectId": PROJECT_ID,
            "id": item_id,
        }),
    )?;
    Ok(())
}
