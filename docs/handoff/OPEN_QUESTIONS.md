# Open questions — pythrust
Claude Code: append here instead of stopping when a question isn't blocking.
Format: `- [ ] Q<n> (<date>, blocking? y/n): <question> — interim choice: <what you did>`
Resolved items are checked and point to the D#/QA entry that answers them.

- [ ] Q1 (2026-11-03, n): Does `docs/handoff.md` stay as the historical Step-0/threads doc now that `docs/handoff/` exists, or merge into the pack? — interim: keep both; `CLAUDE.md` read-first still lists `docs/handoff.md`.
- [x] Q2 (2026-11-03): 1.97 floor landed in #99 and the docs follow (grammar.md, rust-interop.md); see D34.
- [ ] Q3 (2026-11-03, n): Shared-list conflicts: "pending Nano's pick" between per-function coarse sharing and a call-site check (roadmap M6c rule 4 coarseness). — interim: coarse rule stands.
- [ ] Q4 (2026-11-03, n): "Ownership checks in tempered programs": a lowering gap in a program containing `temper def` should be a named error. Not started; Nano decides on its own (roadmap). — interim: current behaviour.
- [ ] Q5 (2026-11-03, n): Relax `for k in d: d[k] = ...` in tempered code (unmodified loop variable)? Revisit only if fclone-lite or the corpus shows the idiom matters (roadmap). — interim: error stays.
- [ ] Q6 (2026-11-03, n): File-type `mut` rule gaps (copies, calls) once a file type is spellable (roadmap M6a follow-up). — interim: syntactic rule only.
- [ ] Q7 (2026-11-03, n): `shared[T]` and channels: build when a program needs one (parked). Also `abi::Raise` and native `run` for interop deferred until an app needs them. — interim: not built.
- [ ] Q8 (2026-11-03, n): M6e Python bridge waits for a second app; which app? — interim: none.
- [ ] Q9 (2026-11-03, n): Lock entries keyed by name+version, not source: same crate/version from two registries in two bindings is refused (deferred by Nano 2026-11-01). — interim: named error.
- [x] Q10 (2026-11-03): the harness's quoted-string `mut`/`owned` stripping was fixed in #78 ("loosen keeps mut/owned inside string literals").
- [x] Q11 (2026-11-03): #99 merged with `pythrust lock --check` (see roadmap M6d); #101 moved conformance scratch dirs onto the guard.
- [ ] Q12 (2026-11-03, n): Design chat link for this project is unknown; paste it into SPEC.md when found.
- [x] Q13 (2026-11-03, n): A thread asks to silently widen `total = 0` / `total += 1.5` to float for fclone-lite. Reverses D3 [stated]; escalated to Nano. → D3 (Nano chose a, keep D3, 2026-11-03T18:55Z; no new row).
- [ ] Q14 (2026-11-03, y: PR #102 review): Contract for a `PYTHRUST_RT_SRC` override's `Cargo.toml`. Logged as D39 [default], re-tiered to escalation by D40: a developer who sets the override sees which manifests are accepted without reading the code, and reviewer Dottius asked for the owner's acceptance. Options: a) D39 as at PR #102 head: a standalone manifest, or the repo runtime's own `key.workspace = true` forms; anything else, including other inheritance spellings (e76fbe4), is a named error; `package.metadata` is skipped by both the inheritance and the path-dependency checks (ee49e50, 1bd6113). b) A standalone manifest only, so `PYTHRUST_RT_SRC=crates/pythrust-rt` fails. c) A general workspace-inheritance transform, hardened as reviews find gaps. An unkeyed override (PR 3 behaviour) is not offered, because it would reverse D32 choice 5. — interim: PR #102 head behaviour as it stands; nothing widened or narrowed.
