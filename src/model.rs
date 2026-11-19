//! Browser state that doesn't depend on GPUI: navigation history and selection.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// What the main area shows when no search is active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    Folder(PathBuf),
    /// Recently used files across the search roots.
    Recents,
    /// Files sent from elsewhere, like `fff show` in Nushell, shown the way
    /// search results are.
    Results {
        title: String,
        paths: Arc<Vec<PathBuf>>,
    },
    /// What's in the Trash.
    Trash,
}

impl Location {
    pub fn folder(&self) -> Option<&Path> {
        match self {
            Location::Folder(path) => Some(path),
            Location::Recents | Location::Results { .. } | Location::Trash => None,
        }
    }

    pub fn title(&self) -> String {
        match self {
            Location::Folder(path) => fffinder_core::entry::display_name(path),
            Location::Recents => "Recents".into(),
            Location::Results { title, .. } => title.clone(),
            Location::Trash => "Trash".into(),
        }
    }

    /// Go where the folder went, when it moved or was inside one that did.
    /// Returns whether it moved.
    fn follow(&mut self, moves: &Moves) -> bool {
        let Location::Folder(path) = self else {
            return false;
        };
        match moves.follow(path) {
            Some(to) => {
                *path = to;
                true
            }
            None => false,
        }
    }
}

/// `path` with `from` at its start swapped for `to`.
///
/// # Arguments
///
/// * `path` - The path to move
/// * `from` - Where a folder was
/// * `to` - Where it is now
///
/// # Returns
///
/// The path inside the moved folder, or `None` when `path` isn't in `from`
pub fn rebase(path: &Path, from: &Path, to: &Path) -> Option<PathBuf> {
    let rest = path.strip_prefix(from).ok()?;
    // Joining an empty path would add a trailing separator.
    Some(if rest.as_os_str().is_empty() {
        to.to_path_buf()
    } else {
        to.join(rest)
    })
}

/// Items that were renamed or moved, looked up by where each was, to tell
/// where a path that was in one of them is now.
#[derive(Debug, Clone, Default)]
pub struct Moves<'a>(HashMap<&'a Path, &'a Path>);

impl<'a> Moves<'a> {
    /// Moves from each pair's first path to its second.
    ///
    /// # Arguments
    ///
    /// * `pairs` - Where each item was, and where it is now
    ///
    /// # Returns
    ///
    /// The moves, ready to look up
    pub fn new(pairs: &'a [(PathBuf, PathBuf)]) -> Moves<'a> {
        Moves(
            pairs
                .iter()
                .map(|(from, to)| (from.as_path(), to.as_path()))
                .collect(),
        )
    }

    /// Where `path` is now. Each folder above it is looked up, so this
    /// takes as many lookups as the path is deep, however many items moved.
    ///
    /// # Arguments
    ///
    /// * `path` - Where something was
    ///
    /// # Returns
    ///
    /// Where it is now, or `None` when neither it nor a folder holding it
    /// moved
    pub fn follow(&self, path: &Path) -> Option<PathBuf> {
        path.ancestors()
            .find_map(|dir| rebase(path, dir, self.0.get(dir)?))
    }
}

/// A search as the window showed it, for Back and Forward to run again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedSearch {
    pub query: String,
    /// It looked only in the folder the window showed.
    pub in_folder: bool,
    /// It looked inside files, not at their names.
    pub contents: bool,
}

/// What the window is called while it shows a search for `words`.
pub fn search_title(words: &str) -> String {
    format!("Searching “{words}”")
}

/// What the window showed at one point in its history: a place, or a search
/// made there. The place a search was made in always comes before the search,
/// whether or not the history holds a visit for that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Visit {
    pub location: Location,
    pub search: Option<SavedSearch>,
}

impl Visit {
    fn seen(&self) -> Seen<'_> {
        Seen {
            location: &self.location,
            search: self.search.as_ref(),
        }
    }
}

/// A [`Visit`] where the history has it, to compare and list without a copy.
#[derive(Clone, Copy, PartialEq)]
struct Seen<'a> {
    location: &'a Location,
    search: Option<&'a SavedSearch>,
}

impl Seen<'_> {
    fn visit(self) -> Visit {
        Visit {
            location: self.location.clone(),
            search: self.search.cloned(),
        }
    }
}

/// Where Back or Forward led.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// The window showed another location on the way, or shows one now. It
    /// doesn't between a search and the place the search was made in.
    pub moved: bool,
    /// The search to show there.
    pub search: Option<SavedSearch>,
}

#[derive(Debug, Clone)]
pub struct History {
    back: Vec<Visit>,
    forward: Vec<Visit>,
    current: Location,
}

impl History {
    pub fn new(start: Location) -> History {
        History {
            back: Vec::new(),
            forward: Vec::new(),
            current: start,
        }
    }

    pub fn current(&self) -> &Location {
        &self.current
    }

    /// Go somewhere new; clears the forward stack. A search the window shows
    /// stays behind for Back to return to. No-op for the current location,
    /// unless a search is left there.
    ///
    /// # Arguments
    ///
    /// * `to` - Where to go
    /// * `live` - The search the window shows, which going there ends
    ///
    /// # Returns
    ///
    /// Whether the location changed
    pub fn navigate(&mut self, to: Location, live: Option<SavedSearch>) -> bool {
        let moved = to != self.current;
        if !moved && live.is_none() {
            return false;
        }
        let from = std::mem::replace(&mut self.current, to);
        self.back.push(Visit {
            location: from,
            search: live,
        });
        self.forward.clear();
        moved
    }

    pub fn back(&mut self, live: Option<SavedSearch>) -> Option<Step> {
        self.back_by(1, live)
    }

    pub fn forward(&mut self, live: Option<SavedSearch>) -> Option<Step> {
        self.forward_by(1, live)
    }

    /// Go back `steps` steps at once, to where that many single steps would
    /// lead. With fewer steps than that to go back, it stays where it is
    /// rather than go as far as it can. Back from a search is one step, to
    /// the place the search was made in, and Forward returns to the search.
    ///
    /// # Arguments
    ///
    /// * `steps` - How many steps to go back; 1 is the nearest
    /// * `live` - The search the window shows
    ///
    /// # Returns
    ///
    /// What to show now, or `None` if it went nowhere
    pub fn back_by(&mut self, steps: usize, live: Option<SavedSearch>) -> Option<Step> {
        self.go(false, steps, live)
    }

    /// Go forward `steps` steps at once, or stay put when there are fewer,
    /// as [`History::back_by`] does.
    ///
    /// # Arguments
    ///
    /// * `steps` - How many steps to go forward; 1 is the nearest
    /// * `live` - The search the window shows
    ///
    /// # Returns
    ///
    /// What to show now, or `None` if it went nowhere
    pub fn forward_by(&mut self, steps: usize, live: Option<SavedSearch>) -> Option<Step> {
        self.go(true, steps, live)
    }

    /// Whether Back leads anywhere, which from a search it always does.
    pub fn can_back(&self, live: Option<&SavedSearch>) -> bool {
        self.steps(false, live).next().is_some()
    }

    /// Whether Forward leads anywhere.
    pub fn can_forward(&self, live: Option<&SavedSearch>) -> bool {
        self.steps(true, live).next().is_some()
    }

    /// What Back leads to, a step at a time and nearest first: places, and
    /// searches the window left. From a search, the nearest is the place it
    /// was made in.
    ///
    /// # Arguments
    ///
    /// * `live` - The search the window shows
    pub fn back_list<'a>(
        &'a self,
        live: Option<&'a SavedSearch>,
    ) -> impl Iterator<Item = Visit> + 'a {
        self.steps(false, live).map(|(_, stop)| stop.visit())
    }

    /// What Forward leads to, a step at a time and nearest first.
    ///
    /// # Arguments
    ///
    /// * `live` - The search the window shows
    pub fn forward_list<'a>(
        &'a self,
        live: Option<&'a SavedSearch>,
    ) -> impl Iterator<Item = Visit> + 'a {
        self.steps(true, live).map(|(_, stop)| stop.visit())
    }

    /// The steps Back takes from what the window shows, or with `forward`
    /// the steps Forward takes: where each one leads, and how many of the
    /// visits that way are left beyond it. The lists and the steps
    /// themselves are both made from this, so that they can't disagree.
    fn steps<'a>(
        &'a self,
        forward: bool,
        live: Option<&'a SavedSearch>,
    ) -> impl Iterator<Item = (usize, Seen<'a>)> + 'a {
        let visits = if forward { &self.forward } else { &self.back };
        let mut left = visits.len();
        let mut shown = Seen {
            location: &self.current,
            search: live,
        };
        std::iter::from_fn(move || {
            left = past_repeats(&visits[..left], shown);
            if shown.search.is_some() && !forward {
                // Back from a search leads to the place it was made in.
                shown.search = None;
            } else {
                left = left.checked_sub(1)?;
                shown = visits[left].seen();
            }
            Some((left, shown))
        })
    }

    /// Take `steps` steps back, or with `forward` that many forward, if
    /// there are as many.
    fn go(&mut self, forward: bool, steps: usize, live: Option<SavedSearch>) -> Option<Step> {
        let mut stops: Vec<(usize, Visit)> = self
            .steps(forward, live.as_ref())
            .take(steps)
            .map(|(left, stop)| (left, stop.visit()))
            .collect();
        if stops.len() < steps {
            return None;
        }
        let moved = stops.iter().any(|(_, stop)| stop.location != self.current);
        // No steps at all leave no stop to go to.
        let (left, to) = stops.pop()?;
        let (taken, given) = match forward {
            true => (&mut self.forward, &mut self.back),
            false => (&mut self.back, &mut self.forward),
        };
        taken.truncate(left);
        // The other way now leads through what the window showed, and
        // through each stop that was passed, again.
        given.push(Visit {
            location: std::mem::replace(&mut self.current, to.location),
            search: live,
        });
        given.extend(stops.into_iter().map(|(_, stop)| stop));
        Some(Step {
            moved,
            search: to.search,
        })
    }

    /// Keep up with items that moved: every folder in the history that was
    /// one of them, or inside one, is now where it went.
    ///
    /// # Arguments
    ///
    /// * `moves` - What moved, and where
    ///
    /// # Returns
    ///
    /// Whether the current location moved
    pub fn follow(&mut self, moves: &Moves) -> bool {
        for visit in self.back.iter_mut().chain(&mut self.forward) {
            visit.location.follow(moves);
        }
        self.current.follow(moves)
    }
}

/// How many of `visits` are left after the nearest ones, at the end, that
/// are just `shown`, which is what the window shows by then. Going to one of
/// those would change nothing, so it's no step. One is there when a search
/// that Back or Forward led to was ended some other way, as with Escape; when
/// the same search was made twice in a row, as by opening a Smart Folder
/// again; and behind a search that Forward led to from the place it was made
/// in.
fn past_repeats(visits: &[Visit], shown: Seen<'_>) -> usize {
    let repeats = visits
        .iter()
        .rev()
        .take_while(|visit| visit.seen() == shown);
    visits.len() - repeats.count()
}

/// Multi-selection over a list of items, Finder style: click, Cmd-click
/// toggles, Shift-click and Shift-arrows extend from an anchor.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    items: BTreeSet<usize>,
    anchor: Option<usize>,
    cursor: Option<usize>,
    /// Counts each change to `items`.
    revision: u64,
}

impl Selection {
    pub fn clear(&mut self) {
        *self = Selection {
            revision: self.revision + 1,
            ..Selection::default()
        };
    }

    /// Tells one set of selected items from another without comparing them.
    ///
    /// # Returns
    ///
    /// A number that changes whenever the selected items may have changed
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn contains(&self, ix: usize) -> bool {
        self.items.contains(&ix)
    }

    pub fn indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.items.iter().copied()
    }

    /// The focused item: last clicked or moved to.
    pub fn cursor(&self) -> Option<usize> {
        self.cursor
    }

    pub fn select_only(&mut self, ix: usize) {
        self.revision += 1;
        self.items.clear();
        self.items.insert(ix);
        self.anchor = Some(ix);
        self.cursor = Some(ix);
    }

    pub fn toggle(&mut self, ix: usize) {
        self.revision += 1;
        if !self.items.remove(&ix) {
            self.items.insert(ix);
        }
        self.anchor = Some(ix);
        self.cursor = Some(ix);
    }

    /// Select the contiguous range from the anchor to `ix`.
    pub fn extend_to(&mut self, ix: usize) {
        self.revision += 1;
        let anchor = self.anchor.unwrap_or(ix);
        self.items = (anchor.min(ix)..=anchor.max(ix)).collect();
        self.anchor = Some(anchor);
        self.cursor = Some(ix);
    }

    /// Select exactly `items`, leaving the cursor on the last one and the
    /// anchor on the first, as a drag across them would.
    ///
    /// # Arguments
    ///
    /// * `items` - The indices to select
    pub fn select_items(&mut self, items: BTreeSet<usize>) {
        self.revision += 1;
        self.anchor = items.first().copied();
        self.cursor = items.last().copied();
        self.items = items;
    }

    pub fn select_all(&mut self, len: usize) {
        self.revision += 1;
        self.items = (0..len).collect();
        if len > 0 {
            self.anchor.get_or_insert(0);
            self.cursor.get_or_insert(0);
        }
    }

    /// Arrow-key movement by `delta` items, clamped to the list.
    pub fn move_by(&mut self, delta: isize, extend: bool, len: usize) {
        if len == 0 {
            return;
        }
        let target = match self.cursor {
            Some(cur) => (cur as isize + delta).clamp(0, len as isize - 1) as usize,
            None if delta < 0 => len - 1,
            None => 0,
        };
        if extend {
            self.extend_to(target);
        } else {
            self.select_only(target);
        }
    }

    /// Move the cursor `step` selected items along, wrapping around, and keep
    /// the selection: Quick Look steps through several items this way. A
    /// cursor outside the selection counts as the first selected item.
    pub fn cycle_cursor(&mut self, step: isize) {
        let items: Vec<usize> = self.items.iter().copied().collect();
        if items.is_empty() {
            return;
        }
        let pos = self
            .cursor
            .and_then(|cur| items.iter().position(|&ix| ix == cur))
            .unwrap_or(0);
        let next = (pos as isize + step).rem_euclid(items.len() as isize) as usize;
        self.cursor = Some(items[next]);
    }

    /// Keep the same items selected after the list was re-sorted or refreshed.
    pub fn remap(&mut self, old: &[PathBuf], new: &[PathBuf]) {
        self.revision += 1;
        let index_of = |path: &PathBuf| new.iter().position(|p| p == path);
        let items: BTreeSet<usize> = self
            .items
            .iter()
            .filter_map(|&ix| old.get(ix).and_then(index_of))
            .collect();
        let map = |ix: Option<usize>| ix.and_then(|ix| old.get(ix)).and_then(index_of);
        self.anchor = map(self.anchor);
        self.cursor = map(self.cursor);
        self.items = items;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selecting_a_set_of_items_runs_from_the_first_to_the_last() {
        let mut selection = Selection::default();

        selection.select_items([2, 5, 7].into_iter().collect());

        assert_eq!(selection.indices().collect::<Vec<_>>(), [2, 5, 7]);
        assert_eq!(selection.cursor(), Some(7));
        // Shift+click starts again from the first of them.
        selection.extend_to(3);
        assert_eq!(selection.indices().collect::<Vec<_>>(), [2, 3]);

        selection.select_items(Default::default());
        assert!(selection.is_empty());
        assert_eq!(selection.cursor(), None);
    }

    #[test]
    fn the_revision_changes_with_the_selected_items() {
        let mut s = Selection::default();
        let mut seen = vec![s.revision()];
        let mut changed = |s: &Selection| {
            assert!(!seen.contains(&s.revision()), "{seen:?}");
            seen.push(s.revision());
        };

        s.select_only(1);
        changed(&s);
        s.toggle(2);
        changed(&s);
        s.extend_to(4);
        changed(&s);
        s.select_items([3].into_iter().collect());
        changed(&s);
        s.select_all(9);
        changed(&s);
        s.move_by(1, false, 9);
        changed(&s);
        s.clear();
        changed(&s);
        s.select_only(1);
        changed(&s);
        let paths: Vec<PathBuf> = ["/a", "/b"].map(PathBuf::from).into();
        s.remap(&paths, &paths);
        changed(&s);
    }

    #[test]
    fn history_back_forward() {
        let a = Location::Folder("/a".into());
        let b = Location::Folder("/b".into());
        let c = Location::Folder("/c".into());
        let mut h = History::new(a.clone());
        assert!(h.navigate(b.clone(), None));
        assert!(!h.navigate(b.clone(), None));
        h.navigate(c.clone(), None);
        assert!(h.back(None).is_some());
        assert_eq!(h.current(), &b);
        assert!(h.can_forward(None));
        h.navigate(Location::Recents, None);
        assert!(!h.can_forward(None));
        assert!(h.back(None).is_some() && h.back(None).is_some());
        assert_eq!(h.current(), &a);
        assert!(h.back(None).is_none());
    }

    #[test]
    fn history_goes_several_places_at_once() {
        let [a, b, c, d] = ["/a", "/b", "/c", "/d"].map(|p| Location::Folder(p.into()));
        let mut h = History::new(a.clone());
        for to in [&b, &c, &d] {
            h.navigate(to.clone(), None);
        }
        assert_eq!(names(h.back_list(None)), ["c", "b", "a"]);
        assert_eq!(h.forward_list(None).count(), 0);

        // More steps than there are places, or none, goes nowhere.
        assert!(h.back_by(4, None).is_none() && h.back_by(0, None).is_none());
        assert_eq!(h.current(), &d);
        assert_eq!(h.back_list(None).count(), 3);

        // Two at once end where two single steps do.
        let mut stepped = h.clone();
        assert!(stepped.back(None).is_some() && stepped.back(None).is_some());
        assert!(h.back_by(2, None).is_some());
        assert_eq!(h.current(), &b);
        assert_eq!(names(h.back_list(None)), ["a"]);
        assert_eq!(names(h.forward_list(None)), ["c", "d"]);
        assert_eq!(stepped.current(), h.current());
        assert!(stepped.back_list(None).eq(h.back_list(None)));
        assert!(stepped.forward_list(None).eq(h.forward_list(None)));

        assert!(h.forward_by(3, None).is_none() && h.forward_by(0, None).is_none());
        assert_eq!(h.current(), &b);
        assert!(h.forward_by(2, None).is_some());
        assert_eq!(h.current(), &d);
        assert_eq!(names(h.back_list(None)), ["c", "b", "a"]);
        assert!(!h.can_forward(None));

        assert!(h.back_by(3, None).is_some());
        assert_eq!(h.current(), &a);
        assert!(!h.can_back(None));
        assert_eq!(names(h.forward_list(None)), ["b", "c", "d"]);
    }

    #[test]
    fn rebase_moves_paths_inside_the_folder() {
        let (from, to) = (Path::new("/a/old"), Path::new("/a/new"));
        assert_eq!(
            rebase(Path::new("/a/old"), from, to),
            Some(PathBuf::from("/a/new"))
        );
        assert_eq!(
            rebase(Path::new("/a/old/x/y"), from, to),
            Some(PathBuf::from("/a/new/x/y"))
        );
        assert_eq!(rebase(Path::new("/a/older"), from, to), None);
        assert_eq!(rebase(Path::new("/b"), from, to), None);
    }

    #[test]
    fn moves_follow_paths_inside_what_moved() {
        let pairs = [
            (PathBuf::from("/a/old"), PathBuf::from("/b/new")),
            (PathBuf::from("/c"), PathBuf::from("/d")),
        ];
        let moves = Moves::new(&pairs);
        assert_eq!(moves.follow(Path::new("/a/old")), Some("/b/new".into()));
        assert_eq!(moves.follow(Path::new("/a/old/x")), Some("/b/new/x".into()));
        assert_eq!(moves.follow(Path::new("/c/y/z")), Some("/d/y/z".into()));
        assert_eq!(moves.follow(Path::new("/a")), None);
        assert_eq!(moves.follow(Path::new("/a/older")), None);
    }

    #[test]
    fn history_follows_folders_that_moved() {
        let pairs = [(PathBuf::from("/p/old"), PathBuf::from("/p/new"))];
        let moves = Moves::new(&pairs);
        let mut h = History::new(Location::Folder("/p/old/a".into()));
        h.navigate(Location::Folder("/p".into()), None);
        h.navigate(Location::Folder("/p/old".into()), None);
        h.back(None);
        assert!(!h.follow(&moves), "the current folder stayed put");
        assert_eq!(h.current(), &Location::Folder("/p".into()));
        h.forward(None);
        assert_eq!(h.current(), &Location::Folder("/p/new".into()));
        h.back(None);
        h.back(None);
        assert_eq!(h.current(), &Location::Folder("/p/new/a".into()));

        let mut shown = History::new(Location::Folder("/p/old".into()));
        assert!(shown.follow(&moves));
        assert!(!shown.can_back(None), "following isn't going somewhere");
        assert_eq!(shown.current(), &Location::Folder("/p/new".into()));
    }

    /// A search for `query`, by name, in every search location.
    fn search(query: &str) -> Option<SavedSearch> {
        Some(SavedSearch {
            query: query.into(),
            in_folder: false,
            contents: false,
        })
    }

    /// What Back or Forward answers with when it led somewhere.
    fn step(moved: bool, search: Option<SavedSearch>) -> Option<Step> {
        Some(Step { moved, search })
    }

    /// How a list of visits reads: each place by its name, and a search by
    /// its words after the name of the place it was made in.
    fn names(visits: impl Iterator<Item = Visit>) -> Vec<String> {
        visits
            .map(|visit| match visit.search {
                Some(search) => format!("{}?{}", visit.location.title(), search.query),
                None => visit.location.title(),
            })
            .collect()
    }

    #[test]
    fn history_goes_back_to_a_search_and_then_to_its_folder() {
        let [x, a, b] = ["/x", "/a", "/b"].map(|p| Location::Folder(p.into()));
        let mut h = History::new(x.clone());
        h.navigate(a.clone(), None);
        // A folder opened from the results of a search made in `a`.
        assert!(h.navigate(b.clone(), search("atlas")));
        assert_eq!(names(h.back_list(None)), ["a?atlas", "a", "x"]);

        // Every step back shows something else: the results, the folder they
        // were found from, and the folder before it.
        assert_eq!(h.back(None), step(true, search("atlas")));
        assert_eq!(h.current(), &a);
        assert_eq!(h.back(search("atlas")), step(false, None));
        assert_eq!(h.current(), &a);
        assert_eq!(h.back(None), step(true, None));
        assert_eq!(h.current(), &x);
        assert_eq!(h.back(None), None);
        assert_eq!(names(h.forward_list(None)), ["a", "a?atlas", "b"]);

        // And every step forward, the same three the other way.
        assert_eq!(h.forward(None), step(true, None));
        assert_eq!(h.current(), &a);
        assert_eq!(h.forward(None), step(false, search("atlas")));
        assert_eq!(h.current(), &a);
        assert_eq!(h.forward(search("atlas")), step(true, None));
        assert_eq!(h.current(), &b);
        assert_eq!(h.forward(None), None);
        assert_eq!(names(h.back_list(None)), ["a?atlas", "a", "x"]);

        // Three at once pass through the search, and two end in its folder.
        assert_eq!(h.back_by(3, None), step(true, None));
        assert_eq!(h.current(), &x);
        assert_eq!(names(h.forward_list(None)), ["a", "a?atlas", "b"]);
        assert_eq!(h.forward_by(2, None), step(true, search("atlas")));
        assert_eq!(h.current(), &a);
        assert_eq!(h.forward_by(1, search("atlas")), step(true, None));
        assert_eq!(h.back_by(2, None), step(true, None));
        assert_eq!(h.current(), &a);
        assert_eq!(names(h.back_list(None)), ["x"]);
        assert_eq!(names(h.forward_list(None)), ["a?atlas", "b"]);
    }

    #[test]
    fn back_from_a_search_puts_it_ahead_of_what_forward_led_to() {
        let [a, b] = ["/a", "/b"].map(|p| Location::Folder(p.into()));
        let mut h = History::new(a.clone());
        h.navigate(b.clone(), None);
        h.back(None);

        // The search was typed in `a`, with `b` still ahead. Back from it
        // is `a`, though no place is behind that.
        assert!(!h.can_back(None));
        assert_eq!(names(h.back_list(search("road").as_ref())), ["a"]);
        assert_eq!(h.back(search("road")), step(false, None));
        assert_eq!(h.current(), &a);
        assert!(!h.can_back(None));
        assert_eq!(names(h.forward_list(None)), ["a?road", "b"]);
        assert_eq!(h.forward(None), step(false, search("road")));
        assert_eq!(h.forward(search("road")), step(true, None));
        assert_eq!(h.current(), &b);
        assert_eq!(h.back(None), step(true, search("road")));

        // With other words by the time the window goes on, those are kept.
        assert_eq!(h.forward(search("map")), step(true, None));
        assert_eq!(h.back(None), step(true, search("map")));
        assert_eq!(h.current(), &a);
    }

    #[test]
    fn history_keeps_a_search_left_for_its_own_folder() {
        let a = Location::Folder("/a".into());
        let mut h = History::new(a.clone());
        // Going where the window already is does nothing, except to keep a
        // search that it ends, as showing a result in its folder does.
        assert!(!h.navigate(a.clone(), None));
        assert!(!h.can_back(None));
        assert!(!h.navigate(a.clone(), search("road")));
        assert_eq!(names(h.back_list(None)), ["a?road", "a"]);

        assert_eq!(h.back(None), step(false, search("road")));
        assert_eq!(h.forward(search("road")), step(false, None));
        assert_eq!(h.back(None), step(false, search("road")));
        assert_eq!(h.current(), &a);

        // It's going somewhere all the same, so nothing is ahead after it.
        assert_eq!(h.back(search("road")), step(false, None));
        assert!(h.can_forward(None));
        assert!(!h.navigate(a.clone(), search("map")));
        assert!(!h.can_forward(None));
        assert_eq!(h.back(None), step(false, search("map")));
        assert_eq!(h.back(search("map")), step(false, None));
        assert_eq!(h.back(None), None);
    }

    #[test]
    fn history_skips_a_step_that_would_change_nothing() {
        let [a, b] = ["/a", "/b"].map(|p| Location::Folder(p.into()));
        let mut h = History::new(a.clone());
        h.navigate(a.clone(), search("road"));
        assert_eq!(h.back(None), step(false, search("road")));
        // The search is ended with Escape, which the history doesn't hear
        // of. Forward would be `a` with no search, where the window is.
        assert!(h.can_forward(search("road").as_ref()));
        assert!(!h.can_forward(None));
        assert_eq!(h.forward(None), None);

        // The same search typed again after Back ended it is what Forward
        // would lead to, and `b` is the next place that isn't.
        h.navigate(b.clone(), None);
        assert_eq!(h.back(None), step(true, None));
        assert_eq!(h.back(search("road")), step(false, None));
        assert_eq!(names(h.forward_list(search("road").as_ref())), ["b"]);
        assert_eq!(h.forward(search("road")), step(true, None));
        assert_eq!(h.current(), &b);

        // The same search made twice in a row, as by opening a Smart Folder
        // again, is one step back and not two.
        let mut h = History::new(a.clone());
        h.navigate(b.clone(), None);
        h.navigate(b.clone(), search("road"));
        h.navigate(b.clone(), search("road"));
        assert_eq!(names(h.back_list(search("road").as_ref())), ["b", "a"]);
        assert_eq!(h.back(search("road")), step(false, None));
        assert_eq!(h.back(None), step(true, None));
        assert_eq!(h.current(), &a);
        assert_eq!(names(h.forward_list(None)), ["b", "b?road"]);
    }

    #[test]
    fn a_search_in_the_history_follows_its_folder() {
        let pairs = [(PathBuf::from("/p/old"), PathBuf::from("/p/new"))];
        let moves = Moves::new(&pairs);
        let mut h = History::new(Location::Folder("/p/old".into()));
        h.navigate(Location::Folder("/q".into()), search("atlas"));
        h.navigate(Location::Folder("/p/old/a".into()), search("road"));
        h.back(None);
        assert!(!h.follow(&moves));

        assert_eq!(
            names(h.back_list(search("road").as_ref())),
            ["q", "new?atlas", "new"]
        );
        assert_eq!(names(h.forward_list(search("road").as_ref())), ["a"]);
        assert_eq!(h.back_by(2, search("road")), step(true, search("atlas")));
        assert_eq!(h.current(), &Location::Folder("/p/new".into()));
        assert_eq!(h.forward_by(3, search("atlas")), step(true, None));
        assert_eq!(h.current(), &Location::Folder("/p/new/a".into()));
    }

    /// Check that one step at a time leads through what Back and Forward
    /// list, that every step shows something else, and that several steps at
    /// once end where single ones do.
    fn assert_steps_follow_the_lists(h: &History, live: &Option<SavedSearch>) {
        let was = Visit {
            location: h.current.clone(),
            search: live.clone(),
        };
        let parts = |h: &History| (h.current.clone(), h.back.clone(), h.forward.clone());
        for forward in [false, true] {
            let go = |h: &mut History, steps: usize, live: Option<SavedSearch>| match forward {
                true => h.forward_by(steps, live),
                false => h.back_by(steps, live),
            };
            let list: Vec<Visit> = match forward {
                true => h.forward_list(live.as_ref()).collect(),
                false => h.back_list(live.as_ref()).collect(),
            };
            let (mut stepped, mut shown, mut moved) = (h.clone(), was.clone(), false);
            for (ix, stop) in list.iter().enumerate() {
                let one = go(&mut stepped, 1, shown.search.clone()).expect("a listed step");
                let now = Visit {
                    location: stepped.current.clone(),
                    search: one.search,
                };
                assert_eq!(&now, stop);
                assert_ne!(now, shown, "a step that changed nothing");
                assert_eq!(one.moved, now.location != shown.location);
                (shown, moved) = (now, moved || one.moved);

                let mut jumped = h.clone();
                let jump = go(&mut jumped, ix + 1, live.clone());
                assert_eq!(jump, step(moved, shown.search.clone()));
                assert_eq!(parts(&jumped), parts(&stepped));
            }
            assert_eq!(go(&mut stepped, 1, shown.search.clone()), None);
            let mut too_far = h.clone();
            assert_eq!(go(&mut too_far, list.len() + 1, live.clone()), None);
            assert_eq!(parts(&too_far), parts(h), "all the steps or none");
        }

        // Forward takes back what Back did. Back takes back what Forward
        // did, in two steps when that was a search made somewhere else.
        let mut there = h.clone();
        if let Some(step) = there.back(live.clone()) {
            let again = there.forward(step.search).expect("forward after back");
            assert_eq!((&there.current, &again.search), (&was.location, live));
        }
        let mut there = h.clone();
        if let Some(step) = there.forward(live.clone()) {
            let mut behind = there.back_list(step.search.as_ref()).take(2);
            assert!(behind.any(|visit| visit == was));
        }
    }

    #[test]
    fn steps_back_and_forward_lead_where_the_lists_say() {
        /// Every history that `depth` more things done in the window lead to.
        fn explore(h: &History, live: &Option<SavedSearch>, depth: usize) {
            assert_steps_follow_the_lists(h, live);
            let Some(depth) = depth.checked_sub(1) else {
                return;
            };
            // Going somewhere, which may be where the window is already.
            for to in ["/a", "/b"] {
                let mut next = h.clone();
                next.navigate(Location::Folder(to.into()), live.clone());
                explore(&next, &None, depth);
            }
            // Typing a search and ending it with Escape, which the history
            // hears of only when the window next goes somewhere.
            for typed in [search("road"), search("map"), None] {
                if typed != *live {
                    explore(h, &typed, depth);
                }
            }
            let mut next = h.clone();
            if let Some(step) = next.back(live.clone()) {
                explore(&next, &step.search, depth);
            }
            let mut next = h.clone();
            if let Some(step) = next.forward(live.clone()) {
                explore(&next, &step.search, depth);
            }
        }
        explore(&History::new(Location::Folder("/a".into())), &None, 6);
    }

    #[test]
    fn selection_ranges() {
        let mut s = Selection::default();
        s.select_only(2);
        s.extend_to(5);
        assert_eq!(s.indices().collect::<Vec<_>>(), [2, 3, 4, 5]);
        s.extend_to(0);
        assert_eq!(s.indices().collect::<Vec<_>>(), [0, 1, 2]);
        s.toggle(1);
        assert_eq!(s.indices().collect::<Vec<_>>(), [0, 2]);
        s.move_by(10, false, 4);
        assert_eq!(s.cursor(), Some(3));
        s.move_by(-1, true, 4);
        assert_eq!(s.indices().collect::<Vec<_>>(), [2, 3]);
    }

    #[test]
    fn cycle_cursor_wraps_within_the_selection() {
        let mut s = Selection::default();
        s.select_only(1);
        s.toggle(4);
        s.toggle(6);
        s.cycle_cursor(1);
        assert_eq!(s.cursor(), Some(1), "wraps from the last to the first");
        s.cycle_cursor(-1);
        assert_eq!(s.cursor(), Some(6), "and back");
        s.cycle_cursor(-1);
        assert_eq!(s.cursor(), Some(4));
        assert_eq!(s.indices().collect::<Vec<_>>(), [1, 4, 6]);
        s.toggle(4);
        s.cycle_cursor(1);
        assert_eq!(
            s.cursor(),
            Some(6),
            "a deselected cursor counts as the first"
        );
    }

    #[test]
    fn selection_survives_resort() {
        let old: Vec<PathBuf> = ["/a", "/b", "/c"].map(PathBuf::from).into();
        let new: Vec<PathBuf> = ["/c", "/b"].map(PathBuf::from).into();
        let mut s = Selection::default();
        s.select_only(0);
        s.toggle(2);
        s.remap(&old, &new);
        assert_eq!(s.indices().collect::<Vec<_>>(), [0]);
        assert_eq!(s.cursor(), Some(0));
    }
}
