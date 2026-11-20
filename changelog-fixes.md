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
