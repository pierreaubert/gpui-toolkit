//! Queryable component catalog: detail, list, and search.
//!
//! These functions read the component-lab story registry and project it
//! into CLI-sized documents. [`component_detail`] returns one full story,
//! [`component_list`] returns id/title/description rows at a [`DetailLevel`],
//! [`component_props`] returns just the props table, [`search`] ranks
//! stories against a free-text query, and [`component_batch`] resolves
//! several names in one catalog load. Unknown ids fail with
//! [`ErrorCode::UnknownComponent`](crate::ErrorCode) plus suggestions, so
//! agents recover with one follow-up call instead of guessing.

// Rust guideline compliant 2026-02-21

use crate::error_codes::{ErrorCode, Suggestion, ToolkitError};
use gpui_component_lab::{ComponentStory, StoryProp, StoryPropValue, builtin_story_registry};
use serde::Serialize;

/// Discriminator for component detail payloads.
pub const COMPONENT_DETAIL_TYPE: &str = "component.detail";

/// Discriminator for component list payloads.
pub const COMPONENT_LIST_TYPE: &str = "component.list";

/// Discriminator for component props payloads.
pub const COMPONENT_PROPS_TYPE: &str = "component.detail.props";

/// Discriminator for batch payloads in JSON envelopes.
pub const COMPONENT_BATCH_TYPE: &str = "component.batch";

/// Discriminator for search payloads.
pub const SEARCH_TYPE: &str = "search";

/// Default maximum search results.
///
/// Large enough for exploration, small enough to keep agent context
/// cheap; callers override with an explicit limit.
pub const DEFAULT_SEARCH_LIMIT: usize = 20;

/// Amount of detail in list output.
///
/// Brief carries names only, compact adds one-line titles, full adds
/// descriptions. Single-item views always return everything and ignore
/// this setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetailLevel {
    /// Names only.
    #[default]
    Brief,
    /// Names plus one-line titles.
    Compact,
    /// Names, titles, and descriptions.
    Full,
}

/// Parses a detail level from user input.
///
/// Accepts `brief`, `compact`, and `full` exactly.
///
/// # Errors
///
/// Returns [`ErrorCode::InvalidArgument`] for any other value.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::{DetailLevel, parse_detail_level};
///
/// assert_eq!(parse_detail_level("compact").unwrap(), DetailLevel::Compact);
/// assert!(parse_detail_level("verbose").is_err());
/// ```
pub fn parse_detail_level(text: &str) -> Result<DetailLevel, ToolkitError> {
    match text {
        "brief" => Ok(DetailLevel::Brief),
        "compact" => Ok(DetailLevel::Compact),
        "full" => Ok(DetailLevel::Full),
        _ => Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!("detail '{text}' must be brief, compact, or full"),
        )),
    }
}

/// One row of component list output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentEntry {
    /// Story id, such as `ui-kit.button`.
    pub id: String,
    /// Owning crate, such as `gpui-ui-kit`.
    pub crate_name: String,
    /// One-line title; present at compact detail and above.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Description; present at full detail only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Component list payload honoring a [`DetailLevel`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentList {
    /// Detail level served: `brief`, `compact`, or `full`.
    pub detail: String,
    /// Number of entries.
    pub count: usize,
    /// Entries sorted by id.
    pub entries: Vec<ComponentEntry>,
}

/// Full component document wrapping one story.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentDetail {
    /// The story, including props, metadata, and conformance.
    pub story: ComponentStory,
    /// Follow-up command printing just the props table.
    pub follow_up: String,
}

/// Props-table payload for one component.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentProps {
    /// Story id.
    pub id: String,
    /// Props in story order.
    pub props: Vec<StoryProp>,
}

/// One ranked search hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    /// Story id.
    pub id: String,
    /// Owning crate.
    pub crate_name: String,
    /// One-line title.
    pub title: String,
    /// Match score, higher is better; see [`search`].
    pub score: u32,
    /// Why this story matched.
    pub reason: String,
    /// Follow-up command printing the full document.
    pub command: String,
}

/// Ranked search payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    /// Echoed query.
    pub query: String,
    /// Total matches before the limit applied.
    pub match_count: usize,
    /// Hits by score, then id.
    pub results: Vec<SearchHit>,
}

/// One batch result: `ok` carries a payload, `error` a code.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentBatchItem {
    /// Requested name, as spelled.
    pub name: String,
    /// `ok` or `error`.
    pub status: String,
    /// Detail document (detail mode hits only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ComponentDetail>,
    /// Props table (props mode hits only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<ComponentProps>,
    /// Failure code (misses only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<ErrorCode>,
    /// Human-readable failure (misses only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Similarly named story ids (misses only).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<String>,
}

/// Multi-read payload resolving several names at once.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentBatch {
    /// Payload mode: `detail` or `props`.
    pub mode: String,
    /// Requested names.
    pub count: usize,
    /// Hits.
    pub ok: usize,
    /// Misses.
    pub errors: usize,
    /// Results in request order.
    pub results: Vec<ComponentBatchItem>,
}

/// Loads all stories sorted by id.
///
/// # Errors
///
/// Returns [`ErrorCode::CatalogLoad`] when the registry cannot build.
fn load_stories() -> Result<Vec<ComponentStory>, ToolkitError> {
    let registry = builtin_story_registry()
        .map_err(|error| ToolkitError::new(ErrorCode::CatalogLoad, error.to_string()))?;
    let mut stories: Vec<ComponentStory> = registry.stories().cloned().collect();
    stories.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(stories)
}

/// Returns the full document for one component.
///
/// Matches `name` against story ids exactly, then titles
/// case-insensitively, so both `ui-kit.button` and `button` resolve.
///
/// # Errors
///
/// Returns [`ErrorCode::CatalogLoad`] when the registry cannot build and
/// [`ErrorCode::UnknownComponent`] with up to three suggestions when
/// nothing matches.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::component_detail;
///
/// let detail = component_detail("ui-kit.button").unwrap();
/// assert_eq!(detail.story.id, "ui-kit.button");
/// ```
pub fn component_detail(name: &str) -> Result<ComponentDetail, ToolkitError> {
    let stories = load_stories()?;
    if let Some(story) = find_story(&stories, name) {
        return Ok(detail_for(story));
    }
    Err(missing_component(&stories, name))
}

/// Matches a name against story ids exactly, then titles.
///
/// Title matching is case-insensitive, so both `ui-kit.button` and
/// `button` resolve.
fn find_story<'stories>(
    stories: &'stories [ComponentStory],
    name: &str,
) -> Option<&'stories ComponentStory> {
    if let Some(story) = stories.iter().find(|story| story.id == name) {
        return Some(story);
    }
    let lowered = name.to_lowercase();
    stories
        .iter()
        .find(|story| story.title.to_lowercase() == lowered)
}

/// Builds the unknown-component error with up to three suggestions.
fn missing_component(stories: &[ComponentStory], name: &str) -> ToolkitError {
    let suggestions: Vec<Suggestion> = search_in(stories, name, 3)
        .results
        .into_iter()
        .map(|hit| Suggestion::new(hit.id, "similar name"))
        .collect();
    ToolkitError::new(
        ErrorCode::UnknownComponent,
        format!("no component named '{name}'"),
    )
    .with_suggestions(suggestions)
}

/// Returns detail or props documents for several names at once.
///
/// The catalog loads once; misses record per-item errors with
/// suggestions instead of failing the batch, so agents resolve a
/// whole import list in one call.
///
/// # Errors
///
/// Returns [`ErrorCode::CatalogLoad`] when the registry cannot build.
/// Unknown names never fail the call; they surface as `error` items.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::component_batch;
///
/// let batch = component_batch(&["ui-kit.button".to_owned(), "nope".to_owned()], false).unwrap();
/// assert_eq!(batch.count, 2);
/// assert_eq!(batch.ok, 1);
/// assert_eq!(batch.errors, 1);
/// ```
pub fn component_batch(names: &[String], props_only: bool) -> Result<ComponentBatch, ToolkitError> {
    let stories = load_stories()?;
    let mut results = Vec::with_capacity(names.len());
    let mut ok = 0;
    for name in names {
        if let Some(story) = find_story(&stories, name) {
            ok += 1;
            let detail = detail_for(story);
            let (detail, props) = if props_only {
                (
                    None,
                    Some(ComponentProps {
                        id: detail.story.id.clone(),
                        props: detail.story.props.clone(),
                    }),
                )
            } else {
                (Some(detail), None)
            };
            results.push(ComponentBatchItem {
                name: name.clone(),
                status: String::from("ok"),
                detail,
                props,
                code: None,
                message: None,
                suggestions: Vec::new(),
            });
        } else {
            let suggestions: Vec<String> = search_in(&stories, name, 3)
                .results
                .into_iter()
                .map(|hit| hit.id)
                .collect();
            results.push(ComponentBatchItem {
                name: name.clone(),
                status: String::from("error"),
                detail: None,
                props: None,
                code: Some(ErrorCode::UnknownComponent),
                message: Some(format!("no component named '{name}'")),
                suggestions,
            });
        }
    }
    Ok(ComponentBatch {
        mode: if props_only {
            String::from("props")
        } else {
            String::from("detail")
        },
        count: names.len(),
        ok,
        errors: names.len() - ok,
        results,
    })
}

/// Builds a detail document with its follow-up command.
fn detail_for(story: &ComponentStory) -> ComponentDetail {
    ComponentDetail {
        story: story.clone(),
        follow_up: format!("gpui-toolkit component {} --props", story.id),
    }
}

/// Lists components at a detail level.
///
/// Entries sort by id. Brief carries names only, compact adds titles,
/// full adds descriptions; props always require [`component_detail`] or
/// [`component_props`] so listings stay cheap.
///
/// # Errors
///
/// Returns [`ErrorCode::CatalogLoad`] when the registry cannot build.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::{DetailLevel, component_list};
///
/// let list = component_list(DetailLevel::Brief).unwrap();
/// assert!(list.count > 0);
/// assert_eq!(list.detail, "brief");
/// ```
pub fn component_list(level: DetailLevel) -> Result<ComponentList, ToolkitError> {
    let stories = load_stories()?;
    let entries = stories
        .iter()
        .map(|story| ComponentEntry {
            id: story.id.clone(),
            crate_name: story.crate_name.clone(),
            title: (level == DetailLevel::Compact || level == DetailLevel::Full)
                .then(|| story.title.clone()),
            description: (level == DetailLevel::Full).then(|| story.description.clone()),
        })
        .collect();
    Ok(ComponentList {
        detail: match level {
            DetailLevel::Brief => "brief",
            DetailLevel::Compact => "compact",
            DetailLevel::Full => "full",
        }
        .to_owned(),
        count: stories.len(),
        entries,
    })
}

/// Returns just the props table for one component.
///
/// Accepts the same names as [`component_detail`].
///
/// # Errors
///
/// Returns [`ErrorCode::CatalogLoad`] when the registry cannot build and
/// [`ErrorCode::UnknownComponent`] with suggestions when nothing matches.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::component_props;
///
/// let props = component_props("ui-kit.button").unwrap();
/// assert!(props.props.iter().any(|prop| prop.name == "label"));
/// ```
pub fn component_props(name: &str) -> Result<ComponentProps, ToolkitError> {
    let detail = component_detail(name)?;
    Ok(ComponentProps {
        id: detail.story.id.clone(),
        props: detail.story.props.clone(),
    })
}

/// Ranks stories against a free-text query.
///
/// Scoring is deterministic: exact id (100), id prefix (80), title
/// substring (60), id substring (50), description substring (30), crate
/// substring (10); ties break by id. Matching is case-insensitive.
/// `limit` caps the returned slice; `match_count` reports the total.
///
/// # Errors
///
/// Returns [`ErrorCode::CatalogLoad`] when the registry cannot build.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::search;
///
/// let results = search("button", 5).unwrap();
/// assert!(results.match_count > 0);
/// assert_eq!(results.results[0].id, "ui-kit.button");
/// ```
pub fn search(query: &str, limit: usize) -> Result<SearchResults, ToolkitError> {
    let stories = load_stories()?;
    Ok(search_in(&stories, query, limit))
}

/// Ranks preloaded stories against a query; see [`search`].
fn search_in(stories: &[ComponentStory], query: &str, limit: usize) -> SearchResults {
    let needle = query.to_lowercase();
    let mut scored: Vec<(u32, &str, &ComponentStory)> = Vec::new();
    for story in stories {
        let id = story.id.to_lowercase();
        let title = story.title.to_lowercase();
        let description = story.description.to_lowercase();
        let crate_name = story.crate_name.to_lowercase();
        if id == needle {
            scored.push((100, "exact id", story));
        } else if id.starts_with(&needle) {
            scored.push((80, "id prefix", story));
        } else if title.contains(&needle) {
            scored.push((60, "title match", story));
        } else if id.contains(&needle) {
            scored.push((50, "id match", story));
        } else if description.contains(&needle) {
            scored.push((30, "description match", story));
        } else if crate_name.contains(&needle) {
            scored.push((10, "crate match", story));
        }
    }
    scored.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.2.id.cmp(&right.2.id))
    });
    let match_count = scored.len();
    let results = scored
        .into_iter()
        .take(limit)
        .map(|(score, reason, story)| SearchHit {
            id: story.id.clone(),
            crate_name: story.crate_name.clone(),
            title: story.title.clone(),
            score,
            reason: reason.to_owned(),
            command: format!("gpui-toolkit component {}", story.id),
        })
        .collect();
    SearchResults {
        query: query.to_owned(),
        match_count,
        results,
    }
}

/// Renders one prop value in human-readable text.
fn render_value(value: &StoryPropValue) -> String {
    match value {
        StoryPropValue::Bool(flag) => format!("bool = {flag}"),
        StoryPropValue::Number(number) => format!("number = {number}"),
        StoryPropValue::Text(text) => format!("text = \"{text}\""),
        StoryPropValue::Choice(choice) => format!("choice = \"{choice}\""),
        StoryPropValue::Color(color) => format!("color = \"{color}\""),
    }
}

/// Renders a detail document as human-readable text.
///
/// Dense output keeps one fact per line without prose; the default
/// rendering uses short sections instead.
pub fn render_detail_text(detail: &ComponentDetail, dense: bool) -> String {
    let story = &detail.story;
    if dense {
        let mut text = format!(
            "{} | {} | {}\n{}\n",
            story.id, story.crate_name, story.title, story.description
        );
        for prop in &story.props {
            text.push_str(&format!(
                "prop {} \"{}\" {}\n",
                prop.name,
                prop.label,
                render_value(&prop.value)
            ));
        }
        text.push_str(&format!("follow-up {}\n", detail.follow_up));
        return text;
    }
    let mut text = format!(
        "{} ({})\n{}\n\n{}\n",
        story.title, story.id, story.crate_name, story.description
    );
    if story.props.is_empty() {
        text.push_str("\nNo props.\n");
    } else {
        text.push_str("\nProps:\n");
        for prop in &story.props {
            text.push_str(&format!(
                "  {} — {} [{}]",
                prop.name,
                prop.label,
                render_value(&prop.value)
            ));
            if !prop.options.is_empty() {
                text.push_str(&format!(" (options: {})", prop.options.join(", ")));
            }
            text.push('\n');
        }
    }
    text.push_str(&format!("\n{}\n", detail.follow_up));
    text
}

/// Renders a component list as human-readable text.
pub fn render_list_text(list: &ComponentList, dense: bool) -> String {
    let mut text = String::new();
    if !dense {
        text.push_str(&format!("{} components ({})\n\n", list.count, list.detail));
    }
    for entry in &list.entries {
        if dense {
            text.push_str(&format!("{} {}\n", entry.id, entry.crate_name));
        } else {
            match (&entry.title, &entry.description) {
                (Some(title), Some(description)) => {
                    text.push_str(&format!("{} — {title}: {description}\n", entry.id));
                }
                (Some(title), None) => {
                    text.push_str(&format!("{} — {title}\n", entry.id));
                }
                (None, _) => text.push_str(&format!("{}\n", entry.id)),
            }
        }
    }
    text
}

/// Renders a props payload as human-readable text.
pub fn render_props_text(props: &ComponentProps, dense: bool) -> String {
    let mut text = String::new();
    if !dense {
        text.push_str(&format!("{} props:\n", props.id));
    }
    for prop in &props.props {
        if dense {
            text.push_str(&format!("{} {}\n", prop.name, render_value(&prop.value)));
        } else {
            text.push_str(&format!(
                "  {} — {} [{}]",
                prop.name,
                prop.label,
                render_value(&prop.value)
            ));
            if !prop.options.is_empty() {
                text.push_str(&format!(" (options: {})", prop.options.join(", ")));
            }
            text.push('\n');
        }
    }
    text
}

/// Renders a batch payload as human-readable text.
///
/// Dense output keeps one line per item plus the counts; the default
/// rendering sections each hit with its full document.
pub fn render_batch_text(batch: &ComponentBatch, dense: bool) -> String {
    let mut text = String::new();
    for item in &batch.results {
        if item.status == "ok" {
            if dense {
                let id = item
                    .detail
                    .as_ref()
                    .map(|detail| detail.story.id.as_str())
                    .or_else(|| item.props.as_ref().map(|props| props.id.as_str()))
                    .unwrap_or(&item.name);
                text.push_str(&format!("ok {id}\n"));
                continue;
            }
            text.push_str(&format!("=== {} (ok)\n", item.name));
            if let Some(detail) = &item.detail {
                text.push_str(&render_detail_text(detail, false));
            }
            if let Some(props) = &item.props {
                text.push_str(&render_props_text(props, false));
            }
            text.push('\n');
        } else if dense {
            text.push_str(&format!(
                "error {} {}\n",
                item.name,
                item.code.map_or("ERR_UNKNOWN_COMPONENT", ErrorCode::as_str)
            ));
        } else {
            text.push_str(&format!(
                "=== {} (error): {}\n",
                item.name,
                item.message.as_deref().unwrap_or("unknown error")
            ));
            if !item.suggestions.is_empty() {
                text.push_str(&format!("  similar: {}\n", item.suggestions.join(", ")));
            }
            text.push('\n');
        }
    }
    if dense {
        text.push_str(&format!("ok={} errors={}\n", batch.ok, batch.errors));
    } else {
        text.push_str(&format!(
            "{} of {} resolved ({} errors).\n",
            batch.ok, batch.count, batch.errors
        ));
    }
    text
}

/// Renders search results as human-readable text.
pub fn render_search_text(results: &SearchResults, dense: bool) -> String {
    let mut text = String::new();
    if !dense {
        text.push_str(&format!(
            "Results for \"{}\" ({} of {}):\n\n",
            results.query,
            results.results.len(),
            results.match_count
        ));
    }
    for hit in &results.results {
        if dense {
            text.push_str(&format!("{} {} {}\n", hit.score, hit.id, hit.reason));
        } else {
            text.push_str(&format!(
                "[{}] {} — {} ({})\n  → {}\n",
                hit.score, hit.id, hit.title, hit.reason, hit.command
            ));
        }
    }
    text
}
