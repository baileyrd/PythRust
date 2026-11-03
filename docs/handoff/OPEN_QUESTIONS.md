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
