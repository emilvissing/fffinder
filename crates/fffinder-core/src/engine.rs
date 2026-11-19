//! Search engine: one `fff-search` index per search root, searched in
//! parallel and merged by score. Folders outside every root get a temporary
//! index on demand, so "search this folder" works anywhere. Apps come from
//! an [`AppList`] instead of an index.

use crate::app_list::AppList;
use crate::config::{SearchCase, SearchSettings};
use crate::entry::{FileEntry, display_name};
use crate::kind::{self, KindFilter};
use crate::listing::{self, SortKey, SortOrder};
use fff_search::file_picker::FilePicker;
use fff_search::{
    Casing, Constraint, DirSearchConfig, FFFMode, FilePickerOptions, FrecencyTracker, FuzzyQuery,
    FuzzySearchOptions, GitRecencyConfig, GrepConfig, GrepMode, GrepSearchOptions, MixedItemRef,
    PaginationArgs, ParserConfig, QueryParser, QueryTracker, SharedFilePicker, SharedFrecency,
    SharedQueryTracker, WatchEvent, WatchId, WatchOptions,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};

/// Temporary (outside-of-roots) indexes kept alive, most recent first.
const MAX_ADHOC: usize = 3;

#[derive(Debug, Clone)]
pub struct EngineOptions {
    pub roots: Vec<PathBuf>,
    /// Folders whose apps "This Computer" searches find by name without
    /// indexing them (see [`AppList`]); empty for none.
    pub app_dirs: Vec<PathBuf>,
    /// Directory for frecency/query-history databases; `None` disables them.
    pub db_dir: Option<PathBuf>,
    /// Keep indexes live with filesystem watchers (when `search.watch` is on).
    pub watch: bool,
    /// fff-search's options from Settings.
    pub search: SearchSettings,
}

#[derive(Clone)]
struct RootIndex {
    path: PathBuf,
    /// `path` with symlinks resolved, to recognize folders spelled another
    /// way (`/private/var/…` for `/var/…` on macOS).
    canonical: PathBuf,
    picker: SharedFilePicker,
}

impl RootIndex {
    fn start(
        path: &Path,
        frecency: &SharedFrecency,
        watch: bool,
        settings: &SearchSettings,
    ) -> anyhow::Result<RootIndex> {
        let picker = SharedFilePicker::default();
        FilePicker::new_with_shared_state(
            picker.clone(),
            frecency.clone(),
            FilePickerOptions {
                base_path: path.to_string_lossy().into_owned(),
                mode: FFFMode::Neovim,
                watch: watch && settings.watch,
                follow_symlinks: settings.follow_symlinks,
                enable_content_indexing: settings.content_index,
                enable_mmap_cache: settings.preload_files,
                git_recency: GitRecencyConfig {
                    enabled: settings.git_recency,
                    ..Default::default()
                },
                // Roots are chosen by the user, so home and `/` are allowed.
                enable_fs_root_scanning: true,
                enable_home_dir_scanning: true,
                ..Default::default()
            },
        )
        .map_err(|e| anyhow::anyhow!("cannot index {}: {e}", path.display()))?;
        Ok(RootIndex {
            path: path.to_path_buf(),
            canonical: std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()),
            picker,
        })
    }

    /// `scope`, a folder in this root, spelled the way fff spells the root's
    /// base path, which starts every path fff returns. On Windows fff
    /// canonicalizes the base (long names, no `\\?\`), so it can differ from
    /// the root as given, as `C:\Users\RUNNER~1` does from `C:\Users\runneradmin`.
    fn in_base(&self, scope: &Path, base: &Path) -> PathBuf {
        match scope.strip_prefix(&self.path) {
            Ok(rest) if rest.as_os_str().is_empty() => base.to_path_buf(),
            Ok(rest) => base.join(rest),
            Err(_) => scope.to_path_buf(),
        }
    }

    /// Whether its index is still being built; cheaper than [`Self::status`],
    /// which counts folders.
    fn scanning(&self) -> bool {
        let guard = self.picker.read().ok();
        guard
            .as_ref()
            .and_then(|g| g.as_ref())
            .is_none_or(|p| p.get_scan_progress().is_scanning)
    }

    fn status(&self) -> RootStatus {
        let guard = self.picker.read().ok();
        let picker = guard.as_ref().and_then(|g| g.as_ref());
        match picker {
            Some(p) => {
                let progress = p.get_scan_progress();
                RootStatus {
                    path: self.path.clone(),
                    scanning: progress.is_scanning,
                    finishing: !progress.is_scanning && p.is_post_scan_active(),
                    indexed_files: if progress.is_scanning {
                        progress.scanned_files_count
                    } else {
                        p.live_file_count()
                    },
                    indexed_dirs: p.get_dirs().iter().filter(|d| !d.is_deleted()).count(),
                    git_repo: p.has_git_repo(),
                }
            }
            None => RootStatus {
                path: self.path.clone(),
                scanning: true,
                finishing: false,
                indexed_files: 0,
                indexed_dirs: 0,
                git_repo: false,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootStatus {
    pub path: PathBuf,
    pub scanning: bool,
    /// Scanned and searchable, while fff-search still reads each file once
    /// to flag binaries, which takes minutes for a large folder.
    #[serde(default)]
    pub finishing: bool,
    /// Files scanned so far while scanning, or live files once ready.
    pub indexed_files: usize,
    #[serde(default)]
    pub indexed_dirs: usize,
    pub git_repo: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchRequest {
    pub query: String,
    /// Only return results under this folder.
    pub scope: Option<PathBuf>,
    pub kind: KindFilter,
    pub limit: usize,
    pub show_hidden: bool,
    /// With [`KindFilter::Any`], leave folders out.
    pub files_only: bool,
    /// What the hits are going to be sorted by. When more match than
    /// `limit`, the list is then the first of every match in this order,
    /// the largest of them all, say, and not the best `limit`. That is only
    /// done where weak matches can't take the place of strong ones by it; a
    /// search where they could lists its best matches whatever this says,
    /// and [`SearchResponse::sort_chooses`] says which it was. The hits come
    /// back best first either way, for the caller to put in order. `None`
    /// and [`SortKey::Relevance`] ask for the best.
    pub sort: Option<SortOrder>,
    /// Read from the disk when each file was made and its permissions,
    /// which the index doesn't hold. A sort by creation date reads them too.
    pub full_metadata: bool,
    /// Read when each hit was last opened, which takes another read. A sort
    /// by that date reads it too.
    pub last_opened: bool,
}

/// What the index knows about one folder's items: see [`Engine::folder_stats`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FolderStats {
    /// Bytes under each subfolder.
    pub sizes: std::collections::HashMap<PathBuf, u64>,
    /// Git status of each file that isn't clean: `modified`, `untracked`, …
    pub git: std::collections::HashMap<PathBuf, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    pub entry: FileEntry,
    pub score: i32,
    /// Byte ranges in `entry.name` to highlight.
    pub highlights: Vec<(usize, usize)>,
    pub git_status: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub total_matched: usize,
    pub total_indexed: usize,
    pub elapsed: Duration,
    /// True when some searched index was still scanning (results incomplete).
    pub indexing: bool,
    /// More items matched than the limit let into `hits`.
    pub truncated: bool,
    /// Whether a sort (see [`SearchRequest::sort`]) says which items `hits`
    /// holds. In the answer to a search made with one, it did: they are the
    /// first of the matches in its order. In the answer to a search made
    /// without, one may: the limit cut the list, and nothing in it is a
    /// match a sort would set aside.
    pub sort_chooses: bool,
}

/// How `GrepRequest::patterns` match.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GrepModeKind {
    #[default]
    Plain,
    Regex,
    /// Tolerates typos and gaps.
    Fuzzy,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GrepRequest {
    /// One or more patterns; a line matching any of them is a hit.
    pub patterns: Vec<String>,
    pub mode: GrepModeKind,
    pub scope: Option<PathBuf>,
    /// The most matching lines to return, however few files they are in;
    /// 0 for 200.
    pub limit: usize,
    /// Context lines to include around each match.
    pub before: usize,
    pub after: usize,
    /// Matches kept per file; 0 keeps up to `limit`.
    pub per_file: usize,
    /// Strip leading whitespace from lines (for display).
    pub trim: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrepHit {
    pub path: PathBuf,
    pub line: u64,
    pub column: usize,
    /// Offset of the line from the start of the file.
    pub byte_offset: u64,
    pub text: String,
    /// Byte ranges within `text`.
    pub ranges: Vec<(usize, usize)>,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

/// An indexed file or folder, for listing the whole index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexedItem {
    pub path: PathBuf,
    /// The search location it belongs to.
    pub root: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GrepResponse {
    pub hits: Vec<GrepHit>,
    pub files_with_matches: usize,
    pub files_searched: usize,
    pub elapsed: Duration,
    pub indexing: bool,
    /// The time limit (Settings › Search engine) ended the search before
    /// every file was searched.
    pub stopped: bool,
    /// More may match than `hits` holds: the search ended at its `limit`, or
    /// was told to stop, with matches left out or files left unsearched.
    pub more: bool,
}

/// A change subscription from [`Engine::watch`]; dropping it unsubscribes.
pub struct Watch {
    pub events: std::sync::mpsc::Receiver<Vec<WatchEvent>>,
    subscriptions: Vec<(SharedFilePicker, WatchId)>,
}

impl Drop for Watch {
    fn drop(&mut self) {
        for (picker, id) in &self.subscriptions {
            picker.unwatch(*id);
        }
    }
}

#[derive(Default)]
struct Indexes {
    roots: Vec<RootIndex>,
    adhoc: Vec<RootIndex>,
}

/// The folders apps are read from, and what was read.
#[derive(Default)]
struct Apps {
    dirs: Vec<PathBuf>,
    /// Read by the first search that wants it, and again once a folder in it
    /// changes.
    list: Option<Arc<AppList>>,
}

pub struct Engine {
    indexes: RwLock<Indexes>,
    apps: RwLock<Apps>,
    /// Sizes of app bundles outside the roots, by path, with the bundle's
    /// modification time when it was added up.
    bundle_sizes: RwLock<HashMap<PathBuf, (Option<SystemTime>, u64)>>,
    frecency: SharedFrecency,
    queries: SharedQueryTracker,
    watch: bool,
    search: RwLock<SearchSettings>,
    generation: AtomicU64,
}

impl Engine {
    pub fn new(options: EngineOptions) -> Arc<Engine> {
        let (frecency, queries) = open_dbs(options.db_dir.as_deref());
        let engine = Arc::new(Engine {
            indexes: RwLock::new(Indexes::default()),
            apps: RwLock::new(Apps {
                dirs: options.app_dirs,
                list: None,
            }),
            bundle_sizes: RwLock::default(),
            frecency,
            queries,
            watch: options.watch,
            search: RwLock::new(options.search),
            generation: AtomicU64::new(0),
        });
        engine.set_roots(&options.roots);
        engine
    }

    /// Replace the indexed roots. Unchanged roots keep their index.
    pub fn set_roots(&self, roots: &[PathBuf]) {
        let roots = normalize_roots(roots);
        let settings = self.search_settings();
        let mut indexes = self.indexes.write().unwrap_or_else(|e| e.into_inner());
        let mut keep: Vec<RootIndex> = Vec::new();
        for root in &roots {
            if let Some(existing) = indexes.roots.iter().find(|r| &r.path == root) {
                keep.push(existing.clone());
                continue;
            }
            match RootIndex::start(root, &self.frecency, self.watch, &settings) {
                Ok(index) => keep.push(index),
                Err(err) => log::warn!("{err}"),
            }
        }
        indexes.roots = keep;
        self.generation.fetch_add(1, Ordering::Relaxed);
    }

    /// Read apps from `dirs` from now on; empty stops finding apps.
    pub fn set_app_dirs(&self, dirs: &[PathBuf]) {
        *self.apps.write().unwrap_or_else(|e| e.into_inner()) = Apps {
            dirs: dirs.to_vec(),
            list: None,
        };
    }

    /// The app list, read again first if a folder in it changed.
    fn app_list(&self) -> Arc<AppList> {
        {
            let apps = self.apps.read().unwrap_or_else(|e| e.into_inner());
            if let Some(list) = apps.list.as_ref().filter(|l| l.is_current()) {
                return Arc::clone(list);
            }
        }
        let mut apps = self.apps.write().unwrap_or_else(|e| e.into_inner());
        let list = Arc::new(AppList::scan(&apps.dirs));
        apps.list = Some(Arc::clone(&list));
        list
    }

    pub fn search_settings(&self) -> SearchSettings {
        self.search
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Use new fff-search options. Changing how folders are indexed (symlinks,
    /// watching, git recency, content index, preloading) rebuilds every index.
    pub fn set_search_settings(&self, settings: SearchSettings) {
        let reindex = {
            let mut current = self.search.write().unwrap_or_else(|e| e.into_inner());
            let reindex = current.index_options() != settings.index_options();
            *current = settings;
            reindex
        };
        if reindex {
            let roots = self.roots();
            *self.indexes.write().unwrap_or_else(|e| e.into_inner()) = Indexes::default();
            self.set_roots(&roots);
        }
        self.generation.fetch_add(1, Ordering::Relaxed);
    }

    pub fn roots(&self) -> Vec<PathBuf> {
        self.read_indexes()
            .roots
            .iter()
            .map(|r| r.path.clone())
            .collect()
    }

    /// Bumped whenever the set of roots changes.
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Relaxed)
    }

    pub fn status(&self) -> Vec<RootStatus> {
        self.read_indexes()
            .roots
            .iter()
            .map(RootIndex::status)
            .collect()
    }

    pub fn is_scanning(&self) -> bool {
        self.roots_indexing() > 0
    }

    /// How many roots are still building their index. A search made while
    /// one is can run again as soon as this drops.
    pub fn roots_indexing(&self) -> usize {
        self.read_indexes()
            .roots
            .iter()
            .filter(|root| root.scanning())
            .count()
    }

    /// Block until every root finished its initial scan, or `timeout` passes.
    pub fn wait_ready(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let pickers: Vec<SharedFilePicker> = self
            .read_indexes()
            .roots
            .iter()
            .map(|r| r.picker.clone())
            .collect();
        pickers.iter().all(|p| {
            let left = deadline.saturating_duration_since(Instant::now());
            p.wait_for_scan(left)
        })
    }

    /// Create the index covering `scope` if needed and wait up to `timeout`
    /// for its scan. Returns whether it is ready.
    pub fn prepare(&self, scope: Option<&Path>, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        self.indexes_for(scope).iter().all(|r| {
            let left = deadline.saturating_duration_since(Instant::now());
            r.picker.wait_for_scan(left)
        })
    }

    /// Rescan every root (for example after permissions changed).
    pub fn rescan(&self) {
        for root in &self.read_indexes().roots {
            if let Err(err) = root.picker.trigger_full_rescan_async(&self.frecency) {
                log::warn!("rescan {}: {err}", root.path.display());
            }
        }
    }

    /// `scope` spelled the way a root is, when it is inside one only after
    /// resolving symlinks; `None` when it needs no change.
    fn root_spelling(&self, scope: &Path) -> Option<PathBuf> {
        let indexes = self.read_indexes();
        if indexes.roots.iter().any(|r| scope.starts_with(&r.path)) {
            return None;
        }
        let real = std::fs::canonicalize(scope).ok()?;
        indexes.roots.iter().find_map(|r| {
            let rest = real.strip_prefix(&r.canonical).ok()?;
            Some(if rest.as_os_str().is_empty() {
                r.path.clone()
            } else {
                r.path.join(rest)
            })
        })
    }

    /// Indexes covering `scope`: the roots containing it, or a temporary
    /// index of the folder itself when no root does.
    fn indexes_for(&self, scope: Option<&Path>) -> Vec<RootIndex> {
        let Some(scope) = scope else {
            return self.read_indexes().roots.clone();
        };
        {
            let indexes = self.read_indexes();
            let covering: Vec<RootIndex> = indexes
                .roots
                .iter()
                .filter(|r| scope.starts_with(&r.path))
                .cloned()
                .collect();
            if !covering.is_empty() {
                return covering;
            }
            if let Some(index) = indexes.adhoc.iter().find(|r| r.path == scope) {
                return vec![index.clone()];
            }
        }
        match RootIndex::start(scope, &self.frecency, self.watch, &self.search_settings()) {
            Ok(index) => {
                let mut indexes = self.indexes.write().unwrap_or_else(|e| e.into_inner());
                indexes.adhoc.retain(|r| r.path != scope);
                indexes.adhoc.insert(0, index.clone());
                indexes.adhoc.truncate(MAX_ADHOC);
                vec![index]
            }
            Err(err) => {
                log::warn!("{err}");
                Vec::new()
            }
        }
    }

    pub fn search(&self, req: &SearchRequest) -> SearchResponse {
        if let Some(scope) = req.scope.as_deref().and_then(|s| self.root_spelling(s)) {
            return self.search(&SearchRequest {
                scope: Some(scope),
                ..req.clone()
            });
        }
        let started = Instant::now();
        let limit = if req.limit == 0 { 500 } else { req.limit };
        // A sort starts from the list the search makes without it. That is
        // the answer unless a sort would choose its items: a list the limit
        // didn't cut holds every match in any order, and one that holds a
        // match the sort would set aside stays the best matches.
        let sort = req.sort.filter(|order| order.key != SortKey::Relevance);
        let unsorted = sort.map(|_| {
            self.search(&SearchRequest {
                sort: None,
                full_metadata: false,
                last_opened: false,
                ..req.clone()
            })
        });
        let unsorted = match unsorted {
            Some(found) if !found.sort_chooses => return finish(found, req, started),
            cut => cut,
        };
        let indexes = self.indexes_for(req.scope.as_deref());
        let indexing = indexes.iter().any(|r| r.scanning());
        let settings = self.search_settings();
        let case = settings.case;
        // fff-search reads a quoted phrase as plain fuzzy words, which find
        // every name that has it; the names are checked for it below.
        let (query, exact) = split_quoted(&req.query);
        // The searches `search_root` makes with folders in the results. In
        // them a folder token that is the whole search is a word too.
        let folders =
            req.kind == KindFilter::Folder || (req.kind == KindFilter::Any && !req.files_only);
        let typed = fold_glob_case(&query, case);
        // Words reach fff-search in lowercase, so a capital in one doesn't
        // narrow the search.
        let req = &SearchRequest {
            query: fold_word_case(&typed, folders && !folder_tokens_filter(&typed)),
            ..req.clone()
        };
        // A capital is a preference instead, and only fff-search can rank by
        // it: given `TEC` as typed, it scores `TaxExemptionController.java`
        // for its three capitals, and given `tec` it sees a t, an e and a c
        // like any other. So a search with such a word is made as typed as
        // well, and each result keeps the higher of its two scores.
        let as_typed = words_as_typed(&req.query, &typed);
        let as_typed = (as_typed != req.query).then(|| SearchRequest {
            query: as_typed,
            ..req.clone()
        });
        let as_typed = as_typed.as_ref();
        let wide = !exact.is_empty();
        // A sort that chooses a cut list's items chooses among every match,
        // so it is given as many as the phrases are checked in.
        let many = wide || sort.is_some();

        let parts: Vec<RootPart> = std::thread::scope(|s| {
            let handles: Vec<_> = indexes
                .iter()
                .map(|root| {
                    s.spawn(move || {
                        let mut part = self.search_root(root, req, limit, many);
                        if let Some(as_typed) = as_typed {
                            let ranked = self.search_root(root, as_typed, limit, many);
                            keep_higher(&mut part.hits, ranked.hits);
                        }
                        part
                    })
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });

        let mut total_matched = 0;
        let mut total_indexed = 0;
        let mut loose = false;
        let mut raw = Vec::new();
        for part in parts {
            total_matched += part.total_matched;
            total_indexed += part.total_indexed;
            loose |= part.loose;
            raw.extend(part.hits);
        }
        let words: Vec<String> = fuzzy_text(&req.query)
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        if wants_apps(req, &words) {
            let mut apps = app_hits(self.app_list().entries(), &words);
            // fff-search isn't asked about apps, so they get here what the
            // search as typed gives a name that fits its capitals. Without
            // it, `Safari` lists a file named `Safari.txt` ahead of the app.
            if let Some(as_typed) = as_typed {
                for app in &mut apps {
                    app.score += case_fit(&display_name(&app.path), &as_typed.query);
                }
            }
            total_matched += apps.len();
            raw.extend(apps);
        }
        if wide {
            raw.retain(|hit| {
                let name = display_name(&hit.path);
                exact
                    .iter()
                    .all(|phrase| contains_phrase(&name, phrase, case))
            });
            total_matched = raw.len();
        }
        // Typo-tolerant matches are a fallback: when some results contain every
        // word as an in-order subsequence (fzf-style), keep only those. With
        // typos turned off, they're never kept.
        let strict = |hit: &RawHit| words.iter().all(|w| is_subsequence(w, &hit.rel));
        if !words.is_empty() && (!settings.typos || raw.iter().any(&strict)) {
            raw.retain(strict);
        }
        for hit in &mut raw {
            hit.score = rerank(hit, &words);
        }
        raw.sort_by_key(|h| std::cmp::Reverse(h.score));
        let mut seen = HashSet::new();
        raw.retain(|h| seen.insert(h.path.clone()));

        let needle = fuzzy_text(&req.query);
        let (hits, truncated) = match sort.zip(unsorted) {
            Some((order, unsorted)) => {
                match cut_in_order(raw, order, limit, &words, &needle, req.show_hidden) {
                    Some(chosen) => chosen,
                    // Among every match, the sort had nothing to choose.
                    None => {
                        let found = SearchResponse {
                            sort_chooses: false,
                            ..unsorted
                        };
                        return finish(found, req, started);
                    }
                }
            }
            None => cut_to_limit(raw, limit, &needle, req.show_hidden),
        };
        let found = SearchResponse {
            sort_chooses: truncated && sort_can_choose(&hits, &words, loose),
            hits,
            total_matched,
            total_indexed,
            elapsed: started.elapsed(),
            indexing,
            truncated,
        };
        finish(found, req, started)
    }

    /// Search one root. `wide` fetches as many candidates as a one-letter
    /// search does, for a filter or a sort the results go through afterwards.
    fn search_root(
        &self,
        root: &RootIndex,
        req: &SearchRequest,
        limit: usize,
        wide: bool,
    ) -> RootPart {
        let mut part = RootPart::default();
        let Ok(guard) = root.picker.read() else {
            return part;
        };
        let Some(picker) = guard.as_ref() else {
            return part;
        };
        let base = picker.base_path().to_path_buf();
        part.total_indexed = picker.live_file_count();
        let scope = req.scope.as_deref().map(|s| root.in_base(s, &base));

        let scope_rel = scope
            .as_deref()
            .filter(|s| *s != base.as_path())
            .and_then(|s| s.strip_prefix(&base).ok())
            .map(|rel| rel.to_string_lossy().replace('\\', "/"));
        // A glob narrows the search inside fff; the prefix check below is exact.
        let scope_glob = scope_rel
            .as_deref()
            .filter(|rel| {
                !rel.chars()
                    .any(|c| c.is_whitespace() || "*?[]{}!\\".contains(c))
            })
            .map(|rel| format!("{rel}/**"));

        let user = req.query.trim();
        let single_char =
            fuzzy_text(user).chars().count() == 1 && !user.contains(char::is_whitespace);
        let mut query = if single_char {
            String::new()
        } else {
            user.to_string()
        };
        query.push_str(&kind_tokens(req.kind, user));
        if let Some(glob) = &scope_glob {
            query.push(' ');
            query.push_str(glob);
        }
        let mut query = query.trim().to_string();
        // In mixed search a query ending in `/` means "folders only"; keep that
        // for a lone folder token but not for "main src/"-style path filters,
        // which go before the tokens that don't end in `/`.
        if req.kind == KindFilter::Any && query.ends_with('/') {
            let mut tokens: Vec<&str> = query.split_whitespace().collect();
            tokens.sort_by_key(|token| !token.ends_with('/'));
            query = tokens.join(" ");
        }

        // fff ignores one-character words, so over-fetch and filter by name.
        // Over-fetch so the re-rank and package filter have candidates to work with.
        let fetch = if wide || single_char || (scope_rel.is_some() && scope_glob.is_none()) {
            limit.max(20_000)
        } else {
            (limit * 4).max(2_000)
        };
        let qt_guard = self.queries.read().ok();
        let qt = qt_guard.as_ref().and_then(|g| g.as_ref());
        let options = FuzzySearchOptions {
            max_threads: self.search_settings().threads,
            project_path: Some(&base),
            pagination: PaginationArgs {
                offset: 0,
                limit: fetch,
            },
            ..Default::default()
        };

        let in_scope = |path: &Path| {
            let scoped = match &scope {
                Some(scope) => path.starts_with(scope) && path != scope,
                None => true,
            };
            scoped && !inside_package(path, scope.as_deref())
        };
        let name_ok = |name: &str| {
            !single_char
                || name
                    .to_lowercase()
                    .contains(&fuzzy_text(user).to_lowercase())
        };

        // `query` may have the searched folder's glob in it by now, and a
        // folder token alone is the whole search all the same.
        let folder_filters = folder_tokens_filter(user);
        if req.kind == KindFilter::Folder {
            let config = WithFolders {
                only: true,
                folder_filters,
            };
            let parsed = QueryParser::new(config).parse(&query);
            part.loose = loose_text(&parsed.fuzzy_query, user);
            let result = picker.fuzzy_search_directories(&parsed, options);
            part.total_matched = result.total_matched;
            for (item, score) in result.items.iter().zip(&result.scores) {
                let rel = item.relative_path(picker);
                if rel.is_empty() {
                    continue;
                }
                let path = join_rel(&base, &rel);
                let name = display_name(&path);
                if in_scope(&path) && name_ok(&name) {
                    part.hits.push(RawHit::dir(path, score.total));
                }
            }
        } else if req.kind == KindFilter::Any && !req.files_only {
            let config = WithFolders {
                only: false,
                folder_filters,
            };
            let parsed = QueryParser::new(config).parse(&query);
            part.loose = loose_text(&parsed.fuzzy_query, user);
            let result = picker.fuzzy_search_mixed(&parsed, qt, options);
            part.total_matched = result.total_matched;
            for (item, score) in result.items.iter().zip(&result.scores) {
                let hit = match item {
                    MixedItemRef::File(f) => {
                        let path = join_rel(&base, &f.relative_path(picker));
                        RawHit::file(
                            path,
                            f.size,
                            f.modified,
                            fff_search::git::format_git_status_opt(f.git_status),
                            score.total,
                        )
                    }
                    MixedItemRef::Dir(d) => {
                        let rel = d.relative_path(picker);
                        if rel.is_empty() {
                            continue;
                        }
                        RawHit::dir(join_rel(&base, &rel), score.total)
                    }
                };
                if in_scope(&hit.path) && name_ok(&display_name(&hit.path)) {
                    part.hits.push(hit);
                }
            }
        } else {
            let parsed = QueryParser::default().parse(&query);
            part.loose = loose_text(&parsed.fuzzy_query, user);
            let result = picker.fuzzy_search(&parsed, qt, options);
            part.total_matched = result.total_matched;
            for (item, score) in result.items.iter().zip(&result.scores) {
                let path = join_rel(&base, &item.relative_path(picker));
                if in_scope(&path) && req.kind.accepts(&path) && name_ok(&display_name(&path)) {
                    part.hits.push(RawHit::file(
                        path,
                        item.size,
                        item.modified,
                        fff_search::git::format_git_status_opt(item.git_status),
                        score.total,
                    ));
                }
            }
        }
        // Each hit's path from the searched folder down, which is where
        // `search` looks for the words.
        let searched = scope.as_deref().unwrap_or(&base);
        for hit in &mut part.hits {
            hit.rel = hit
                .path
                .strip_prefix(searched)
                .unwrap_or(&hit.path)
                .to_string_lossy()
                .to_lowercase();
        }
        part
    }

    /// Absolute paths of indexed files matching `pattern` (a glob over the
    /// path inside each root, like `**/*.app/Contents/Info.plist`), across
    /// every root. Nothing is re-ranked or hidden, package contents included.
    pub fn glob(&self, pattern: &str, limit: usize) -> Vec<PathBuf> {
        let mut found = Vec::new();
        for root in &self.read_indexes().roots {
            let Ok(guard) = root.picker.read() else {
                continue;
            };
            let Some(picker) = guard.as_ref() else {
                continue;
            };
            let base = picker.base_path().to_path_buf();
            let result = picker.glob(
                pattern,
                FuzzySearchOptions {
                    project_path: Some(&base),
                    pagination: PaginationArgs { offset: 0, limit },
                    ..Default::default()
                },
            );
            found.extend(
                result
                    .items
                    .iter()
                    .map(|item| join_rel(&base, &item.relative_path(picker))),
            );
        }
        found.sort();
        found.dedup();
        found.truncate(limit);
        found
    }

    /// Files modified in the last 30 days across the roots, newest first,
    /// skipping hidden paths, `Library` folders, and package contents.
    pub fn recents(&self, limit: usize) -> Vec<FileEntry> {
        const WINDOW: Duration = Duration::from_secs(30 * 24 * 60 * 60);
        let cutoff = SystemTime::now()
            .checked_sub(WINDOW)
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs());
        let mut found: Vec<(u64, PathBuf, u64)> = Vec::new();
        for root in &self.read_indexes().roots {
            let Ok(guard) = root.picker.read() else {
                continue;
            };
            let Some(picker) = guard.as_ref() else {
                continue;
            };
            let base = picker.base_path().to_path_buf();
            let query = QueryParser::default().parse("");
            // An empty query ranks by frecency, then by newest modification.
            let result = picker.fuzzy_search(
                &query,
                None,
                FuzzySearchOptions {
                    pagination: PaginationArgs {
                        offset: 0,
                        limit: (limit * 4).max(4_000),
                    },
                    ..Default::default()
                },
            );
            for item in result.items {
                if item.modified < cutoff {
                    continue;
                }
                let rel = item.relative_path(picker);
                if rel
                    .split('/')
                    .any(|part| part.starts_with('.') || part == "Library")
                {
                    continue;
                }
                let path = join_rel(&base, &rel);
                if !inside_package(&path, None) {
                    found.push((item.modified, path, item.size));
                }
            }
        }
        found.sort_by_key(|f| std::cmp::Reverse(f.0));
        found.truncate(limit);
        found
            .into_iter()
            .filter_map(|(modified, path, size)| {
                RawHit::file(path, size, modified, None, 0)
                    .into_hit("", false)
                    .map(|hit| hit.entry)
            })
            .collect()
    }

    /// Content search across the indexes covering `scope`.
    pub fn grep(&self, req: &GrepRequest) -> GrepResponse {
        self.grep_cancellable(req, None)
    }

    /// [`Self::grep`] for a caller that may stop waiting for the answer.
    /// Once `abort` is set, fff-search notices within a few files and stops
    /// reading, the roots not yet reached are left out, and what was found
    /// so far comes back.
    pub fn grep_cancellable(
        &self,
        req: &GrepRequest,
        abort: Option<&Arc<AtomicBool>>,
    ) -> GrepResponse {
        if let Some(scope) = req.scope.as_deref().and_then(|s| self.root_spelling(s)) {
            let req = GrepRequest {
                scope: Some(scope),
                ..req.clone()
            };
            return self.grep_cancellable(&req, abort);
        }
        let settings = self.search_settings();
        let started = Instant::now();
        let limit = if req.limit == 0 { 200 } else { req.limit };
        let indexes = self.indexes_for(req.scope.as_deref());
        let mut response = GrepResponse {
            indexing: indexes.iter().any(|r| r.scanning()),
            ..Default::default()
        };
        let mut patterns: Vec<String> = req
            .patterns
            .iter()
            .filter(|p| !p.trim().is_empty())
            .cloned()
            .collect();
        if patterns.is_empty() {
            return response;
        }
        // fff matches several patterns as literal text; several regexes
        // mean "any of these", so join them into one.
        if req.mode == GrepModeKind::Regex && patterns.len() > 1 {
            let joined = patterns
                .iter()
                .map(|p| format!("(?:{p})"))
                .collect::<Vec<_>>()
                .join("|");
            patterns = vec![joined];
        }
        let patterns: Vec<&str> = patterns.iter().map(String::as_str).collect();
        for (ix, root) in indexes.iter().enumerate() {
            let Ok(guard) = root.picker.read() else {
                continue;
            };
            let Some(picker) = guard.as_ref() else {
                continue;
            };
            // The time limit is for the whole search. fff-search counts from
            // zero each time it is called, so a root gets what the roots
            // before it left, and none is searched once the time is up.
            let Some(budget_ms) = root_budget_ms(settings.time_limit_secs, started.elapsed())
            else {
                response.stopped = true;
                break;
            };
            let root_started = Instant::now();
            let base = picker.base_path().to_path_buf();
            let scope = req.scope.as_deref().map(|s| root.in_base(s, &base));
            let scope_rel = scope
                .as_deref()
                .filter(|s| *s != base.as_path())
                .and_then(|s| s.strip_prefix(&base).ok())
                .map(|rel| rel.to_string_lossy().replace('\\', "/"));
            // A scope inside this root narrows the search with a glob.
            let scope_glob = scope_rel
                .as_deref()
                .filter(|rel| !rel.contains(char::is_whitespace))
                .map(|rel| format!(" {rel}/**"));
            // No glob is made for a folder with a space in its path: the
            // whole root is searched, and the hits outside the folder are
            // dropped below. fff-search ends at a full page wherever its
            // matches are, so the page is larger than `limit`, to hold
            // enough of the folder's own.
            let page_limit = if scope_rel.is_some() && scope_glob.is_none() {
                limit.saturating_mul(4)
            } else {
                limit
            };
            let options = GrepSearchOptions {
                mode: match req.mode {
                    GrepModeKind::Plain => GrepMode::PlainText,
                    GrepModeKind::Regex => GrepMode::Regex,
                    GrepModeKind::Fuzzy => GrepMode::Fuzzy,
                },
                page_limit,
                max_matches_per_file: if req.per_file == 0 {
                    limit
                } else {
                    req.per_file
                },
                max_file_size: settings.max_file_size_mb.max(1) * 1024 * 1024,
                casing: Some(match settings.case {
                    SearchCase::Smart => Casing::Smart,
                    SearchCase::Sensitive => Casing::Sensitive,
                    SearchCase::Insensitive => Casing::Insensitive,
                }),
                time_budget_ms: budget_ms,
                // fff-search otherwise waits for a first match before
                // counting, so a search that finds nothing ran to the end.
                enforce_time_budget: true,
                trim_whitespace: req.trim,
                before_context: req.before,
                after_context: req.after,
                abort_signal: abort.cloned(),
                ..Default::default()
            };
            let parser = QueryParser::new(GrepConfig);
            let result = if let [only] = patterns.as_slice() {
                let text = format!("{only}{}", scope_glob.as_deref().unwrap_or(""));
                picker.grep(&parser.parse(&text), &options)
            } else {
                // Each pattern may carry file constraints; the rest is text.
                let mut constraints = Vec::new();
                let mut texts: Vec<String> = Vec::new();
                let scoped: Vec<String> = patterns
                    .iter()
                    .enumerate()
                    .map(|(i, p)| match (&scope_glob, i) {
                        (Some(glob), 0) => format!("{p}{glob}"),
                        _ => p.to_string(),
                    })
                    .collect();
                for pattern in &scoped {
                    let query = parser.parse(pattern);
                    let text = query.grep_text();
                    constraints.extend(query.constraints);
                    if !text.is_empty() {
                        texts.push(text);
                    }
                }
                let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
                picker.multi_grep(&refs, &constraints, &options)
            };
            // An abort has cut this root short, which isn't the time limit's
            // doing, and the roots after it aren't searched.
            let aborted = abort.is_some_and(|flag| flag.load(Ordering::Relaxed));
            let out_of_time = !aborted
                && budget_ms > 0
                && root_started.elapsed() >= Duration::from_millis(budget_ms);
            match page_end(
                result.matches.len(),
                page_limit,
                result.next_file_offset,
                out_of_time,
            ) {
                PageEnd::Complete => {}
                PageEnd::Full => response.more = true,
                PageEnd::Stopped => response.stopped = true,
            }
            response.files_with_matches += result.files_with_matches;
            response.files_searched += result.total_files_searched;
            for (n, m) in result.matches.iter().enumerate() {
                let Some(file) = result.files.get(m.file_index) else {
                    continue;
                };
                let path = join_rel(&base, &file.relative_path(picker));
                if let Some(scope) = &scope
                    && !path.starts_with(scope)
                {
                    continue;
                }
                response.hits.push(GrepHit {
                    path,
                    line: m.line_number,
                    column: m.col,
                    byte_offset: m.byte_offset,
                    text: m.line_content.clone(),
                    ranges: m
                        .match_byte_offsets
                        .iter()
                        .map(|&(a, b)| (a as usize, b as usize))
                        .collect(),
                    context_before: m.context_before.clone(),
                    context_after: m.context_after.clone(),
                });
                if response.hits.len() >= limit {
                    // The rest of this root's matches are left out.
                    response.more |= n + 1 < result.matches.len();
                    break;
                }
            }
            if response.hits.len() >= limit || aborted {
                // So is whatever the roots after it hold.
                response.more |= ix + 1 < indexes.len();
                break;
            }
        }
        response.elapsed = started.elapsed();
        response
    }

    /// Every indexed file, or folder, across the roots; with a query, the
    /// fuzzy matches in fff's own order. At most `limit` (0 for no limit).
    pub fn list(&self, dirs: bool, query: Option<&str>, limit: usize) -> Vec<IndexedItem> {
        let limit = if limit == 0 { usize::MAX } else { limit };
        let threads = self.search_settings().threads;
        // Words ignore letter case here as they do in `search`.
        let folded = query.map(|q| fold_word_case(q, dirs));
        let mut items = Vec::new();
        for root in &self.read_indexes().roots {
            if items.len() >= limit {
                break;
            }
            let Ok(guard) = root.picker.read() else {
                continue;
            };
            let Some(picker) = guard.as_ref() else {
                continue;
            };
            let base = picker.base_path().to_path_buf();
            let left = limit - items.len();
            let item = |rel: String, is_dir: bool, size: u64, modified: u64| IndexedItem {
                path: join_rel(&base, &rel),
                root: base.clone(),
                is_dir,
                size,
                modified,
            };
            match (dirs, folded.as_deref().filter(|q| !q.is_empty())) {
                // The overflow files (added since the scan) are the end of
                // `get_files`, so they're already in it.
                (false, None) => items.extend(
                    picker
                        .get_files()
                        .iter()
                        .filter(|f| !f.is_deleted())
                        .take(left)
                        .map(|f| item(f.relative_path(picker), false, f.size, f.modified)),
                ),
                (true, None) => items.extend(
                    picker
                        .get_dirs()
                        .iter()
                        .filter(|d| !d.is_deleted())
                        .map(|d| d.relative_path(picker))
                        .filter(|rel| !rel.is_empty())
                        .take(left)
                        .map(|rel| item(rel, true, 0, 0)),
                ),
                (false, Some(q)) => {
                    let parsed = QueryParser::default().parse(q);
                    let options = FuzzySearchOptions {
                        max_threads: threads,
                        project_path: Some(&base),
                        pagination: PaginationArgs {
                            offset: 0,
                            limit: left.min(100_000),
                        },
                        ..Default::default()
                    };
                    let result = picker.fuzzy_search(&parsed, None, options);
                    items.extend(
                        result
                            .items
                            .iter()
                            .map(|f| item(f.relative_path(picker), false, f.size, f.modified)),
                    );
                }
                (true, Some(q)) => {
                    let parsed = QueryParser::new(DirSearchConfig).parse(q);
                    let options = FuzzySearchOptions {
                        max_threads: threads,
                        project_path: Some(&base),
                        pagination: PaginationArgs {
                            offset: 0,
                            limit: left.min(100_000),
                        },
                        ..Default::default()
                    };
                    let result = picker.fuzzy_search_directories(&parsed, options);
                    items.extend(
                        result
                            .items
                            .iter()
                            .map(|d| d.relative_path(picker))
                            .filter(|rel| !rel.is_empty())
                            .map(|rel| item(rel, true, 0, 0)),
                    );
                }
            }
        }
        if dirs && items.len() < limit {
            let words: Vec<String> = query
                .unwrap_or_default()
                .split_whitespace()
                .map(str::to_lowercase)
                .collect();
            let roots = self.roots();
            let list = self.app_list();
            let apps = list
                .entries()
                .iter()
                .filter(|p| !roots.iter().any(|r| p.starts_with(r)))
                .filter(|p| name_holds(p, &words))
                .map(|p| IndexedItem {
                    path: p.clone(),
                    root: list.dir_of(p).unwrap_or(p).to_path_buf(),
                    is_dir: true,
                    size: 0,
                    modified: 0,
                });
            items.extend(apps.take(limit - items.len()));
        }
        items.truncate(limit);
        items
    }

    /// Sizes of the app bundles directly in `dir`, an app folder no root
    /// covers, which the index can't add up, so the app folders show app
    /// sizes as Finder does. The bundles come from the app list, so no other
    /// folder is read. Each is walked once, bundles in parallel, and added up
    /// again only when its modification time changes.
    ///
    /// # Returns
    ///
    /// Bytes in each bundle, by path; empty for any other folder
    pub fn bundle_sizes(&self, dir: &Path) -> HashMap<PathBuf, u64> {
        if self.roots().iter().any(|r| dir.starts_with(r)) {
            return HashMap::new();
        }
        let list = self.app_list();
        let bundles: Vec<(PathBuf, Option<SystemTime>)> = list
            .apps()
            .filter(|p| p.parent() == Some(dir))
            .map(|p| {
                let time = std::fs::metadata(p).and_then(|m| m.modified()).ok();
                (p.clone(), time)
            })
            .collect();
        let mut sizes = HashMap::with_capacity(bundles.len());
        let mut todo = Vec::new();
        {
            let known = self.bundle_sizes.read().unwrap_or_else(|e| e.into_inner());
            for (path, time) in bundles {
                match known.get(&path) {
                    Some((then, bytes)) if *then == time => {
                        sizes.insert(path, *bytes);
                    }
                    _ => todo.push((path, time)),
                }
            }
        }
        let never = AtomicBool::new(false);
        let added: Vec<(PathBuf, Option<SystemTime>, u64)> = todo
            .into_par_iter()
            .map(|(path, time)| {
                let bytes = crate::info::folder_size(&path, &never).bytes;
                (path, time, bytes)
            })
            .collect();
        let mut known = self.bundle_sizes.write().unwrap_or_else(|e| e.into_inner());
        for (path, time, bytes) in added {
            sizes.insert(path.clone(), bytes);
            known.insert(path, (time, bytes));
        }
        sizes
    }

    /// Folder sizes and git statuses for the items directly in `dir`, read
    /// from the index without touching the disk: each subfolder's total is
    /// every indexed file under it, and each file's git status is what the
    /// index last saw. Folders and files the index skips aren't included.
    pub fn folder_stats(&self, dir: &Path) -> FolderStats {
        self.folder_stats_for(&[dir.to_path_buf()])
            .pop()
            .unwrap_or_default()
    }

    /// [`Self::folder_stats`] for several folders at once, in the order
    /// given, with one pass over each root's files for all of them.
    pub fn folder_stats_for(&self, dirs: &[PathBuf]) -> Vec<FolderStats> {
        let mut all = vec![FolderStats::default(); dirs.len()];
        let indexes = self.read_indexes();
        // `set_roots` leaves out roots inside others, so no file counts twice.
        for root in &indexes.roots {
            let Ok(guard) = root.picker.read() else {
                continue;
            };
            let Some(picker) = guard.as_ref() else {
                continue;
            };
            // The overflow files (added since the scan) are the end of
            // `get_files`, so they're already in it.
            let files = || picker.get_files().iter().filter(|f| !f.is_deleted());
            let base = picker.base_path();
            let mut root_total: Option<u64> = None;
            // Each folder in this root, as the start of fff-search's relative
            // paths: `/` between names on every system, and one at the end.
            let mut wanted: Vec<(usize, String)> = Vec::new();
            for (ix, dir) in dirs.iter().enumerate() {
                // A root inside `dir` adds up as one of its subfolders.
                if let Ok(below) = root.path.strip_prefix(dir)
                    && let Some(first) = below.components().next()
                {
                    let total = *root_total.get_or_insert_with(|| files().map(|f| f.size).sum());
                    *all[ix].sizes.entry(dir.join(first)).or_default() += total;
                    continue;
                }
                if let Ok(rel) = root.in_base(dir, base).strip_prefix(base) {
                    let mut prefix = rel
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/");
                    if !prefix.is_empty() {
                        prefix.push('/');
                    }
                    wanted.push((ix, prefix));
                }
            }
            if wanted.is_empty() {
                continue;
            }
            // A file can only be under a folder whose path is no longer than
            // its own folder's, which is known without reading the path.
            let shortest = wanted.iter().map(|(_, p)| p.len()).min().unwrap_or(0);
            for file in files() {
                if file.filename_offset_in_relative_path() < shortest {
                    continue;
                }
                let path = file.relative_path(picker);
                for (ix, prefix) in &wanted {
                    let Some(rest) = path.strip_prefix(prefix.as_str()) else {
                        continue;
                    };
                    let (dir, stats) = (&dirs[*ix], &mut all[*ix]);
                    match rest.split_once('/') {
                        Some((child, _)) => {
                            *stats.sizes.entry(dir.join(child)).or_default() += file.size;
                        }
                        None => {
                            if let Some(status) =
                                fff_search::git::format_git_status_opt(file.git_status)
                                    .filter(|s| *s != "clean" && *s != "unknown")
                            {
                                stats.git.insert(dir.join(rest), status.to_string());
                            }
                        }
                    }
                }
            }
        }
        all
    }

    /// Subscribe to changes under the roots. `pattern` is a glob relative to
    /// each root, an absolute path, or empty for everything. Events arrive in
    /// debounced batches until the returned [`Watch`] is dropped.
    pub fn watch(&self, pattern: &str, ignore: &[String]) -> anyhow::Result<Watch> {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut subscriptions = Vec::new();
        let mut last_error = None;
        let pattern = pattern.trim();
        for root in &self.read_indexes().roots {
            // An absolute pattern only applies to the root that holds it.
            if Path::new(pattern).is_absolute() && !Path::new(pattern).starts_with(&root.path) {
                continue;
            }
            let tx = tx.clone();
            let options = WatchOptions {
                ignore: ignore.to_vec(),
            };
            match root.picker.watch(pattern, options, move |_, events| {
                let _ = tx.send(events.to_vec());
            }) {
                Ok(id) => subscriptions.push((root.picker.clone(), id)),
                Err(err) => last_error = Some(err),
            }
        }
        if subscriptions.is_empty() {
            return Err(match last_error {
                Some(err) => anyhow::anyhow!("cannot watch {pattern:?}: {err}"),
                None => anyhow::anyhow!("{pattern:?} is outside every search location"),
            });
        }
        Ok(Watch {
            events: rx,
            subscriptions,
        })
    }

    /// Record that the user opened `path`, boosting it in later searches.
    pub fn track_open(&self, path: &Path, query: Option<&str>) {
        let Ok(frecency) = self.frecency.read() else {
            return;
        };
        let Some(tracker) = frecency.as_ref() else {
            return;
        };
        if let Err(err) = tracker.track_access(path) {
            log::debug!("track_access {}: {err}", path.display());
            return;
        }
        // fff-search looks a search up under the text it is given, so a
        // search is remembered under the lowercase text `search` gives it,
        // in the form that takes when folders can be in the results, as they
        // can in the window.
        let query = query.map(|q| {
            let (query, _) = split_quoted(q);
            let typed = fold_glob_case(&query, self.search_settings().case);
            fold_word_case(&typed, !folder_tokens_filter(&typed))
        });
        let roots = self.read_indexes().roots.clone();
        for root in roots.iter().filter(|r| path.starts_with(&r.path)) {
            if let Ok(mut guard) = root.picker.write()
                && let Some(picker) = guard.as_mut()
            {
                let _ = picker.update_single_file_frecency(path, tracker);
            }
            if let Some(query) = query.as_deref().filter(|q| !q.is_empty())
                && let Ok(mut qt) = self.queries.write()
                && let Some(qt) = qt.as_mut()
            {
                let _ = qt.track_query_completion(query, &root.path, path);
            }
        }
    }

    fn read_indexes(&self) -> std::sync::RwLockReadGuard<'_, Indexes> {
        self.indexes.read().unwrap_or_else(|e| e.into_inner())
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let indexes = self.indexes.get_mut().unwrap_or_else(|e| e.into_inner());
        for root in indexes.roots.iter().chain(indexes.adhoc.iter()) {
            root.picker.cancel();
        }
    }
}

#[derive(Default)]
struct RootPart {
    hits: Vec<RawHit>,
    total_matched: usize,
    total_indexed: usize,
    /// fff-search matched text loosely that isn't one of the search's
    /// words: see [`loose_text`].
    loose: bool,
}

struct RawHit {
    path: PathBuf,
    /// Its path inside the folder that was searched, or an app's name alone,
    /// in lowercase, which is where a search's words are looked for. Nothing
    /// above the search location is in it, because fff-search matches nothing
    /// there: letters found in `/Users/alice` would pass a near miss off as a
    /// match. Empty when no search made the hit.
    rel: String,
    is_dir: bool,
    size: u64,
    modified: u64,
    git_status: Option<&'static str>,
    score: i32,
}

impl RawHit {
    fn file(
        path: PathBuf,
        size: u64,
        modified: u64,
        git_status: Option<&'static str>,
        score: i32,
    ) -> RawHit {
        RawHit {
            path,
            rel: String::new(),
            is_dir: false,
            size,
            modified,
            git_status,
            score,
        }
    }

    fn dir(path: PathBuf, score: i32) -> RawHit {
        RawHit {
            path,
            rel: String::new(),
            is_dir: true,
            size: 0,
            modified: 0,
            git_status: None,
            score,
        }
    }

    /// When the index says a file last changed.
    fn modified_time(&self) -> Option<SystemTime> {
        (self.modified > 0).then(|| SystemTime::UNIX_EPOCH + Duration::from_secs(self.modified))
    }

    fn into_hit(self, needle: &str, show_hidden: bool) -> Option<SearchHit> {
        let entry = if self.is_dir {
            // Directory rows carry no metadata in the index.
            FileEntry::from_path(&self.path)?
        } else {
            let name = display_name(&self.path);
            FileEntry {
                kind: kind::classify(&self.path, false),
                hidden: name.starts_with('.'),
                name,
                is_dir: false,
                size: Some(self.size),
                modified: self.modified_time(),
                is_symlink: false,
                created: None,
                mode: None,
                last_opened: None,
                readonly: false,
                path: self.path,
            }
        };
        if entry.hidden && !show_hidden {
            return None;
        }
        Some(SearchHit {
            highlights: highlight(&entry.name, needle),
            entry,
            score: self.score,
            git_status: self
                .git_status
                .filter(|s| *s != "clean" && *s != "unknown")
                .map(str::to_owned),
        })
    }
}

/// The first `limit` of a search's hits that can be shown, and whether that
/// left any out. A hit that can't be shown takes no place among them: a
/// hidden file while hidden files are off, or a folder that is gone. Were
/// the list cut before those are known, it would come out short of `limit`
/// with items left over that should have been in it.
///
/// # Arguments
///
/// * `raw` - The hits, in the order they're listed in
/// * `limit` - The most to return
/// * `needle` - The search's fuzzy words, for the highlights
/// * `show_hidden` - Whether hidden files are shown
///
/// # Returns
///
/// The hits to show, and whether more could have been
fn cut_to_limit(
    raw: Vec<RawHit>,
    limit: usize,
    needle: &str,
    show_hidden: bool,
) -> (Vec<SearchHit>, bool) {
    let mut shown = raw
        .into_iter()
        .filter_map(|hit| hit.into_hit(needle, show_hidden));
    let hits: Vec<SearchHit> = shown.by_ref().take(limit).collect();
    let truncated = shown.next().is_some();
    (hits, truncated)
}

/// Finish a search's answer, once its hits are no more than the limit: read
/// from the disk what the request asks to have of each, and time the search
/// to here.
///
/// # Arguments
///
/// * `found` - The answer, with the hits the search lists
/// * `req` - What was asked for
/// * `started` - When the search began
fn finish(mut found: SearchResponse, req: &SearchRequest, started: Instant) -> SearchResponse {
    let by = |key: SortKey| req.sort.is_some_and(|order| order.key == key);
    let metadata = req.full_metadata || by(SortKey::Created);
    let last_opened = req.last_opened || by(SortKey::LastOpened);
    for hit in &mut found.hits {
        // A folder's entry was read from the disk already.
        if metadata && !hit.entry.is_dir {
            hit.entry.read_metadata();
        }
        if last_opened {
            hit.entry.read_last_opened();
        }
    }
    found.elapsed = started.elapsed();
    found
}

/// The list a sort chooses for a search whose list the limit cut: the first
/// `limit` of the hits [`sort_before_cut`] puts in order that can be shown.
///
/// # Arguments
///
/// * `raw` - The hits, best first
/// * `order` - What to sort by, which isn't relevance
/// * `limit` - The most the search returns
/// * `words` - The search's fuzzy words, in lowercase
/// * `needle` - The search's fuzzy words, for the highlights
/// * `show_hidden` - Whether hidden files are shown
///
/// # Returns
///
/// The hits, best first like any answer, and whether the limit left any
/// out. `None` when the search lists what it lists without the sort
fn cut_in_order(
    mut raw: Vec<RawHit>,
    order: SortOrder,
    limit: usize,
    words: &[String],
    needle: &str,
    show_hidden: bool,
) -> Option<(Vec<SearchHit>, bool)> {
    let ranked = raw.len();
    if !sort_before_cut(&mut raw, order, limit, words) {
        return None;
    }
    // A sort sets matches aside: the ones without the words in their names,
    // and the ones past the most it reads from the disk.
    let set_aside = raw.len() < ranked;
    let (mut hits, truncated) = cut_to_limit(raw, limit, needle, show_hidden);
    // The hits it kept can all fit after all, as when hidden files are
    // among them. The list then has room for matches that were set aside.
    if set_aside && !truncated {
        return None;
    }
    hits.sort_by_key(|hit| std::cmp::Reverse(hit.score));
    Some((hits, truncated))
}

/// Whether a sort may choose which of a search's matches its list holds,
/// going by the hits the search lists without one, where the limit has cut
/// them. [`sort_before_cut`] keeps to the names that have every word, and a
/// list with any other hit in it is left as the ranking made it. Either the
/// names with the words don't fill the list, and a sort has nothing to
/// choose, or the ranking puts something ahead of them that the sort would
/// drop: the file opened from this search before ranks first wherever the
/// words are in its path.
///
/// # Arguments
///
/// * `hits` - The hits the search lists, best first
/// * `words` - The search's fuzzy words, in lowercase
/// * `loose` - Whether fff-search matched text loosely that the words leave
///   out ([`loose_text`]). Nothing here tells its strong matches from its
///   weak ones
fn sort_can_choose(hits: &[SearchHit], words: &[String], loose: bool) -> bool {
    !loose && hits.iter().all(|hit| name_has(&hit.entry.path, words))
}

/// The most hits a sort reads from the disk to put them in order, for the
/// dates the index doesn't hold.
const SORT_READS: usize = 5_000;

/// Put a search's hits in the order of a sort before the limit cuts the
/// list, so that a sort by size lists the largest of every match and not the
/// largest of the best-ranked few.
///
/// Words match loosely: their letters can be anywhere along the path. Sorted
/// by size or date, the weakest of thousands of such matches would bury the
/// items that were meant. So a search with words is sorted among the hits
/// that have every word in their name, the ones [`rerank`] puts first, and
/// only when there are more of those than `limit`. Otherwise nothing is
/// done, and the search lists what it lists without the sort. Whether a
/// sort may choose at all is for [`sort_can_choose`] to say first, from the
/// list the search makes without it.
///
/// # Arguments
///
/// * `raw` - The hits, best first; when sorted, left with the ones the sort
///   chooses among, in its order
/// * `order` - What to sort by, which isn't relevance
/// * `limit` - The most the search returns
/// * `words` - The search's fuzzy words, in lowercase
///
/// # Returns
///
/// Whether the hits were sorted
fn sort_before_cut(
    raw: &mut Vec<RawHit>,
    order: SortOrder,
    limit: usize,
    words: &[String],
) -> bool {
    let named = |hit: &RawHit| name_has(&hit.path, words);
    if raw.iter().filter(|hit| named(hit)).count() <= limit {
        return false;
    }
    raw.retain(named);
    // A date the index doesn't hold takes a read of the hit, so only the
    // best of those hits are read, and the sort goes without the rest.
    let mut reads = limit.max(SORT_READS);
    raw.retain(|hit| {
        if !CutKey::reads_disk(hit, order.key) {
            return true;
        }
        let read = reads > 0;
        reads = reads.saturating_sub(1);
        read
    });
    let mut keyed: Vec<(CutKey, String, RawHit)> = raw
        .drain(..)
        .map(|hit| (CutKey::of(&hit, order.key), display_name(&hit.path), hit))
        .collect();
    // As `listing::compare` orders the entries the hits become. The sort is
    // stable, so hits it can't tell apart stay best first.
    keyed.sort_by(|(a, a_name, _), (b, b_name, _)| {
        let ord = a.cmp(b).then_with(|| listing::natural_cmp(a_name, b_name));
        if order.ascending { ord } else { ord.reverse() }
    });
    raw.extend(keyed.into_iter().map(|(_, _, hit)| hit));
    true
}

/// What [`sort_before_cut`] compares a hit by: what [`listing::compare`]
/// reads from the entry the hit becomes, made once for each hit.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum CutKey {
    /// Nothing but the name, which every sort ends with.
    Name,
    Date(Option<SystemTime>),
    /// A folder has no size and goes before an empty file.
    Size(u64, bool),
    Kind(String),
    Folder(Option<PathBuf>),
}

impl CutKey {
    /// Whether [`CutKey::of`] reads the disk for `hit`: for the two dates
    /// the index holds for nothing, and for when a folder last changed.
    fn reads_disk(hit: &RawHit, key: SortKey) -> bool {
        match key {
            SortKey::Created | SortKey::LastOpened => true,
            SortKey::Modified => hit.is_dir,
            SortKey::Relevance
            | SortKey::Name
            | SortKey::Size
            | SortKey::Kind
            | SortKey::Folder => false,
        }
    }

    fn of(hit: &RawHit, key: SortKey) -> CutKey {
        match key {
            SortKey::Relevance | SortKey::Name => CutKey::Name,
            // The index holds no date for a folder.
            SortKey::Modified if hit.is_dir => CutKey::Date(listing::dir_stamp(&hit.path)),
            SortKey::Modified => CutKey::Date(hit.modified_time()),
            SortKey::Size => CutKey::Size(hit.size, !hit.is_dir),
            SortKey::Kind => CutKey::Kind(kind::kind_label(&hit.path, hit.is_dir)),
            SortKey::Created => {
                let created = std::fs::metadata(&hit.path).and_then(|meta| meta.created());
                CutKey::Date(created.ok())
            }
            SortKey::LastOpened => CutKey::Date(crate::info::last_opened(&hit.path)),
            SortKey::Folder => CutKey::Folder(hit.path.parent().map(Path::to_path_buf)),
        }
    }
}

/// What is left of a content search's time limit for the root it searches
/// next, in milliseconds, the way fff-search takes it.
///
/// # Arguments
///
/// * `limit_secs` - The time limit from the settings; 0 for none
/// * `elapsed` - How long the search has run
///
/// # Returns
///
/// `Some(0)` when there is no limit, which is what 0 means to fff-search as
/// well; `None` once the limit has passed; otherwise at least 1
fn root_budget_ms(limit_secs: u64, elapsed: Duration) -> Option<u64> {
    if limit_secs == 0 {
        return Some(0);
    }
    let left = Duration::from_secs(limit_secs).saturating_sub(elapsed);
    (!left.is_zero()).then(|| u64::try_from(left.as_millis()).map_or(u64::MAX, |ms| ms.max(1)))
}

/// How fff-search's search of one root ended.
#[derive(Debug, PartialEq, Eq)]
enum PageEnd {
    /// Every file was searched.
    Complete,
    /// It had the matches it was asked for with files left to search, or it
    /// was told to stop.
    Full,
    /// The time ran out with files left to search.
    Stopped,
}

/// Tell why fff-search left files of a root unsearched. It only says that it
/// did, by naming the file the next page would start at, and it does that
/// for a full page, for the time limit and for an abort alike.
///
/// # Arguments
///
/// * `matches` - How many matches it returned
/// * `page_limit` - How many it was asked for
/// * `next_file_offset` - The file the next page starts at; 0 when every
///   file was searched
/// * `out_of_time` - Whether the root's time was up when it returned
fn page_end(
    matches: usize,
    page_limit: usize,
    next_file_offset: usize,
    out_of_time: bool,
) -> PageEnd {
    if next_file_offset == 0 {
        PageEnd::Complete
    } else if matches < page_limit && out_of_time {
        PageEnd::Stopped
    } else {
        // A full page. Or, short of one with time left, it was told to stop:
        // by the caller, or because the index is being dropped.
        PageEnd::Full
    }
}

/// Finder treats bundles (`.app`, `.framework`, …) as single items, so their
/// contents are hidden unless the search is scoped inside one.
fn inside_package(path: &Path, scope: Option<&Path>) -> bool {
    const PACKAGES: [&str; 7] = [
        ".app",
        ".framework",
        ".bundle",
        ".xcodeproj",
        ".xcworkspace",
        ".photoslibrary",
        ".plugin",
    ];
    let Some(parent) = path.parent() else {
        return false;
    };
    parent.ancestors().any(|dir| {
        if scope.is_some_and(|s| s.starts_with(dir)) {
            return false;
        }
        dir.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| PACKAGES.iter().any(|ext| n.ends_with(ext)))
    })
}

/// The apps and app folders in `entries` (from an [`AppList`]) whose names
/// hold every one of a query's fuzzy `words` in order. They're scored about
/// as fff-search scores a folder by its name: 15 points a letter for a word
/// found whole and 10 for one spread out, and none for a one-letter query,
/// which fff-search doesn't score either. [`rerank`] adds the name bonuses.
/// An app is one item wherever its folder is, as in Finder, so it's spared
/// rerank's depth penalty; otherwise `/System/Applications/Utilities/
/// Terminal.app` would lose to a file in home with the same name.
fn app_hits(entries: &[PathBuf], words: &[String]) -> Vec<RawHit> {
    let one_letter = matches!(words, [word] if word.chars().count() == 1);
    entries
        .iter()
        .filter_map(|path| {
            let name = display_name(path).to_lowercase();
            if !words.iter().all(|w| is_subsequence(w, &name)) {
                return None;
            }
            let matched: i32 = if one_letter {
                0
            } else {
                words
                    .iter()
                    .map(|w| {
                        let per_letter = if name.contains(w.as_str()) { 15 } else { 10 };
                        w.chars().count() as i32 * per_letter
                    })
                    .sum()
            };
            let depth = path.components().count() as i32;
            // Only an app's name is searched, whatever folder it is in.
            Some(RawHit {
                rel: name,
                ..RawHit::dir(path.clone(), matched + depth * DEPTH_PENALTY)
            })
        })
        .collect()
}

/// What [`case_fit`] gives a name for each letter.
const CASE_FIT: i32 = 5;

/// About what fff-search adds to a name for the words of a search made as
/// typed (see [`words_as_typed`]) that the name has in the same letter case:
/// 4 points a letter, and a sixth more when they are in the name. An app is
/// scored by [`app_hits`] as the words in lowercase score, so it is given
/// this to stay where it ranks among the names fff-search scored.
///
/// # Arguments
///
/// * `name` - An app's name
/// * `as_typed` - The search made as typed, which has only fuzzy words
fn case_fit(name: &str, as_typed: &str) -> i32 {
    as_typed
        .split_whitespace()
        .filter(|word| word.chars().nth(1).is_some() && is_subsequence(word, name))
        .map(|word| word.chars().count() as i32 * CASE_FIT)
        .sum()
}

/// Whether a search can find apps: one of every location, for folders,
/// with `words` (its fuzzy words) and nothing else.
fn wants_apps(req: &SearchRequest, words: &[String]) -> bool {
    let kind = match req.kind {
        KindFilter::Any => !req.files_only,
        KindFilter::Folder => true,
        KindFilter::Document
        | KindFilter::Image
        | KindFilter::Movie
        | KindFilter::Audio
        | KindFilter::Code
        | KindFilter::Archive => false,
    };
    // Constraints such as `*.rs` or `!test` can't be checked on an app.
    let constrained = req.query.split_whitespace().count() != words.len();
    req.scope.is_none() && kind && !constrained && !words.is_empty()
}

/// Whether fff-search matches text loosely, as it does words, that isn't one
/// of the search's words ([`fuzzy_text`]), which names are checked for: a
/// word with a line suffix, as in `main.rs:42`, which it reads as `main.rs`,
/// and a folder name typed alone, as in `src/`. Filters such as `*.pdf`
/// aren't matched loosely: they hold for an item or they don't.
///
/// # Arguments
///
/// * `fuzzy` - What fff-search matches loosely of the search
/// * `user` - The search as typed
fn loose_text(fuzzy: &FuzzyQuery, user: &str) -> bool {
    let words = fuzzy_text(user);
    let left_out = |text: &str| !words.split_whitespace().any(|word| word == text);
    match fuzzy {
        FuzzyQuery::Empty => false,
        FuzzyQuery::Text(text) => left_out(text),
        FuzzyQuery::Parts(parts) => parts.iter().any(|part| left_out(part)),
    }
}

/// Whether `path`'s name holds each of `words` (lowercase) in order.
fn name_holds(path: &Path, words: &[String]) -> bool {
    let name = display_name(path).to_lowercase();
    words.iter().all(|w| is_subsequence(w, &name))
}

/// Whether `path`'s name has each of `words` (lowercase) in it, letters in a
/// row, which is what [`rerank`] ranks first. Without words, every name does.
fn name_has(path: &Path, words: &[String]) -> bool {
    if words.is_empty() {
        return true;
    }
    let name = display_name(path).to_lowercase();
    words.iter().all(|word| name.contains(word.as_str()))
}

fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut chars = haystack.chars();
    needle.chars().all(|c| chars.any(|h| h == c))
}

/// What [`rerank`] takes off for each folder in a hit's path.
const DEPTH_PENALTY: i32 = 3;

/// Finder-style ranking on top of fff's score: words found in the file
/// name beat words found only in its folders, and shallower paths win ties.
fn rerank(hit: &RawHit, words: &[String]) -> i32 {
    if words.is_empty() {
        return hit.score;
    }
    let name = display_name(&hit.path).to_lowercase();
    let stem = name.rsplit_once('.').map_or(name.as_str(), |(s, _)| s);
    let mut bonus = 0;
    let mut in_name = 0;
    for word in words {
        if stem == word || name == *word {
            bonus += 120;
        } else if name.starts_with(word.as_str()) {
            bonus += 70;
        } else if let Some(pos) = name.find(word.as_str()) {
            let boundary = name[..pos]
                .chars()
                .last()
                .is_some_and(|c| !c.is_alphanumeric());
            bonus += if boundary { 55 } else { 40 };
        } else {
            continue;
        }
        in_name += 1;
    }
    if in_name == words.len() {
        bonus += 40;
    }
    let depth = hit.path.components().count() as i32;
    hit.score + bonus - depth * DEPTH_PENALTY
}

/// Add what the same search found when it was made another way. An item
/// found both ways keeps the higher of its two scores.
///
/// # Arguments
///
/// * `hits` - What one search of a root found
/// * `more` - What the other found there
fn keep_higher(hits: &mut Vec<RawHit>, more: Vec<RawHit>) {
    let found: HashMap<PathBuf, usize> = hits
        .iter()
        .enumerate()
        .map(|(ix, hit)| (hit.path.clone(), ix))
        .collect();
    for hit in more {
        match found.get(&hit.path) {
            Some(&ix) => hits[ix].score = hits[ix].score.max(hit.score),
            None => hits.push(hit),
        }
    }
}

/// fff returns `/`-separated relative paths on every OS.
fn join_rel(base: &Path, rel: &str) -> PathBuf {
    let rel = rel.trim_end_matches('/');
    let mut path = base.to_path_buf();
    for part in rel.split('/').filter(|p| !p.is_empty()) {
        path.push(part);
    }
    path
}

/// The fuzzy words of a query, without constraint tokens.
fn fuzzy_text(query: &str) -> String {
    query
        .split_whitespace()
        .filter(|t| {
            !t.starts_with('!')
                && !t.contains(['*', '?', '[', '{'])
                && !t.contains(':')
                && !t.starts_with('/')
                && !t.ends_with('/')
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether `/notes/` and `src/` are folder filters in a search that can have
/// folders in its results. They are beside other tokens, as in `/notes/ todo`.
/// One that is the whole search is a word there instead, so that typing
/// `src/` lists the folders that match it.
fn folder_tokens_filter(query: &str) -> bool {
    query.split_whitespace().nth(1).is_some()
}

/// How fff-search reads a search that can have folders in its results. Its
/// own settings for these searches take every folder token for a word, and
/// `/notes/ todo` then finds `meeting-notes/todo.txt` as well.
struct WithFolders {
    /// Only folders are searched. They have no extension, file type or git
    /// status to filter by.
    only: bool,
    /// What [`folder_tokens_filter`] says of the search.
    folder_filters: bool,
}

impl ParserConfig for WithFolders {
    // Off, because fff-search's own folder filters take any token with a
    // slash at an end for a folder's name, a glob such as `src*/` too.
    // `parse_custom` reads them instead, which a glob never reaches.
    fn enable_path_segments(&self) -> bool {
        false
    }

    fn enable_extension(&self) -> bool {
        !self.only
    }

    fn enable_type_filter(&self) -> bool {
        !self.only
    }

    fn enable_git_status(&self) -> bool {
        !self.only
    }

    fn parse_custom<'a>(&self, token: &'a str) -> Option<Constraint<'a>> {
        let folder = token.trim_matches('/');
        let is_filter = self.folder_filters
            && !folder.is_empty()
            && (token.starts_with('/') || token.ends_with('/'));
        is_filter.then_some(Constraint::PathSegment(folder))
    }
}

/// The tokens that keep a search to the files of one kind, to go after the
/// typed ones: the kind's extensions, each as `*.ext`. fff-search takes the
/// extensions in a search as alternatives, though, so beside a typed `*.svg`
/// they would bring back every image. A search with a typed extension gets
/// none of them. It gets an exclusion for each typed extension the kind
/// doesn't have instead, so Image finds only the SVG files of `*.svg *.txt`
/// and nothing for `*.txt`. Were those files left to the check on each hit,
/// they would use up the hits fff-search is asked for.
///
/// # Arguments
///
/// * `kind` - The kind searched for
/// * `user` - What was typed, as fff-search is given it
///
/// # Returns
///
/// The tokens, each after a space; none for Any Kind and Folder, which have
/// no extensions
fn kind_tokens(kind: KindFilter, user: &str) -> String {
    let mut tokens = String::new();
    let extensions = kind.extensions();
    if extensions.is_empty() {
        return tokens;
    }
    let parsed = QueryParser::default().parse(user);
    let typed: Vec<&str> = parsed
        .constraints
        .iter()
        .filter_map(|constraint| match constraint {
            Constraint::Extension(ext) => Some(*ext),
            _ => None,
        })
        .collect();
    if typed.is_empty() {
        for ext in extensions {
            tokens.push_str(" *.");
            tokens.push_str(ext);
        }
    }
    for ext in typed {
        // A kind goes by a file's extension alone, so one name stands for
        // every file that has this one.
        if !kind.accepts(Path::new(&format!("x.{ext}"))) {
            tokens.push_str(" !*.");
            tokens.push_str(ext);
        }
    }
    tokens
}

/// Take the quoted phrases out of a name search, `"tax exemption"` and the
/// like. A quote left open runs to the end, so typing one narrows the results
/// right away.
///
/// # Arguments
///
/// * `query` - What was typed in the search field
///
/// # Returns
///
/// The query with each phrase's words left unquoted, for fff-search, and the
/// phrases each name must contain
fn split_quoted(query: &str) -> (String, Vec<String>) {
    let mut rest = String::new();
    let mut phrases = Vec::new();
    let mut parts = query.split('"');
    rest.push_str(parts.next().unwrap_or_default());
    for (i, part) in parts.enumerate() {
        if i % 2 == 0 {
            let phrase = part.trim();
            if !phrase.is_empty() {
                phrases.push(phrase.to_string());
                // Only words fff-search reads as fuzzy text, not as a glob,
                // folder, or exclusion.
                rest.push(' ');
                rest.push_str(&fuzzy_text(phrase));
                rest.push(' ');
            }
        } else {
            rest.push_str(part);
        }
    }
    (
        rest.split_whitespace().collect::<Vec<_>>().join(" "),
        phrases,
    )
}

/// Whether letter case is ignored for `text` under `case`.
fn ignores_case(case: SearchCase, text: &str) -> bool {
    match case {
        SearchCase::Smart => !text.chars().any(char::is_uppercase),
        SearchCase::Sensitive => false,
        SearchCase::Insensitive => true,
    }
}

/// Whether `name` has `phrase` in it, letters in a row.
///
/// # Arguments
///
/// * `name` - A file or folder name
/// * `phrase` - A quoted phrase from the search
/// * `case` - The letter case setting
fn contains_phrase(name: &str, phrase: &str, case: SearchCase) -> bool {
    match ignores_case(case, phrase) {
        true => name.to_lowercase().contains(&phrase.to_lowercase()),
        false => name.contains(phrase),
    }
}

/// Make the globs in a name search follow the letter case setting.
/// fff-search matches globs letter for letter, so where case is ignored each
/// letter outside a `[...]` set becomes a set of both cases: `*Tax*` turns
/// into `*[tT][aA][xX]*`. Extensions such as `*.pdf` are left alone.
///
/// # Arguments
///
/// * `query` - The search, after quoted phrases are taken out
/// * `case` - The letter case setting
fn fold_glob_case(query: &str, case: SearchCase) -> String {
    query
        .split_whitespace()
        .map(|token| {
            let body = token.strip_prefix('!').unwrap_or(token);
            let is_glob = body.contains(['*', '?', '[', '{']);
            let is_extension = body
                .strip_prefix("*.")
                .is_some_and(|ext| !ext.is_empty() && ext.chars().all(char::is_alphanumeric));
            match is_glob && !is_extension && ignores_case(case, body) {
                true => fold_case(token),
                false => token.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Turn each letter of a glob outside `[...]` and escapes into `[xX]`.
fn fold_case(glob: &str) -> String {
    let mut out = String::with_capacity(glob.len() * 2);
    let mut in_set = false;
    let mut chars = glob.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                out.push(c);
                out.extend(chars.next());
            }
            '[' if !in_set => {
                in_set = true;
                out.push(c);
            }
            ']' if in_set => {
                in_set = false;
                out.push(c);
            }
            _ if !in_set => {
                let mut lower = c.to_lowercase();
                let mut upper = c.to_uppercase();
                match (lower.next(), lower.next(), upper.next(), upper.next()) {
                    (Some(l), None, Some(u), None) if l != u => {
                        out.push('[');
                        out.push(l);
                        out.push(u);
                        out.push(']');
                    }
                    _ => out.push(c),
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Put the words of a name search in lowercase, so that they ignore letter
/// case. fff-search matches a word that has a capital case for case:
/// `README` as typed never finds `readme.md`. Globs follow the letter case
/// setting and exclusions ignore case already, so both are left alone.
///
/// # Arguments
///
/// * `query` - The search, after quoted phrases are taken out
/// * `folder_words` - Whether fff-search reads `/notes/` and `src/` as words
///   too, as it does for one that is the whole search when the results can
///   have folders. Otherwise they are folder filters, which ignore case from
///   A to Z and match any other letter only as typed, so they are left alone
///   as well
fn fold_word_case(query: &str, folder_words: bool) -> String {
    query
        .split_whitespace()
        .map(|token| {
            let is_filter = token.starts_with('!')
                || token.contains(['*', '?', '[', '{'])
                || (!folder_words && (token.starts_with('/') || token.ends_with('/')));
            match is_filter {
                true => token.to_string(),
                false => lowercase_word(token),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `word` with its capitals in lowercase, which makes fff-search match it in
/// either case. fff-search matches a lowercase letter against its capital
/// only when both are one character of the same length in UTF-8, so a word
/// with any other capital, such as `İ`, is left as typed and still finds
/// names spelled that way.
fn lowercase_word(word: &str) -> String {
    let lowered: Option<String> = word
        .chars()
        .map(|c| {
            if !c.is_uppercase() {
                return Some(c);
            }
            let mut lower = c.to_lowercase();
            match (lower.next(), lower.next()) {
                (Some(l), None)
                    if l.is_lowercase()
                        && l.len_utf8() == c.len_utf8()
                        && l.to_uppercase().eq([c]) =>
                {
                    Some(l)
                }
                _ => None,
            }
        })
        .collect();
    lowered.unwrap_or_else(|| word.to_string())
}

/// A lowercase search with its words as typed again, which is the search
/// fff-search can rank by capitals: it gives a word that has one more for
/// each name with the same capitals, and nothing for them once the word is
/// in lowercase. A token with a colon stays in lowercase, where it may be a
/// filter (`git:modified`) that it isn't as typed. So does a lone letter:
/// fff-search ignores it, and the search would be the same one again.
///
/// # Arguments
///
/// * `lowered` - The search from [`fold_word_case`]
/// * `typed` - The search it was made from
fn words_as_typed(lowered: &str, typed: &str) -> String {
    lowered
        .split_whitespace()
        .zip(typed.split_whitespace())
        .map(|(lowered, typed)| {
            let ranked = !typed.contains(':') && typed.chars().nth(1).is_some();
            match ranked {
                true => typed,
                false => lowered,
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Greedy case-insensitive subsequence match of `needle` in `name`, for highlighting.
pub fn highlight(name: &str, needle: &str) -> Vec<(usize, usize)> {
    let needle: Vec<char> = needle
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    if needle.is_empty() {
        return Vec::new();
    }
    let lower = name.to_lowercase();
    // Prefer a contiguous substring match when there is one.
    let joined: String = needle.iter().collect();
    if lower.len() == name.len()
        && let Some(start) = lower.find(&joined)
    {
        return vec![(start, start + joined.len())];
    }
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut want = needle.iter().peekable();
    for (i, c) in name.char_indices() {
        let Some(&&w) = want.peek() else { break };
        if c.to_lowercase().eq(std::iter::once(w)) {
            let end = i + c.len_utf8();
            match ranges.last_mut() {
                Some(last) if last.1 == i => last.1 = end,
                _ => ranges.push((i, end)),
            }
            want.next();
        }
    }
    if want.peek().is_some() {
        // Typo-tolerant matches may not be subsequences; show no partial noise.
        return Vec::new();
    }
    ranges
}

fn normalize_roots(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for root in roots {
        let root = strip_trailing_separator(root);
        if !root.is_dir() {
            continue;
        }
        if out.iter().any(|r| root.starts_with(r)) {
            continue;
        }
        out.retain(|r| !r.starts_with(&root));
        out.push(root);
    }
    out
}

fn strip_trailing_separator(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    let trimmed = s.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() || (trimmed.len() == 2 && trimmed.ends_with(':')) {
        path.to_path_buf()
    } else {
        PathBuf::from(trimmed)
    }
}

fn open_dbs(dir: Option<&Path>) -> (SharedFrecency, SharedQueryTracker) {
    let frecency = SharedFrecency::default();
    let queries = SharedQueryTracker::default();
    let Some(dir) = dir else {
        return (frecency, queries);
    };
    match FrecencyTracker::open(dir.join("frecency")) {
        Ok(tracker) => {
            if let Err(err) = frecency.init(tracker) {
                log::warn!("frecency db: {err}");
            }
        }
        Err(err) => log::warn!("frecency db: {err}"),
    }
    match QueryTracker::open(dir.join("queries")) {
        Ok(tracker) => {
            if let Err(err) = queries.init(tracker) {
                log::warn!("query db: {err}");
            }
        }
        Err(err) => log::warn!("query db: {err}"),
    }
    (frecency, queries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_follows_the_base_spelling() {
        // As on Windows, where fff spells `C:\Users\RUNNER~1` in full.
        let root = RootIndex {
            path: "/short/tree".into(),
            canonical: "/long/tree".into(),
            picker: SharedFilePicker::default(),
        };
        let base = Path::new("/long/tree");
        let scope = |s: &str| root.in_base(Path::new(s), base);
        assert_eq!(scope("/short/tree"), base);
        assert_eq!(
            scope("/short/tree/Projects/atlas"),
            base.join("Projects/atlas")
        );
        assert_eq!(scope("/elsewhere"), Path::new("/elsewhere"));
    }

    #[test]
    fn highlights() {
        assert_eq!(highlight("Project Atlas.txt", "proj"), vec![(0, 4)]);
        assert_eq!(highlight("main.rs", "mrs"), vec![(0, 1), (5, 7)]);
        assert!(highlight("abc", "xyz").is_empty());
    }

    #[test]
    fn roots_are_normalized() {
        let tmp = std::env::temp_dir();
        let nested = tmp.join(format!("fff-norm-{}", std::process::id()));
        std::fs::create_dir_all(&nested).unwrap();
        let roots = normalize_roots(&[nested.clone(), tmp.clone(), PathBuf::from("/nope/x")]);
        assert_eq!(roots, vec![strip_trailing_separator(&tmp)]);
        std::fs::remove_dir_all(nested).unwrap();
    }

    #[test]
    fn subsequence() {
        assert!(is_subsequence("mrs", "main.rs"));
        assert!(!is_subsequence("engine", "crates/nu_plugin_fff/cargo.toml"));
    }

    #[test]
    fn glob_finds_files_inside_bundles() {
        let dir = std::env::temp_dir().join(format!("fff-glob-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for app in ["One.app", "nested/Two.app"] {
            std::fs::create_dir_all(dir.join(app).join("Contents")).unwrap();
            std::fs::write(dir.join(app).join("Contents/Info.plist"), "<plist/>").unwrap();
        }
        std::fs::write(dir.join("Info.plist"), "<plist/>").unwrap();
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(20)));
        let found = engine.glob("**/*.app/Contents/Info.plist", 100);
        // fff spells the root its own way on Windows (long names, `\`), so
        // compare the files rather than the spelling.
        let real = |p: &Path| std::fs::canonicalize(p).unwrap();
        assert_eq!(
            found.iter().map(|p| real(p)).collect::<Vec<_>>(),
            vec![
                real(&dir.join("One.app/Contents/Info.plist")),
                real(&dir.join("nested/Two.app/Contents/Info.plist")),
            ]
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn package_contents_are_hidden() {
        let app = Path::new("/Applications/Xcode.app/Contents/README");
        assert!(inside_package(app, None));
        assert!(!inside_package(Path::new("/Applications/Xcode.app"), None));
        assert!(!inside_package(
            app,
            Some(Path::new("/Applications/Xcode.app"))
        ));
    }

    #[test]
    fn rerank_prefers_name_matches() {
        let words = vec!["project".to_string(), "atlas".to_string()];
        let deep = RawHit::file("/a/b/c/d/e/projected.h".into(), 1, 1, None, 78);
        let good = RawHit::file("/a/Project Atlas.txt".into(), 1, 1, None, 70);
        assert!(rerank(&good, &words) > rerank(&deep, &words));
    }

    #[test]
    fn a_result_found_both_ways_keeps_its_higher_score() {
        let hit = |path: &str, score| RawHit::file(path.into(), 1, 1, None, score);
        let mut hits = vec![hit("/a/README.md", 70), hit("/b/readme.md", 70)];
        let as_typed = vec![
            hit("/a/README.md", 94),
            hit("/b/readme.md", 12),
            hit("/c/Read me.md", 30),
        ];
        keep_higher(&mut hits, as_typed);
        let scores: Vec<(String, i32)> = hits
            .iter()
            .map(|h| (h.path.to_string_lossy().into_owned(), h.score))
            .collect();
        assert_eq!(
            scores,
            [
                ("/a/README.md".to_string(), 94),
                ("/b/readme.md".to_string(), 70),
                ("/c/Read me.md".to_string(), 30),
            ]
        );
    }

    #[test]
    fn an_app_scores_for_the_capitals_it_fits() {
        assert_eq!(case_fit("Safari.app", "Safari"), 6 * CASE_FIT);
        assert_eq!(case_fit("Safari.app", "SAFARI"), 0);
        assert_eq!(
            case_fit("TextEdit.app", "TE"),
            2 * CASE_FIT,
            "letters apart fit too"
        );
        assert_eq!(
            case_fit("Tax tables.app", "t Tax tables"),
            9 * CASE_FIT,
            "fff-search ignores a lone letter"
        );
    }

    #[test]
    fn fuzzy_text_drops_constraints() {
        assert_eq!(fuzzy_text("main *.rs /src/ !test git:m"), "main");
    }

    #[test]
    fn adds_up_folders_from_the_index() {
        let dir = std::env::temp_dir().join(format!("fff-engine-stats-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("a/deep")).unwrap();
        std::fs::create_dir_all(dir.join("b")).unwrap();
        std::fs::write(dir.join("a/one.txt"), "12345").unwrap();
        std::fs::write(dir.join("a/deep/two.txt"), "123").unwrap();
        std::fs::write(dir.join("b/three.txt"), "1").unwrap();
        std::fs::write(dir.join("top.txt"), "xx").unwrap();
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let stats = engine.folder_stats(&dir);
        assert_eq!(stats.sizes.get(&dir.join("a")), Some(&8));
        assert_eq!(stats.sizes.get(&dir.join("b")), Some(&1));
        assert_eq!(stats.sizes.len(), 2, "files at the top aren't folders");
        let above = engine.folder_stats(dir.parent().unwrap());
        assert_eq!(
            above.sizes.get(&dir),
            Some(&11),
            "a root inside counts whole"
        );
        let both = engine.folder_stats_for(&[dir.join("a"), dir.clone()]);
        assert_eq!(both[0].sizes.get(&dir.join("a/deep")), Some(&3));
        assert_eq!(
            both[1], stats,
            "one pass gives what one folder at a time does"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_root_inside_another_counts_once() {
        let dir = std::env::temp_dir().join(format!("fff-engine-nested-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("inner/deep")).unwrap();
        std::fs::write(dir.join("inner/deep/one.txt"), "12345").unwrap();
        std::fs::write(dir.join("inner/two.txt"), "123").unwrap();
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone(), dir.join("inner")],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        assert_eq!(
            engine.folder_stats(&dir).sizes.get(&dir.join("inner")),
            Some(&8)
        );
        assert_eq!(
            engine
                .folder_stats(&dir.join("inner"))
                .sizes
                .get(&dir.join("inner/deep")),
            Some(&5)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn quoted_phrases_come_out_of_the_query() {
        assert_eq!(
            split_quoted(r#""tax exemption" *.sql"#),
            ("tax exemption *.sql".into(), vec!["tax exemption".into()])
        );
        assert_eq!(
            split_quoted(r#"report "Q3"#),
            ("report Q3".into(), vec!["Q3".into()]),
            "an open quote runs to the end"
        );
        assert_eq!(
            split_quoted(r#"a "" b"#),
            ("a b".into(), Vec::<String>::new())
        );
        assert_eq!(
            split_quoted("plain"),
            ("plain".into(), Vec::<String>::new())
        );
    }

    #[test]
    fn globs_follow_the_letter_case_setting() {
        use SearchCase::*;
        assert_eq!(
            fold_glob_case("*tax* budget", Insensitive),
            "*[tT][aA][xX]* budget"
        );
        assert_eq!(
            fold_glob_case("*Tax*", Smart),
            "*Tax*",
            "a capital asks for case"
        );
        assert_eq!(fold_glob_case("*tax*", Smart), "*[tT][aA][xX]*");
        assert_eq!(fold_glob_case("*tax*", Sensitive), "*tax*");
        assert_eq!(fold_glob_case("*.pdf !*.log", Insensitive), "*.pdf !*.log");
        assert_eq!(
            fold_glob_case(r"!src/[ab]x\*", Insensitive),
            r"![sS][rR][cC]/[ab][xX]\*"
        );
    }

    #[test]
    fn words_are_searched_in_lowercase() {
        assert_eq!(
            fold_word_case("Main *.RS !Test /Src/ Docs/ Lib.rs:42 Git:Modified", true),
            "main *.RS !Test /src/ docs/ lib.rs:42 git:modified"
        );
        assert_eq!(
            fold_word_case("Main /Src/ Docs/", false),
            "main /Src/ Docs/",
            "folder filters ignore case as typed"
        );
        assert_eq!(
            fold_word_case(
                &fold_glob_case("*Tax* Budget", SearchCase::Insensitive),
                true
            ),
            "*[tT][aA][xX]* budget"
        );
        assert_eq!(fold_word_case("ÜBER Москва", true), "über москва");
        // Capitals fff-search can't match from lowercase: "İ" becomes two
        // characters and "Ⱥ" a longer one, "ϴ" becomes the lowercase of "Θ",
        // and "ℂ" has none.
        for word in ["İSTANBUL", "ȺBC", "ϴETA", "ℂOMPLEX"] {
            assert_eq!(fold_word_case(word, true), word);
        }
        assert_eq!(fold_word_case("İSTANBUL Notes", true), "İSTANBUL notes");
    }

    #[test]
    fn words_are_searched_as_typed_too() {
        let typed = "Main T *.RS !Test /Src/ Docs/ Lib.rs:42 Git:Modified";
        assert_eq!(
            words_as_typed(&fold_word_case(typed, true), typed),
            "Main t *.RS !Test /Src/ Docs/ lib.rs:42 git:modified",
            "but not a lone letter, or what may be a filter in lowercase"
        );
        assert_eq!(
            words_as_typed(&fold_word_case(typed, false), typed),
            "Main t *.RS !Test /Src/ Docs/ lib.rs:42 git:modified"
        );
        assert_eq!(words_as_typed("main tax", "main tax"), "main tax");
    }

    #[test]
    fn quoted_words_match_names_letter_for_letter() {
        let dir = std::env::temp_dir().join(format!("fff-exact-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("token/exemption/coding")).unwrap();
        std::fs::write(dir.join("TaxExemptionCertificate.object"), "").unwrap();
        std::fs::write(dir.join("TAX_EXEMPTION_CODE.sql"), "").unwrap();
        // Every letter of "taxexemptionc", in order, across the path.
        std::fs::write(dir.join("token/exemption/coding/TaxHandler.java"), "").unwrap();
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let names = |query: &str| {
            let mut names: Vec<String> = engine
                .search(&SearchRequest {
                    query: query.into(),
                    files_only: true,
                    limit: 50,
                    ..Default::default()
                })
                .hits
                .into_iter()
                .map(|h| h.entry.name)
                .collect();
            names.sort();
            names
        };
        let set_case = |case| {
            let mut settings = engine.search_settings();
            settings.case = case;
            engine.set_search_settings(settings);
        };

        assert!(names("taxexemptionc").len() > 1, "fuzzy finds the gaps too");
        assert_eq!(
            names(r#""taxexemptionc""#),
            ["TaxExemptionCertificate.object"]
        );
        assert_eq!(
            names(r#""tax_exemption" *.sql"#),
            ["TAX_EXEMPTION_CODE.sql"]
        );
        assert_eq!(
            names(r#""TaxExemption""#),
            ["TaxExemptionCertificate.object"]
        );
        set_case(SearchCase::Sensitive);
        assert_eq!(names(r#""taxexemption""#), Vec::<String>::new());
        assert_eq!(names("*TaxEx*"), ["TaxExemptionCertificate.object"]);
        assert_eq!(names("*taxex*"), Vec::<String>::new());
        set_case(SearchCase::Insensitive);
        assert_eq!(names("*taxex*"), ["TaxExemptionCertificate.object"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn folder_filters_narrow_searches_that_can_list_folders() {
        let dir = std::env::temp_dir().join(format!("fff-folder-filter-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // Only the first two are inside a folder named "notes".
        for file in [
            "notes/todo.txt",
            "notes/todo-items/first.md",
            "meeting-notes/todo-agenda.txt",
            "other/notes-todo.txt",
            "other/todo-archive/second.md",
            "Москва/todo-moscow.txt",
        ] {
            let path = dir.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "").unwrap();
        }
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        // The window's search, which leaves `files_only` off.
        let hits = |query: &str, kind: KindFilter, scope: Option<&str>| {
            let request = SearchRequest {
                query: query.into(),
                scope: scope.map(|folder| dir.join(folder)),
                kind,
                limit: 50,
                ..Default::default()
            };
            engine.search(&request).hits
        };
        let find = |query: &str, kind: KindFilter| {
            let mut names: Vec<String> = hits(query, kind, None)
                .into_iter()
                .map(|h| h.entry.name)
                .collect();
            names.sort();
            names
        };
        let in_notes = ["first.md", "todo-items", "todo.txt"];

        assert_eq!(find("/notes/ todo", KindFilter::Any), in_notes);
        assert_eq!(find("/NOTES/ todo", KindFilter::Any), in_notes);
        assert_eq!(
            find("todo /notes", KindFilter::Any),
            in_notes,
            "a slash at either end makes it one"
        );
        assert_eq!(
            find("todo notes/", KindFilter::Any),
            in_notes,
            "written at the end, it doesn't ask for folders only"
        );
        assert_eq!(
            find("todo notes/ !other/", KindFilter::Any),
            in_notes,
            "nor do two of them"
        );
        assert_eq!(
            find("todo !notes/", KindFilter::Any),
            [
                "notes-todo.txt",
                "second.md",
                "todo-agenda.txt",
                "todo-archive",
                "todo-moscow.txt"
            ],
            "only the folder named notes is left out"
        );
        assert_eq!(
            find("/notes/ *.md *.txt", KindFilter::Any),
            ["first.md", "todo.txt"],
            "extensions are still alternatives beside one"
        );
        assert_eq!(find("todo notes/", KindFilter::Folder), ["todo-items"]);
        assert_eq!(
            find("/notes/ todo", KindFilter::Document),
            ["first.md", "todo.txt"]
        );
        assert_eq!(
            find("first /notes/todo-items/", KindFilter::Any),
            ["first.md"],
            "folders in a row"
        );
        // The folder a filter names counts as inside it: it is listed with
        // what it holds, and left out with it.
        assert_eq!(
            find("items todo-items/", KindFilter::Any),
            ["first.md", "todo-items"]
        );
        assert_eq!(
            find("todo !todo-items/", KindFilter::Folder),
            ["todo-archive"]
        );
        // Beyond A to Z, a filter's letters are compared as they are typed.
        assert_eq!(find("todo /Москва/", KindFilter::Any), ["todo-moscow.txt"]);

        // A folder token that is the whole search is a word, and lists the
        // folders that match it.
        let alone = hits("notes/", KindFilter::Any, None);
        assert!(alone.iter().all(|h| h.entry.is_dir));
        assert!(alone.iter().any(|h| h.entry.name == "notes"));
        assert!(alone.iter().any(|h| h.entry.name == "meeting-notes"));
        // It is the whole search when one folder is searched as well, though
        // fff-search is then given that folder beside it.
        let scoped = hits("archive/", KindFilter::Any, Some("other"));
        assert!(scoped.iter().any(|h| h.entry.name == "todo-archive"));
        // A glob stays a glob with a slash at its end, where it matches
        // folders.
        assert_eq!(find("archive oth*/", KindFilter::Folder), ["todo-archive"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn typos_can_be_turned_off() {
        let dir = std::env::temp_dir().join(format!("fff-typos-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("budget.txt"), "").unwrap();
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let names = |query: &str| -> Vec<String> {
            engine
                .search(&SearchRequest {
                    query: query.into(),
                    files_only: true,
                    limit: 50,
                    ..Default::default()
                })
                .hits
                .into_iter()
                .map(|h| h.entry.name)
                .collect()
        };
        // "bduget" isn't "budget"'s letters in order: only a typo match.
        assert_eq!(names("bduget"), ["budget.txt"], "typos are allowed");
        let mut settings = engine.search_settings();
        settings.typos = false;
        engine.set_search_settings(settings);
        assert_eq!(names("bduget"), Vec::<String>::new());
        assert_eq!(names("bdgt"), ["budget.txt"], "gaps still match");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn letters_in_the_search_locations_own_path_dont_count() {
        // The folder that is searched has "budget" in its own name, which
        // fff-search never sees: it matches the path inside the folder.
        let dir = std::env::temp_dir().join(format!("budget-box-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("invoices")).unwrap();
        for file in ["budget.txt", "bduget.txt", "invoices/inovice.pdf"] {
            std::fs::write(dir.join(file), "").unwrap();
        }
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let find = |query: &str, kind: KindFilter, files_only: bool, scope: Option<&Path>| {
            let request = SearchRequest {
                query: query.into(),
                scope: scope.map(Path::to_path_buf),
                kind,
                files_only,
                limit: 50,
                ..Default::default()
            };
            let hits = engine.search(&request).hits;
            hits.into_iter().map(|h| h.entry.name).collect::<Vec<_>>()
        };
        let names = |query: &str| find(query, KindFilter::Any, true, None);

        assert_eq!(names("budget"), ["budget.txt"], "the near miss is dropped");
        assert_eq!(names("bduget"), ["bduget.txt"]);
        assert_eq!(
            find("budget", KindFilter::Any, true, Some(&dir)),
            ["budget.txt"],
            "and the same when only that folder is searched"
        );

        let mut settings = engine.search_settings();
        settings.typos = false;
        engine.set_search_settings(settings);
        // The b, u and g are in the folder's name, and the d, e and t in each
        // file's.
        assert_eq!(names("bugdet"), Vec::<String>::new());
        assert_eq!(names("budget"), ["budget.txt"]);
        // A folder inside the search location does count, in each kind of
        // search.
        assert_eq!(names("invoice"), ["inovice.pdf"]);
        assert_eq!(
            find("invoice", KindFilter::Any, false, None),
            ["invoices", "inovice.pdf"]
        );
        assert_eq!(
            find("invoice", KindFilter::Folder, false, None),
            ["invoices"]
        );
        // Searched itself, that folder is the one whose name doesn't count.
        let invoices = dir.join("invoices");
        assert_eq!(
            find("invoice", KindFilter::Any, true, Some(&invoices)),
            Vec::<String>::new()
        );
        assert_eq!(
            find("inovice", KindFilter::Any, true, Some(&invoices)),
            ["inovice.pdf"]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn capitals_in_a_word_dont_narrow_the_search() {
        let dir = std::env::temp_dir().join(format!("fff-caps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // Each spelling in its own folder, for disks that ignore case.
        for file in [
            "a/README.md",
            "b/Readme.md",
            "c/readme.md",
            "pics/img_0001.jpg",
            "Notes/todo.txt",
        ] {
            let path = dir.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "").unwrap();
        }
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let find = |query: &str, kind: KindFilter, files_only: bool| -> Vec<String> {
            engine
                .search(&SearchRequest {
                    query: query.into(),
                    kind,
                    files_only,
                    limit: 50,
                    ..Default::default()
                })
                .hits
                .into_iter()
                .map(|h| h.entry.name)
                .collect()
        };
        let names = |query: &str| find(query, KindFilter::Any, true);
        let sorted = |mut names: Vec<String>| {
            names.sort();
            names
        };
        let set_case = |case| {
            let mut settings = engine.search_settings();
            settings.case = case;
            engine.set_search_settings(settings);
        };
        let listed = |dirs: bool, query: &str| {
            let items = engine.list(dirs, Some(query), 0);
            sorted(items.iter().map(|i| display_name(&i.path)).collect())
        };
        let every_spelling = ["README.md", "Readme.md", "readme.md"];

        assert_eq!(sorted(names("README")), every_spelling);
        assert_eq!(names("README")[0], "README.md", "as typed comes first");
        assert_eq!(sorted(names("Readme")), every_spelling);
        assert_eq!(names("Readme")[0], "Readme.md", "as typed comes first");
        assert_eq!(names("IMG_0001"), ["img_0001.jpg"]);
        assert_eq!(names("TODO"), ["todo.txt"]);
        assert_eq!(
            find("NOTES", KindFilter::Any, false).first(),
            Some(&"Notes".to_string())
        );
        // With folders in the results, a folder token is a word to fff-search.
        assert_eq!(find("NOTES/", KindFilter::Folder, false), ["Notes"]);
        assert_eq!(listed(false, "README"), every_spelling);
        assert_eq!(listed(true, "NOTES"), ["Notes"]);

        // Quoting a word is how to ask for its capitals.
        assert_eq!(names(r#""README""#), ["README.md"]);
        set_case(SearchCase::Insensitive);
        assert_eq!(sorted(names(r#""README""#)), every_spelling);
        set_case(SearchCase::Sensitive);
        assert_eq!(names(r#""Readme""#), ["Readme.md"]);
        assert_eq!(names(r#""readme""#), ["readme.md"]);
        assert_eq!(
            sorted(names("README")),
            every_spelling,
            "the setting isn't for words"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn capitals_say_which_names_come_first() {
        let dir = std::env::temp_dir().join(format!("fff-humps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for file in ["a/UserSettingsController.java", "b/user_service.rb"] {
            let path = dir.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "").unwrap();
        }
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let names = |query: &str| -> Vec<String> {
            engine
                .search(&SearchRequest {
                    query: query.into(),
                    files_only: true,
                    limit: 50,
                    ..Default::default()
                })
                .hits
                .into_iter()
                .map(|h| h.entry.name)
                .collect()
        };

        // Both names have a u, an s and a c in that order, and fff-search
        // likes the second more until it is told which letters are capitals.
        assert_eq!(
            names("USC"),
            ["UserSettingsController.java", "user_service.rb"]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // What a search opened is filed under the root as it was given, and looked
    // up under the root as fff-search spells it. For a temporary folder on
    // Windows the two differ (see `scope_follows_the_base_spelling`).
    #[cfg(not(windows))]
    #[test]
    fn a_search_remembers_what_it_opened_whatever_the_capitals() {
        let dir = std::env::temp_dir().join(format!("fff-opened-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let tree = dir.join("tree");
        for file in ["a/README.md", "b/readme.md"] {
            let path = tree.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "").unwrap();
        }
        let engine = Engine::new(EngineOptions {
            roots: vec![tree.clone()],
            app_dirs: Vec::new(),
            db_dir: Some(dir.join("db")),
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let first = |query: &str| {
            let request = SearchRequest {
                query: query.into(),
                limit: 50,
                ..Default::default()
            };
            engine.search(&request).hits.swap_remove(0).entry.name
        };

        assert_eq!(first("README"), "README.md");
        engine.track_open(&tree.join("b/readme.md"), Some("README"));
        assert_eq!(first("README"), "readme.md", "opened from this search");
        assert_eq!(first("Readme"), "readme.md", "in any letter case");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Left out on Windows for the reason the test above is.
    #[cfg(not(windows))]
    #[test]
    fn a_search_with_a_folder_filter_remembers_what_it_opened() {
        let dir = std::env::temp_dir().join(format!("fff-opened-in-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let tree = dir.join("tree");
        for file in ["Docs/a/README.md", "Docs/b/readme.md"] {
            let path = tree.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "").unwrap();
        }
        let engine = Engine::new(EngineOptions {
            roots: vec![tree.clone()],
            app_dirs: Vec::new(),
            db_dir: Some(dir.join("db")),
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let first = |query: &str| {
            let request = SearchRequest {
                query: query.into(),
                limit: 50,
                ..Default::default()
            };
            engine.search(&request).hits.swap_remove(0).entry.name
        };

        // The filter is searched as typed, so the search is remembered with
        // it as typed.
        assert_eq!(first("/Docs/ README"), "README.md");
        engine.track_open(&tree.join("Docs/b/readme.md"), Some("/Docs/ README"));
        assert_eq!(first("/Docs/ README"), "readme.md");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn searches_a_folder() {
        let dir = std::env::temp_dir().join(format!("fff-engine-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("docs/notes")).unwrap();
        std::fs::write(dir.join("docs/notes/Project Atlas.txt"), "atlas").unwrap();
        std::fs::write(dir.join("docs/readme.md"), "hello").unwrap();
        std::fs::write(dir.join("photo.png"), [0u8; 4]).unwrap();
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        assert!(!engine.is_scanning());
        assert_eq!(engine.roots_indexing(), 0);

        let res = engine.search(&SearchRequest {
            query: "atlas".into(),
            limit: 10,
            ..Default::default()
        });
        assert_eq!(res.hits[0].entry.name, "Project Atlas.txt");

        let res = engine.search(&SearchRequest {
            query: "notes".into(),
            kind: KindFilter::Folder,
            limit: 10,
            ..Default::default()
        });
        assert!(
            res.hits
                .iter()
                .any(|h| h.entry.is_dir && h.entry.name == "notes")
        );

        let res = engine.search(&SearchRequest {
            query: String::new(),
            kind: KindFilter::Image,
            limit: 10,
            ..Default::default()
        });
        assert_eq!(res.hits.len(), 1);
        assert_eq!(res.hits[0].entry.name, "photo.png");

        let res = engine.search(&SearchRequest {
            query: "readme".into(),
            scope: Some(dir.join("docs")),
            limit: 10,
            ..Default::default()
        });
        assert_eq!(res.hits.len(), 1);

        let res = engine.grep(&GrepRequest {
            patterns: vec!["hello".into()],
            limit: 10,
            ..Default::default()
        });
        assert_eq!(res.hits.len(), 1);
        assert_eq!(res.hits[0].line, 1);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_kind_goes_by_extension_and_a_typed_extension_narrows_it() {
        let dir = std::env::temp_dir().join(format!("fff-kinds-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("brand")).unwrap();
        for file in [
            "deck.pptx",
            "page.htm",
            "notes.log",
            "icon.svg",
            "mark.JPG",
            "brand/logo.svg",
            "brand/logo.png",
            "brand/logo.txt",
        ] {
            std::fs::write(dir.join(file), "").unwrap();
        }
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let find_in = |query: &str, kind: KindFilter, scope: Option<&str>| {
            let request = SearchRequest {
                query: query.into(),
                scope: scope.map(|folder| dir.join(folder)),
                kind,
                limit: 50,
                ..Default::default()
            };
            let hits = engine.search(&request).hits;
            let mut names: Vec<String> = hits.into_iter().map(|h| h.entry.name).collect();
            names.sort();
            names
        };
        let find = |query: &str, kind: KindFilter| find_in(query, kind, None);

        // A kind is its list of extensions, whatever `kind::classify` makes
        // of a file: it has no kind of its own for a presentation.
        assert_eq!(find("deck", KindFilter::Document), ["deck.pptx"]);
        assert_eq!(find("page", KindFilter::Code), ["page.htm"]);
        assert_eq!(find("notes", KindFilter::Document), ["notes.log"]);
        assert_eq!(find("mark", KindFilter::Image), ["mark.JPG"]);

        // fff-search takes extensions as alternatives, and a typed one isn't
        // an alternative to the kind's.
        assert_eq!(find("logo", KindFilter::Image), ["logo.png", "logo.svg"]);
        assert_eq!(find("logo *.svg", KindFilter::Image), ["logo.svg"]);
        assert_eq!(find("logo *.SVG", KindFilter::Image), ["logo.svg"]);
        assert_eq!(find("*.svg", KindFilter::Image), ["icon.svg", "logo.svg"]);
        assert_eq!(
            find_in("*.svg", KindFilter::Image, Some("brand")),
            ["logo.svg"]
        );
        assert_eq!(find("logo *.txt", KindFilter::Image), Vec::<String>::new());
        assert_eq!(find("logo *.svg *.txt", KindFilter::Image), ["logo.svg"]);
        assert_eq!(find("logo *.txt", KindFilter::Document), ["logo.txt"]);
        // An exclusion is no extension to keep to, so the kind's still apply.
        assert_eq!(find("logo !*.png", KindFilter::Image), ["logo.svg"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_typed_extension_takes_the_place_of_the_kinds_own() {
        let image = |typed: &str| kind_tokens(KindFilter::Image, typed);
        let all = image("logo");
        assert!(all.starts_with(" *.png *.jpg "), "{all}");
        assert_eq!(image("logo *.svg"), "");
        assert_eq!(image("*.SVG"), "");
        // One the kind doesn't have is excluded, or fff-search would return
        // its files in place of the kind's.
        assert_eq!(image("logo *.svg *.txt"), " !*.txt");
        assert_eq!(image("logo *.txt"), " !*.txt");
        assert_eq!(image("*.tar.gz"), " !*.tar.gz");
        assert_eq!(kind_tokens(KindFilter::Archive, "*.tar.gz"), "");
        // No extensions to keep to: an exclusion, an escaped token and globs.
        for typed in [
            "",
            "logo !*.png",
            r"logo \*.svg",
            "*.test.*",
            "art/**/*.svg",
            "*.{svg,txt}",
        ] {
            assert_eq!(image(typed), all, "{typed}");
        }
        assert_eq!(kind_tokens(KindFilter::Any, "logo *.svg"), "");
        assert_eq!(kind_tokens(KindFilter::Folder, "logo *.txt"), "");
    }

    #[test]
    fn another_typed_extension_doesnt_crowd_out_the_kinds_files() {
        let dir = std::env::temp_dir().join(format!("fff-kinds-many-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // More text files than the 2,000 hits fff-search is asked for, each
        // a better match for "logo" than the one image.
        for n in 0..2100 {
            std::fs::write(dir.join(format!("logo-{n:04}.txt")), "").unwrap();
        }
        std::fs::write(dir.join("lzzzozzzgzzzo.svg"), "").unwrap();
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(20)));
        let found = engine.search(&SearchRequest {
            query: "logo *.txt *.svg".into(),
            kind: KindFilter::Image,
            limit: 50,
            ..Default::default()
        });
        let names: Vec<String> = found.hits.into_iter().map(|h| h.entry.name).collect();
        assert_eq!(names, ["lzzzozzzgzzzo.svg"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn finds_apps_without_indexing_them() {
        let dir = std::env::temp_dir().join(format!("fff-apps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let home = dir.join("Users/me");
        let apps = dir.join("Applications");
        for app in ["Safari.app/Contents", "Notes.app", "Utilities/Terminal.app"] {
            std::fs::create_dir_all(apps.join(app)).unwrap();
        }
        std::fs::write(apps.join("Safari.app/Contents/Info.plist"), "plist").unwrap();
        std::fs::create_dir_all(home.join("Documents")).unwrap();
        for file in ["terminal-notes.txt", "Documents/safari-bookmarks.html"] {
            std::fs::write(home.join(file), "x").unwrap();
        }
        let engine = Engine::new(EngineOptions {
            roots: vec![home.clone()],
            app_dirs: vec![apps.clone()],
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let find = |query: &str, kind: KindFilter, files_only: bool| -> Vec<String> {
            let req = SearchRequest {
                query: query.into(),
                kind,
                files_only,
                limit: 20,
                ..Default::default()
            };
            engine
                .search(&req)
                .hits
                .into_iter()
                .map(|h| h.entry.name)
                .collect()
        };
        let at = |names: &[String], name: &str| names.iter().position(|n| n == name);

        let safari = find("safari", KindFilter::Any, false);
        assert_eq!(safari[0], "Safari.app");
        assert!(at(&safari, "safari-bookmarks.html").is_some());
        assert!(at(&safari, "Info.plist").is_none());

        let term = find("term", KindFilter::Any, false);
        assert!(at(&term, "Terminal.app") < at(&term, "terminal-notes.txt"));
        assert!(at(&term, "terminal-notes.txt").is_some());

        assert_eq!(find("notes", KindFilter::Folder, false)[0], "Notes.app");
        assert_eq!(find("utilities", KindFilter::Folder, false), ["Utilities"]);
        assert!(at(&find("notes", KindFilter::Any, true), "Notes.app").is_none());
        assert_eq!(
            find("notes", KindFilter::Document, false),
            ["terminal-notes.txt"]
        );
        assert!(at(&find("notes *.txt", KindFilter::Any, false), "Notes.app").is_none());
        let scoped = engine.search(&SearchRequest {
            query: "safari".into(),
            scope: Some(home.clone()),
            limit: 20,
            ..Default::default()
        });
        assert!(scoped.hits.iter().all(|h| h.entry.name != "Safari.app"));

        let sizes = engine.bundle_sizes(&apps);
        assert_eq!(sizes.get(&apps.join("Safari.app")), Some(&5));
        assert_eq!(sizes.get(&apps.join("Notes.app")), Some(&0));
        assert!(!sizes.contains_key(&apps.join("Utilities")));
        assert!(engine.bundle_sizes(&home).is_empty());

        let listed = engine.list(true, Some("safari"), 0);
        let app = listed.iter().find(|i| i.path == apps.join("Safari.app"));
        assert_eq!(app.map(|i| i.root.as_path()), Some(apps.as_path()));

        // A new app is found on the next search; folder times can be coarse.
        std::thread::sleep(Duration::from_millis(20));
        std::fs::create_dir_all(apps.join("Utilities/Console.app")).unwrap();
        assert_eq!(find("console", KindFilter::Any, false), ["Console.app"]);

        engine.set_app_dirs(&[]);
        assert!(at(&find("safari", KindFilter::Any, false), "Safari.app").is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_capital_keeps_an_app_ahead_of_other_items_with_its_name() {
        let dir = std::env::temp_dir().join(format!("fff-app-caps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let home = dir.join("Users/me");
        let apps = dir.join("Applications");
        std::fs::create_dir_all(apps.join("Safari.app")).unwrap();
        std::fs::create_dir_all(home.join("Web/Safari")).unwrap();
        for file in ["Safari.txt", "Web/Safari/History.db"] {
            std::fs::write(home.join(file), "x").unwrap();
        }
        let engine = Engine::new(EngineOptions {
            roots: vec![home.clone()],
            app_dirs: vec![apps.clone()],
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let names = |query: &str| -> Vec<String> {
            let request = SearchRequest {
                query: query.into(),
                limit: 20,
                ..Default::default()
            };
            let hits = engine.search(&request).hits;
            hits.into_iter().map(|h| h.entry.name).collect()
        };

        // fff-search gives the file and the folder more for the capital, and
        // it never sees the app.
        for query in ["safari", "Safari"] {
            let found = names(query);
            assert_eq!(found[0], "Safari.app", "{query}: {found:?}");
            assert!(found.contains(&"Safari.txt".to_string()), "{found:?}");
            assert!(found.contains(&"Safari".to_string()), "{found:?}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn one_letter_app_hits_dont_flood() {
        let entries: Vec<PathBuf> = ["/Applications/Safari.app", "/System/Applications/Notes.app"]
            .into_iter()
            .map(PathBuf::from)
            .collect();
        let words = vec!["s".to_string()];
        // fff-search scores one-letter matches 0, as `search_root` asks it.
        let mut hits = app_hits(&entries, &words);
        for file in ["/Users/me/s.txt", "/Users/me/Documents/sketch.txt"] {
            hits.push(RawHit::file(file.into(), 1, 0, None, 0));
        }
        for hit in &mut hits {
            hit.score = rerank(hit, &words);
        }
        hits.sort_by_key(|h| std::cmp::Reverse(h.score));
        let names: Vec<String> = hits.iter().map(|h| display_name(&h.path)).collect();
        assert_eq!(names, ["s.txt", "Safari.app", "sketch.txt", "Notes.app"]);
    }

    #[test]
    fn a_hit_that_isnt_shown_takes_no_place_under_the_limit() {
        let hits = || -> Vec<RawHit> {
            [".env", ".envrc", "env.txt", "environment.md"]
                .into_iter()
                .map(|name| RawHit::file(Path::new("/r").join(name), 1, 0, None, 0))
                .collect()
        };
        let names = |hits: &[SearchHit]| -> Vec<String> {
            hits.iter().map(|hit| hit.entry.name.clone()).collect()
        };

        // With hidden files off, the two that rank first aren't shown, and
        // the two after them are the two the limit has room for.
        let (found, truncated) = cut_to_limit(hits(), 2, "env", false);
        assert_eq!(names(&found), ["env.txt", "environment.md"]);
        assert!(!truncated, "nothing that can be shown was left out");
        let (found, truncated) = cut_to_limit(hits(), 1, "env", false);
        assert_eq!(names(&found), ["env.txt"]);
        assert!(truncated);

        // As many hits as the limit is all of them.
        let (found, truncated) = cut_to_limit(hits(), 4, "env", true);
        assert_eq!(found.len(), 4);
        assert!(!truncated);
        let (found, truncated) = cut_to_limit(hits(), 3, "env", true);
        assert_eq!(names(&found), [".env", ".envrc", "env.txt"]);
        assert!(truncated);
    }

    #[test]
    fn a_sort_before_the_cut_stays_with_the_names_that_have_the_words() {
        // Best first. The last one has the word only in its folder's name.
        let hits = || -> Vec<RawHit> {
            [
                ("/r/tax.txt", 5),
                ("/r/my-tax-notes.txt", 300),
                ("/r/xtaxx.txt", 20),
                ("/r/tax/huge.bin", 9000),
            ]
            .into_iter()
            .map(|(path, size)| RawHit::file(path.into(), size, 0, None, 0))
            .collect()
        };
        let names = |hits: &[RawHit]| -> Vec<String> {
            hits.iter().map(|hit| display_name(&hit.path)).collect()
        };
        let largest = SortOrder {
            key: SortKey::Size,
            ascending: false,
        };
        let words = vec!["tax".to_string()];
        // The hits a sort chooses among, or `None` when it leaves the list
        // to the search.
        let sorted = |limit: usize, words: &[String]| {
            let mut raw = hits();
            if sort_before_cut(&mut raw, largest, limit, words) {
                return Some(names(&raw));
            }
            assert_eq!(names(&raw), names(&hits()), "left as they were");
            None
        };

        // Three names have the word and two fit, so the largest are chosen
        // among the three. The file in the folder isn't one of them.
        assert_eq!(
            sorted(2, &words).unwrap(),
            ["my-tax-notes.txt", "xtaxx.txt", "tax.txt"]
        );
        // All three fit, and the list has room for another match.
        assert_eq!(sorted(3, &words), None);

        // Without words every hit matches as well as the next.
        assert_eq!(
            sorted(2, &[]).unwrap(),
            ["huge.bin", "my-tax-notes.txt", "xtaxx.txt", "tax.txt"]
        );
    }

    #[test]
    fn a_list_with_more_in_it_than_names_with_the_words_is_left_to_the_ranking() {
        let hits = |paths: &[&str]| -> Vec<SearchHit> {
            paths
                .iter()
                .filter_map(|path| RawHit::file(path.into(), 0, 0, None, 0).into_hit("", true))
                .collect()
        };
        let words = vec!["tax".to_string()];
        let names = hits(&["/r/tax.txt", "/r/My-Tax-Notes.txt"]);
        assert!(sort_can_choose(&names, &words, false));
        // One hit that only its folder names, and the list is the ranking's:
        // a sort would take that hit out of it.
        let ranked = hits(&["/r/tax/opened-before.pdf", "/r/tax.txt"]);
        assert!(!sort_can_choose(&ranked, &words, false));

        // Without words every hit matches as well as the next.
        assert!(sort_can_choose(&ranked, &[], false));
        // Text matched loosely that the words leave out is no such search.
        assert!(!sort_can_choose(&names, &[], true));
        assert!(!sort_can_choose(&names, &words, true));
    }

    #[test]
    fn text_matched_loosely_is_told_from_the_words_and_the_filters() {
        let files = |query: &str| {
            let parsed = QueryParser::default().parse(query);
            loose_text(&parsed.fuzzy_query, query)
        };
        assert!(!files("tax report"));
        assert!(!files("tax *.pdf !draft src/"));
        assert!(!files("*.pdf"));
        assert!(!files(""));
        // fff-search reads these as `main.rs` and `10`, at a line.
        assert!(files("main.rs:42"));
        assert!(files("meeting 10:30"));

        // Where folders can be in the results, a folder token that is the
        // whole search is text as well.
        let with_folders = |query: &str| {
            let config = WithFolders {
                only: false,
                folder_filters: folder_tokens_filter(query),
            };
            let parsed = QueryParser::new(config).parse(query);
            loose_text(&parsed.fuzzy_query, query)
        };
        assert!(with_folders("src/"));
        assert!(!with_folders("src/ main"));
        assert!(!with_folders("main"));
    }

    #[test]
    fn names_that_arent_shown_dont_leave_a_sorted_list_short() {
        // Best first: three names with the word, two of them hidden, and a
        // file that only its folder names.
        let hits = || -> Vec<RawHit> {
            [
                ("/r/.tax-rc", 50),
                ("/r/.tax.env", 40),
                ("/r/tax.txt", 5),
                ("/r/tax/huge.bin", 9000),
            ]
            .into_iter()
            .map(|(path, size)| RawHit::file(path.into(), size, 0, None, 0))
            .collect()
        };
        let largest = SortOrder {
            key: SortKey::Size,
            ascending: false,
        };
        let words = vec!["tax".to_string()];
        let chosen = |raw: Vec<RawHit>, words: &[String], show_hidden: bool| {
            let cut = cut_in_order(raw, largest, 2, words, "tax", show_hidden)?;
            let names: Vec<String> = cut.0.into_iter().map(|hit| hit.entry.name).collect();
            Some((names, cut.1))
        };

        // Three names for two places: the largest two, and one is left out.
        let shown = chosen(hits(), &words, true).unwrap();
        assert_eq!(shown, (vec![".tax-rc".into(), ".tax.env".into()], true));
        // With the hidden ones not shown, one name is left for two places.
        // The search's own list has the file in the folder in the other.
        assert_eq!(chosen(hits(), &words, false), None);
        // Nothing is set aside where there are no words, so the one hit
        // that can be shown is the whole list.
        let mut all = hits();
        all.pop();
        let shown = chosen(all, &[], false).unwrap();
        assert_eq!(shown, (vec!["tax.txt".into()], false));
    }

    #[test]
    fn hits_are_sorted_before_the_cut_as_their_entries_are_listed() {
        let hits = || -> Vec<RawHit> {
            [
                ("/r/b/notes10.txt", 40, 300),
                ("/r/a/Notes2.txt", 40, 100),
                ("/r/a/empty.md", 0, 0),
                ("/r/c/photo.png", 7000, 200),
                ("/r/a/photo.png", 7000, 200),
            ]
            .into_iter()
            .map(|(path, size, modified)| RawHit::file(path.into(), size, modified, None, 0))
            .collect()
        };
        for key in [
            SortKey::Name,
            SortKey::Modified,
            SortKey::Size,
            SortKey::Kind,
            SortKey::Folder,
        ] {
            for ascending in [true, false] {
                let order = SortOrder { key, ascending };
                let mut raw = hits();
                assert!(sort_before_cut(&mut raw, order, 1, &[]));
                let entries: Vec<FileEntry> = raw
                    .into_iter()
                    .filter_map(|hit| hit.into_hit("", true))
                    .map(|hit| hit.entry)
                    .collect();
                assert_eq!(entries.len(), 5);
                let mut listed = entries.clone();
                listed.sort_by(|a, b| listing::compare(a, b, order));
                assert_eq!(entries, listed, "{order:?}");
            }
        }
    }

    #[test]
    fn a_sort_by_a_date_on_the_disk_reads_only_the_best_hits() {
        // Best first: a folder, then a file, and so on.
        let hits = || -> Vec<RawHit> {
            (0..SORT_READS + 10)
                .flat_map(|n| {
                    [
                        RawHit::dir(format!("/nowhere/folder-{n}").into(), 0),
                        RawHit::file(format!("/nowhere/file-{n}").into(), 0, 0, None, 0),
                    ]
                })
                .collect()
        };
        let sorted = |key: SortKey| {
            let order = SortOrder {
                key,
                ascending: true,
            };
            let mut raw = hits();
            assert!(sort_before_cut(&mut raw, order, 10, &[]));
            raw
        };
        let has = |read: &[RawHit], name: String| {
            let path = Path::new("/nowhere").join(name);
            read.iter().any(|hit| hit.path == path)
        };
        let every = 2 * (SORT_READS + 10);

        // Every hit is read for these, so the best of them all are sorted.
        for key in [SortKey::Created, SortKey::LastOpened] {
            let read = sorted(key);
            assert_eq!(read.len(), SORT_READS, "{key:?}");
            assert!(
                has(&read, format!("file-{}", SORT_READS / 2 - 1)),
                "{key:?}"
            );
            assert!(!has(&read, format!("folder-{}", SORT_READS / 2)), "{key:?}");
        }
        // The index holds when a file changed and not when a folder did, so
        // every file is sorted, with the best of the folders.
        let read = sorted(SortKey::Modified);
        assert_eq!(read.len(), every - 10);
        assert!(has(&read, format!("folder-{}", SORT_READS - 1)));
        assert!(!has(&read, format!("folder-{SORT_READS}")));
        // What the index holds takes no read, so every hit is sorted.
        assert_eq!(sorted(SortKey::Size).len(), every);
    }

    #[test]
    fn a_sort_chooses_the_hits_of_a_list_the_limit_cuts() {
        let dir = std::env::temp_dir().join(format!("fff-sorted-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("ledger")).unwrap();
        // The larger a log, the longer ago it changed.
        let now = SystemTime::now();
        for (name, size, hours) in [
            ("a.log", 10, 1),
            ("b.log", 3000, 4),
            ("c.log", 200, 3),
            ("d.log", 50, 2),
        ] {
            let path = dir.join(name);
            std::fs::write(&path, vec![b'x'; size]).unwrap();
            let file = std::fs::File::options().write(true).open(&path).unwrap();
            file.set_modified(now - Duration::from_secs(hours * 3600))
                .unwrap();
        }
        // Three files and a folder with a word in their names, and a file
        // that only the folder names.
        for (name, size) in [
            ("ledger.txt", 5),
            ("my-ledger-notes.txt", 3000),
            ("xledgerx.txt", 200),
            ("ledger/huge.bin", 9000),
        ] {
            std::fs::write(dir.join(name), vec![b'x'; size]).unwrap();
        }
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let find = |query: &str, key: SortKey, ascending: bool, limit: usize| {
            let found = engine.search(&SearchRequest {
                query: query.into(),
                limit,
                sort: Some(SortOrder { key, ascending }),
                ..Default::default()
            });
            let names: Vec<String> = found.hits.into_iter().map(|h| h.entry.name).collect();
            (names, found.truncated)
        };
        // The hits by name, as they come back best first and not sorted.
        let chosen = |query: &str, key: SortKey, ascending: bool, limit: usize| {
            let (mut names, _) = find(query, key, ascending, limit);
            names.sort();
            names
        };
        // What a search says of its list: whether a sort may choose its
        // items, as the search is made without a sort and by size.
        let may_choose = |query: &str, limit: usize| {
            let largest = SortOrder {
                key: SortKey::Size,
                ascending: false,
            };
            [None, Some(largest)].map(|sort| {
                let request = SearchRequest {
                    query: query.into(),
                    limit,
                    sort,
                    ..Default::default()
                };
                engine.search(&request).sort_chooses
            })
        };

        // Either end of all four logs, where the best two are the same two
        // whichever way the sort goes.
        let logs = |key, ascending| chosen("*.log", key, ascending, 2);
        assert_eq!(logs(SortKey::Size, false), ["b.log", "c.log"]);
        assert_eq!(logs(SortKey::Size, true), ["a.log", "d.log"]);
        assert_eq!(logs(SortKey::Modified, false), ["a.log", "d.log"]);
        assert_eq!(logs(SortKey::Modified, true), ["b.log", "c.log"]);
        assert_eq!(logs(SortKey::Name, false), ["c.log", "d.log"]);
        assert_eq!(logs(SortKey::Name, true), ["a.log", "b.log"]);
        assert!(find("*.log", SortKey::Size, false, 2).1, "two more match");
        assert_eq!(may_choose("*.log", 2), [true, true]);
        let (all, truncated) = find("*.log", SortKey::Size, false, 4);
        assert_eq!((all.len(), truncated), (4, false));
        assert_eq!(may_choose("*.log", 4), [false, false], "every match");

        // The largest three of the four names that have the word. The file
        // in the folder is larger than any and isn't among them, and the
        // best match comes first, small as it is.
        let (largest, truncated) = find("ledger", SortKey::Size, false, 3);
        assert_eq!(largest[0], "ledger.txt");
        assert_eq!(
            chosen("ledger", SortKey::Size, false, 3),
            ["ledger.txt", "my-ledger-notes.txt", "xledgerx.txt"]
        );
        assert!(truncated);
        assert_eq!(may_choose("ledger", 3), [true, true]);
        // All four fit, so the list is the one the search makes anyway. Only
        // a search made with the sort learns that there is no fifth name.
        assert_eq!(
            chosen("ledger", SortKey::Size, false, 4),
            [
                "ledger",
                "ledger.txt",
                "my-ledger-notes.txt",
                "xledgerx.txt"
            ]
        );
        assert_eq!(may_choose("ledger", 4), [true, false]);
        assert_eq!(chosen("ledger", SortKey::Size, false, 5).len(), 5);
        assert_eq!(may_choose("ledger", 5), [false, false], "every match");

        // A word with a line suffix is matched loosely and isn't one of the
        // words a name is held to, so its list stays the best matches, alone
        // or beside a word.
        for query in ["ledger:7", "ledger txt:7"] {
            let best = chosen(query, SortKey::Relevance, true, 2);
            assert!(best.contains(&"ledger.txt".to_string()), "{query}");
            assert_eq!(chosen(query, SortKey::Size, false, 2), best, "{query}");
            assert_eq!(may_choose(query, 2), [false, false], "{query}");
        }

        // A folder's date is read from the disk, where the index has none.
        let folder = CutKey::of(&RawHit::dir(dir.clone(), 0), SortKey::Modified);
        assert!(folder == CutKey::Date(listing::dir_stamp(&dir)));
        assert!(folder != CutKey::Date(None));
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Left out on Windows for the reason
    // `a_search_remembers_what_it_opened_whatever_the_capitals` is.
    #[cfg(not(windows))]
    #[test]
    fn a_best_match_opened_before_stays_in_the_list_under_a_sort() {
        let dir = std::env::temp_dir().join(format!("fff-sorted-opened-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let tree = dir.join("tree");
        std::fs::create_dir_all(tree.join("ledger")).unwrap();
        // Three files and a folder with the word in their names, and the
        // largest file of all, which only the folder names.
        for (name, size) in [
            ("ledger.txt", 5),
            ("my-ledger-notes.txt", 3000),
            ("xledgerx.txt", 200),
            ("ledger/summary.pdf", 9000),
        ] {
            std::fs::write(tree.join(name), vec![b'x'; size]).unwrap();
        }
        let engine = Engine::new(EngineOptions {
            roots: vec![tree.clone()],
            app_dirs: Vec::new(),
            db_dir: Some(dir.join("db")),
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let find = |sort: Option<SortOrder>| {
            let found = engine.search(&SearchRequest {
                query: "ledger".into(),
                limit: 3,
                sort,
                ..Default::default()
            });
            let mut names: Vec<String> = found.hits.into_iter().map(|h| h.entry.name).collect();
            let first = names[0].clone();
            names.sort();
            (first, names, found.sort_chooses)
        };
        let largest = Some(SortOrder {
            key: SortKey::Size,
            ascending: false,
        });

        // Four names for three places, so the sort chooses the largest.
        let (_, names, chooses) = find(largest);
        assert_eq!(names, ["ledger.txt", "my-ledger-notes.txt", "xledgerx.txt"]);
        assert!(chooses);

        // Opened from this search, the file in the folder is its best match.
        engine.track_open(&tree.join("ledger/summary.pdf"), Some("ledger"));
        let (first, best, chooses) = find(None);
        assert_eq!(first, "summary.pdf");
        assert_eq!(best, ["ledger", "ledger.txt", "summary.pdf"]);
        assert!(!chooses, "a sort would take the best match out");
        // So the list stays the best matches whatever it is sorted by.
        assert_eq!(find(largest), (first, best, false));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_sort_chooses_among_more_matches_than_a_search_ranks() {
        let dir = std::env::temp_dir().join(format!("fff-sorted-wide-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // A search for two items is handed the 2,000 that changed last.
        for n in 0..2_050 {
            std::fs::write(dir.join(format!("new-{n:04}.log")), "x").unwrap();
        }
        // The largest of them all changed before any of those.
        let old = dir.join("old.log");
        std::fs::write(&old, vec![b'x'; 5000]).unwrap();
        let file = std::fs::File::options().write(true).open(&old).unwrap();
        let day = Duration::from_secs(24 * 3600);
        file.set_modified(SystemTime::now() - day).unwrap();
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(20)));

        let largest = engine.search(&SearchRequest {
            query: "*.log".into(),
            limit: 2,
            sort: Some(SortOrder {
                key: SortKey::Size,
                ascending: false,
            }),
            ..Default::default()
        });
        let names: Vec<&str> = largest.hits.iter().map(|h| h.entry.name.as_str()).collect();
        assert!(names.contains(&"old.log"), "{names:?}");
        assert!(largest.truncated);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_search_reads_dates_and_permissions_only_when_asked() {
        let dir = std::env::temp_dir().join(format!("fff-metadata-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("ledger.txt");
        std::fs::write(&file, "x").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o640)).unwrap();
        }
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let find = |request: SearchRequest| {
            let mut hits = engine
                .search(&SearchRequest {
                    query: "ledger".into(),
                    limit: 10,
                    ..request
                })
                .hits;
            assert_eq!(hits.len(), 1);
            hits.remove(0).entry
        };
        let by = |key: SortKey| {
            Some(SortOrder {
                key,
                ascending: true,
            })
        };
        // Not every file system records when a file was made.
        let created = std::fs::metadata(&file).unwrap().created().ok();
        let mode = cfg!(unix).then_some(0o640);

        let plain = find(SearchRequest::default());
        assert_eq!(
            (plain.created, plain.mode, plain.last_opened),
            (None, None, None),
            "the index holds none of them"
        );
        let full = find(SearchRequest {
            full_metadata: true,
            ..Default::default()
        });
        assert_eq!((full.created, full.mode), (created, mode));
        assert_eq!(full.last_opened, None);
        assert_eq!(full.size, Some(1), "the rest is as the index has it");
        let opened = find(SearchRequest {
            last_opened: true,
            ..Default::default()
        });
        assert!(opened.last_opened.is_some());
        assert_eq!(opened.mode, None);

        // A sort by one of the dates reads it without being asked.
        let sorted = find(SearchRequest {
            sort: by(SortKey::Created),
            ..Default::default()
        });
        assert_eq!((sorted.created, sorted.mode), (created, mode));
        let sorted = find(SearchRequest {
            sort: by(SortKey::LastOpened),
            ..Default::default()
        });
        assert!(sorted.last_opened.is_some());
        assert_eq!(
            find(SearchRequest {
                sort: by(SortKey::Size),
                ..Default::default()
            })
            .mode,
            None
        );

        // A file that has gone since it was indexed is listed as it was.
        std::fs::remove_file(&file).unwrap();
        let gone = find(SearchRequest {
            full_metadata: true,
            last_opened: true,
            ..Default::default()
        });
        assert_eq!(
            (gone.created, gone.mode, gone.last_opened),
            (None, None, None)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_content_search_told_to_stop_reads_no_more_than_a_few_files() {
        let dir = std::env::temp_dir().join(format!("fff-grep-stop-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let roots = vec![dir.join("one"), dir.join("two")];
        for root in &roots {
            std::fs::create_dir_all(root).unwrap();
            for n in 0..200 {
                std::fs::write(root.join(format!("{n}.txt")), "needle\n").unwrap();
            }
        }
        let engine = Engine::new(EngineOptions {
            roots,
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let request = GrepRequest {
            patterns: vec!["needle".into()],
            limit: 1000,
            ..Default::default()
        };
        assert_eq!(engine.grep(&request).hits.len(), 400);

        let stop = Arc::new(AtomicBool::new(false));
        let whole = engine.grep_cancellable(&request, Some(&stop));
        assert_eq!(whole.hits.len(), 400, "nothing told it to stop");

        // fff-search looks at the flag every eighth file, and always reads
        // the first one.
        stop.store(true, Ordering::Relaxed);
        let cut = engine.grep_cancellable(&request, Some(&stop));
        assert!(!cut.hits.is_empty());
        assert!(cut.hits.len() < 50, "{} hits", cut.hits.len());
        assert!(cut.files_searched < 50, "{} files", cut.files_searched);
        assert!(
            cut.hits
                .iter()
                .all(|hit| hit.path.parent().is_some_and(|p| p.ends_with("one"))),
            "the second location isn't searched at all"
        );
        assert!(!cut.stopped, "it wasn't the time limit that ended it");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_content_search_says_when_its_limit_left_matches_out() {
        let dir = std::env::temp_dir().join(format!("fff-grep-more-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for n in 0..30 {
            std::fs::write(dir.join(format!("{n:02}.txt")), "needle\n".repeat(25)).unwrap();
        }
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let grep = |limit: usize, per_file: usize| {
            engine.grep(&GrepRequest {
                patterns: vec!["needle".into()],
                limit,
                per_file,
                ..Default::default()
            })
        };
        let files = |response: &GrepResponse| {
            let paths: HashSet<&PathBuf> = response.hits.iter().map(|hit| &hit.path).collect();
            paths.len()
        };

        // With one match kept for a file, the limit counts files.
        let ten = grep(10, 1);
        assert_eq!((ten.hits.len(), files(&ten)), (10, 10));
        assert!(ten.more, "twenty more files have the word");
        assert!(!ten.stopped);

        let all = grep(100, 1);
        assert_eq!((all.hits.len(), files(&all)), (30, 30));
        assert!(!all.more);
        assert!(!all.stopped);

        // A limit met by the last file there is left nothing out.
        let exact = grep(30, 1);
        assert_eq!(files(&exact), 30);
        assert!(!exact.more);

        // The limit is in matches: twenty for a file reach 240 in twelve files.
        let lines = grep(240, 20);
        assert_eq!((lines.hits.len(), files(&lines)), (240, 12));
        assert!(lines.more);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_content_search_says_when_its_limit_left_out_part_of_a_later_root() {
        let dir = std::env::temp_dir().join(format!("fff-grep-roots-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let roots = vec![dir.join("one"), dir.join("two")];
        for (root, count) in roots.iter().zip([5, 18]) {
            std::fs::create_dir_all(root).unwrap();
            for n in 0..count {
                std::fs::write(root.join(format!("{n}.txt")), "needle\n").unwrap();
            }
        }
        let engine = Engine::new(EngineOptions {
            roots,
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let grep = |limit: usize| {
            engine.grep(&GrepRequest {
                patterns: vec!["needle".into()],
                limit,
                per_file: 1,
                ..Default::default()
            })
        };
        let in_one = |response: &GrepResponse| {
            let one = |hit: &&GrepHit| hit.path.parent().is_some_and(|p| p.ends_with("one"));
            response.hits.iter().filter(one).count()
        };

        // The first root has just the five, and the second isn't searched.
        let first = grep(5);
        assert_eq!((first.hits.len(), in_one(&first)), (5, 5));
        assert!(first.more, "the second root may have the word too");

        // The second root has eighteen, and fifteen of them fit.
        let most = grep(20);
        assert_eq!((most.hits.len(), in_one(&most)), (20, 5));
        assert!(most.more, "three matches of the second root were left out");

        let all = grep(23);
        assert_eq!((all.hits.len(), in_one(&all)), (23, 5));
        assert!(!all.more, "the limit was met by the last match there is");
        assert!(!all.stopped);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_with_a_space_in_its_name_isnt_crowded_out_of_a_content_search() {
        let dir = std::env::temp_dir().join(format!("fff-grep-space-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // Such a folder is searched along with everything around it, newer
        // files first, and what was found elsewhere is dropped afterwards.
        let folder = dir.join("my folder");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::create_dir_all(dir.join("elsewhere")).unwrap();
        let hour_ago = SystemTime::now() - Duration::from_secs(3600);
        for n in 0..3 {
            let path = folder.join(format!("{n}.txt"));
            std::fs::write(&path, "needle\n").unwrap();
            let file = std::fs::File::options().write(true).open(&path).unwrap();
            file.set_modified(hour_ago).unwrap();
        }
        for n in 0..20 {
            std::fs::write(dir.join(format!("elsewhere/{n}.txt")), "needle\n").unwrap();
        }
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let grep = |limit: usize| {
            engine.grep(&GrepRequest {
                patterns: vec!["needle".into()],
                scope: Some(folder.clone()),
                limit,
                per_file: 1,
                ..Default::default()
            })
        };

        // Ten matches are found elsewhere before the folder's first. The
        // search goes on past them, and so gets to every file there is.
        let found = grep(10);
        assert_eq!(found.hits.len(), 3);
        assert!(!found.more);

        // It doesn't go on for ever. Where it ends before the folder's
        // files are all found, it says that more may match.
        let cut = grep(2);
        assert!(cut.hits.len() <= 2);
        assert!(cut.more);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_full_page_isnt_taken_for_the_time_limit() {
        // A page of ten with the next page's first file, and the time up.
        assert_eq!(page_end(10, 10, 5, true), PageEnd::Full);
        assert_eq!(page_end(3, 10, 5, true), PageEnd::Stopped);
        assert_eq!(page_end(3, 10, 0, true), PageEnd::Complete);
        // Short of a page with time left: the search was told to stop.
        assert_eq!(page_end(3, 10, 5, false), PageEnd::Full);
        assert_eq!(page_end(10, 10, 0, false), PageEnd::Complete);
    }

    #[test]
    fn the_roots_of_a_content_search_share_one_time_limit() {
        let secs = Duration::from_secs;
        assert_eq!(root_budget_ms(5, secs(0)), Some(5000));
        assert_eq!(root_budget_ms(5, secs(2)), Some(3000));
        assert_eq!(root_budget_ms(5, secs(5)), None);
        assert_eq!(root_budget_ms(5, secs(6)), None);
        // A last moment isn't rounded down to 0, which would be no limit.
        assert_eq!(root_budget_ms(5, Duration::from_micros(4_999_600)), Some(1));
        assert_eq!(root_budget_ms(0, secs(600)), Some(0));
    }

    #[test]
    fn search_settings_change_content_search() {
        let dir = std::env::temp_dir().join(format!("fff-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "Hello there\n").unwrap();
        std::fs::write(dir.join("b.txt"), "hello there\n").unwrap();
        std::fs::write(dir.join("small.txt"), "needle\n").unwrap();
        let big = format!("needle\n{}\n", "x".repeat(2 * 1024 * 1024));
        std::fs::write(dir.join("big.txt"), big).unwrap();
        let engine = Engine::new(EngineOptions {
            roots: vec![dir.clone()],
            app_dirs: Vec::new(),
            db_dir: None,
            watch: false,
            search: Default::default(),
        });
        assert!(engine.wait_ready(Duration::from_secs(10)));
        let grep = |text: &str| {
            let mut names: Vec<String> = engine
                .grep(&GrepRequest {
                    patterns: vec![text.into()],
                    limit: 50,
                    ..Default::default()
                })
                .hits
                .iter()
                .map(|h| display_name(&h.path))
                .collect();
            names.sort();
            names
        };
        let set = |edit: &dyn Fn(&mut SearchSettings)| {
            let mut settings = engine.search_settings();
            edit(&mut settings);
            engine.set_search_settings(settings);
        };

        // Smart: lowercase ignores case, a capital letter matches it.
        assert_eq!(grep("hello"), ["a.txt", "b.txt"]);
        assert_eq!(grep("Hello"), ["a.txt"]);
        set(&|s| s.case = SearchCase::Sensitive);
        assert_eq!(grep("hello"), ["b.txt"]);
        set(&|s| s.case = SearchCase::Insensitive);
        assert_eq!(grep("Hello"), ["a.txt", "b.txt"]);

        assert_eq!(grep("needle"), ["big.txt", "small.txt"]);
        set(&|s| s.max_file_size_mb = 1);
        assert_eq!(grep("needle"), ["small.txt"]);

        set(&|s| s.max_file_size_mb = 10);
        assert_eq!(grep("needle"), ["big.txt", "small.txt"]);

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
