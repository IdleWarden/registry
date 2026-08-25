# IdleWarden plugin registry

This is the whole "marketplace": a git repository holding an index, a JSON
Schema, and a validation workflow. There is **no backend, no accounts, no
server**.

Submission is a pull request, so moderation is the review queue, author identity
is a GitHub account, version history is git history, and binary hosting is
GitHub Releases. Infrastructure cost: zero.

A real marketplace — search, ratings, download counts — earns its keep somewhere
north of fifty plugins and actual traffic. Until then, building one would mean
months of CRUD instead of making the vision pipeline work on one game.

## We index plugins; we do not host them

Third-party plugins live in **their authors' own repositories**. You publish a
release, then open a pull request here adding a file under `plugins/` pointing
at it with a checksum. We carry neither your code nor your maintenance.

## Start from the template

[`template/`](template/) is a complete, working plugin — manifest, rules,
profile, release config. Copy it into your own repository and edit. No
compilation, no Rust, no linking: a plugin is data, which is what makes the API
stable and live reload nearly free.

## Submitting

1. Read [`POLICY.md`](POLICY.md). Multiplayer titles are refused, and
   `multiplayer: true` in a manifest is an automatic decline.
2. Publish a release of your plugin package (`.zip`) in your own repository.
3. Compute its checksum: `sha256sum my-plugin-1.0.0.zip`
4. Add `plugins/<your.plugin.id>.json` following
   [`schema/plugin-entry.schema.json`](schema/plugin-entry.schema.json).
5. Open a pull request. CI validates the schema, the id format, and that the
   URL and checksum resolve.

## Trust levels

| Level | Meaning | Auto-update |
|-------|---------|-------------|
| `official` | Built and signed by the IdleWarden project | yes |
| `verified` | Reviewed here, signed by a registered author key | yes |
| `unverified` | Installed by URL or local file, never listed here | **no** |

## Tooling

Everything a submission is checked against lives in `crates/registry-tools`, a
small Rust binary CI runs on every pull request. Schema conformance stays in
`schema/`, which the tool loads at runtime, so the published schema remains the
single source of truth for plugin authors; the tool adds the rules a schema
cannot express.

```bash
cargo run -p registry-tools -- validate                  # offline: schema, id, policy, URL shape
cargo run -p registry-tools -- validate --check-assets   # also downloads each asset and verifies its sha256
cargo run -p registry-tools -- build-index               # regenerate index.json from plugins/
cargo run -p registry-tools -- build-index --check       # fail if index.json is out of date
```

`index.json` is generated, never hand-edited, and its `generated_at` field stays
`null` on purpose: a build timestamp would make every regeneration a diff, and
git already records when the file changed.

## Layout

```
registry/
├── plugins/<id>.json   ← the source of truth, one file per plugin
├── index.json          ← generated from plugins/, published via Pages
├── schema/             ← JSON Schema for entries and for plugin manifests
├── template/           ← copy this to start a plugin
├── crates/             ← the tooling that validates entries and builds the index
└── POLICY.md
```
