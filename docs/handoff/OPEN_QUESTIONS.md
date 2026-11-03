# Open questions — pythrust
Claude Code: append here instead of stopping when a question isn't blocking.
Format: `- [ ] Q<n> (<date>, blocking? y/n): <question> — interim choice: <what you did>`
Resolved items are checked and point to the D#/QA entry that answers them.

- [ ] Q1 (2026-11-03, n): Does `docs/handoff.md` stay as the historical Step-0/threads doc now that `docs/handoff/` exists, or merge into the pack? — interim: keep both; `CLAUDE.md` read-first still lists `docs/handoff.md`.
- [ ] Q2 (2026-11-03, n): Do `docs/grammar.md` (Rust bindings: "cargo with Rust 1.88 or later"), `docs/handoff.md`/native.md and `CLAUDE.md` text need updating to the 1.97 floor (D34)? PR #99 owns the code change. — interim: leave docs; repo wins over the pack.
- [ ] Q3 (2026-11-03, n): Shared-list conflicts: "pending Nano's pick" between per-function coarse sharing and a call-site check (roadmap M6c rule 4 coarseness). — interim: coarse rule stands.
- [ ] Q4 (2026-11-03, n): "Ownership checks in tempered programs": a lowering gap in a program containing `temper def` should be a named error. Not started; Nano decides on its own (roadmap). — interim: current behaviour.
- [ ] Q5 (2026-11-03, n): Relax `for k in d: d[k] = ...` in tempered code (unmodified loop variable)? Revisit only if fclone-lite or the corpus shows the idiom matters (roadmap). — interim: error stays.
- [ ] Q6 (2026-11-03, n): File-type `mut` rule gaps (copies, calls) once a file type is spellable (roadmap M6a follow-up). — interim: syntactic rule only.
- [ ] Q7 (2026-11-03, n): `shared[T]` and channels: build when a program needs one (parked). Also `abi::Raise` and native `run` for interop deferred until an app needs them. — interim: not built.
- [ ] Q8 (2026-11-03, n): M6e Python bridge waits for a second app; which app? — interim: none.
- [ ] Q9 (2026-11-03, n): Lock entries keyed by name+version, not source: same crate/version from two registries in two bindings is refused (deferred by Nano 2026-11-01). — interim: named error.
- [ ] Q10 (2026-11-03, n): Harness strips `mut`/`owned` as plain text (backlog from #46 review, remind-me mem_72837b9a). Status unknown. — interim: unchanged.
- [ ] Q11 (2026-11-03, n): Open PR #99 items still pending Nano: `pythrust lock --check` CI step, scratch-guard PR, `/tmp/pythrust-native-*` cleanup — are they done? — interim: assume in flight.
- [ ] Q12 (2026-11-03, n): Design chat link for this project is unknown; paste it into SPEC.md when found.
