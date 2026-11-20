# Corrections to apply when CHANGELOG.md [Unreleased] is assembled

mkchangelog.py takes each entry from its commit message. Later commits made some sentences untrue, and three
sentences describe faults in things no release ever had. From the whole-branch review (docs-tests lens, confirmed),
plus two notes from earlier reviews. Apply to the assembled text, then read it once as one text.

1. caceef3 preview-scroll-per-file (Fixed). "Each file's preview now starts at its top, and a table that is wider
   than its space starts at its left edge." becomes: "Each file's preview now starts at its top (a contents search
   result's at its matching line), and a table that is wider than its space starts at its left edge."
2. 323f173 back-forward-restore-search (Changed). "A search that comes back runs again with the kind and the Nushell
   filter chosen now, in relevance order." becomes: "A search that comes back runs again with the kind and the Nushell
   filter chosen now, in relevance order; a Smart Folder's search comes back with its own kind and filter."
3. da6a5b9 strict-check-scope-relative (Fixed). "Near misses with typos then stayed in the list beside the real
   matches, so `music` also listed `basic.txt`, and they showed with **Allow typos** off too." becomes: "Matches that
   fff-search only made by leaving letters out then stayed in the list beside the real matches, so `music` also
   listed `basic.txt`." (5624239 defines a near miss otherwise, and Allow typos never shipped.)
4. 323f173, the blanket "the sidebar": going elsewhere "by ... the sidebar" keeps the search in the history, but a
   click on a Smart Folder that searches This Computer (or whose folder is gone) takes the search's place. Changelog
   and docs/manual.md:337: "...a folder or place clicked in the sidebar or the path bar...", and add to manual.md:337
   "Opening a Smart Folder that searches This Computer takes the search's place, as typing other words does."
5. 8dc26ad fuzzy-word-case (Fixed). "...and when it has quoted words, which it didn't. Quoted words still follow
   **Letter case in searches**." becomes: "A search also remembers the file you opened from it whatever the letter
   case, and when it has quoted words. Quoted words follow **Letter case in searches**."
6. 06d84e9 search-syntax-click-example (Added). "Its first row now says whether typos are allowed, as **Allow typos**
   is set; it used to read "typos allowed in Settings" either way." becomes: "Its first row says whether typos are
   allowed, as **Allow typos** is set."
7. Upstream's existing [Unreleased] entry on quoted words says unquoted words "match fuzzily across the whole path,
   so `taxexemptionc` can find a file whose folders spell it out between them". Corrected (typo-fallback-near-miss'
   reviewer): "Unquoted words still match fuzzily along the path inside the search location, or inside the folder
   when only one is searched, so `taxexemptionc` also finds `tax/exemption/coding/Handler.java`, whose folders spell
   it out in order."
8. A final check on the assembled text: it must not contain "off too", "which it didn't" or "typos allowed in
   Settings".

## From the reading of the assembled text at the branch's end (20 Nov)

9. 52e7de4 stale-previews-and-cancel (Fixed). The last clause, "and a picture still shows as it was first drawn
   until fffinder is restarted", goes: 1acaff3 (thumbnail-memory) made it untrue, and both records ask for it.
10. 5624239 typo-fallback-near-miss (Changed). The sentence "A word with an accent or in another alphabet is held to
    this only where a path is all in A to Z, digits, and symbols: ... as loosely as before." goes: 362de8f
    (accented-names-search) took that exception out (`may_be_spelled_apart` is gone), and its own entry says that a
    misspelled word with an accent now lists only near misses.

## From the check of the assembled text (review/changelog-check.md): a reader's findings that a skeptic confirmed

The reader was given the text of 46 entries at 362de8f and the last change as a patch; ten of its eleven findings
were confirmed, one refuted (a renamed item does stay selected). Two of the ten are 9 and 10 above, and one was about
the patch's wording, which the commit that landed (bf654d0) already had right.

11. 52e7de4 stale-previews-and-cancel (Fixed). After "The old preview stays until the new one is ready, and text
    stays where you were reading it." comes: "An image is the exception: its place is empty while it is read again,
    and a preview drawn from the system's thumbnail shows the icon meanwhile." (Since 1acaff3 an image is read again;
    `large_thumbnail` shows its fallback, the icon, while a thumbnail loads.)
12. 362de8f accented-names-search (Fixed). "A misspelled word with an accent or in another alphabet now lists only
    near misses, as other words do, and what a search opened is remembered for such names too." becomes: "What a
    search opened is remembered for names with accented letters too." (The looser rule for such words came and went
    inside this section, and since bf654d0 no word lists only near misses.)
13. 362de8f (Fixed). "...and never with **Allow typos** off, in a quoted word, or for an app." becomes: "...and never
    found an app whose name is stored the other way." (No release had the switch or quoted words.)
14. e2e1672 new-item-in-hidden-row (Fixed). "it read "0 items" under headers that counted more" becomes: "it read
    "7 items" under headers that counted sixteen". (A release never drew headers over no rows.)
15. e746db6 quick-look-window-mixup (Fixed). "Pressed before the results were in, Return did nothing." goes: a Return
    that waits for the results is new in this section (17a460e).
16. 4efa254 fix-rename-press-selection (Fixed). One sentence added at the end: "A press on an item inside the folder
    that is being renamed, in the list with that folder open in place or in the column to its right, still leaves the
    folder selected, not the item." (Pinned by a_press_inside_a_folder_that_ends_its_rename_leaves_the_folder_selected.)
17. 153eb8f live-search-results (Changed). "Recents is read again when a file it would show is added, removed,
    renamed, or written to." gets: ", though not for writes to the file it has first already, which keeps the size
    and date it showed until Recents is next read."

Not changed, noted by the skeptic while tracing, for the wording fix-up that is saved for later
(fix-wording-after-later-changes): docs/manual.md:428 excepts only an image from "the preview that was showing stays
until the new one is ready", and a preview drawn from a thumbnail shows its icon meanwhile; docs/manual.md:513 (the
Rename… row) does not name the case of an item inside the folder being renamed; in the stale-previews entry "another
size or date" is the date alone for an image.

What the reader did not check against the code: the entries no later commit touches were checked for labels, keys
and tests, not traced again; the "before" halves of some entries rest on the commit messages; anything that needs
macOS or Windows; figures that were measured ("about 100 of 500", "about 48 MB").
