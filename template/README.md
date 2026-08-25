# IdleWarden plugin template

Copy this folder into your own repository, rename what needs renaming, and you
have a working plugin. **No compilation, no Rust, no linking** — a plugin is
data, which is exactly why the plugin API can be stable and why hot reload is
nearly free (ADR-0001, ADR-0012).

```
your-plugin/
├── plugin.json        ← contract + metadata. Changing capabilities needs a real reload.
├── rules.json         ← behaviour. Hot-reloadable.
├── profiles/
│   └── default.json   ← shipped defaults. The user's overrides live elsewhere and win.
├── assets/            ← templates, anchors, icon, screenshots. Hot-reloadable.
├── .ferrflow          ← release automation. calver-short.
└── CHANGELOG.md       ← generated. Do not edit.
```

## The five-minute version

1. Copy this folder, set `id` (reverse-DNS, immutable) and the `game` matcher.
2. Launch the game **borderless windowed**, launch IdleWarden, open the region
   editor, draw one ROI.
3. Save. The running agent picks it up — no restart of the app, and never a
   restart of the game.
4. Iterate on `rules.json` with the game running and the match scores live.

That loop is the whole point. If you find yourself restarting anything, tell us:
that is a bug.

## What hot-reloads, and what does not

| Change | Live? |
|---|---|
| `rules.json` — ROIs, thresholds, commands, tree | **yes** |
| `assets/*.png` — templates and anchors | **yes** |
| `profiles/*.json` — limits and tuning | **yes** |
| `presentation` in `plugin.json` | **yes** |
| `signals` / `intents` schema | yes, with a tree reset |
| **`capabilities`** | **no — explicit reload with user consent** |
| **`api_version`** | **no — full reload** |

Capabilities are excluded on purpose. A plugin that could grant itself new
powers by writing to a file the host is already watching would be a privilege
escalation dressed as a convenience.

## Versioning

`version` is **calver-short**: `YY.M.PATCH`, e.g. `26.8.1`. FerrFlow manages it
from your conventional commits — never edit it by hand.

Why calendar and not semantic: a plugin version tracks *the game's* patches.
There is no meaningful "breaking change" in a plugin — either it still matches
the current build or it does not, and that is what `game.tested_versions` is
for. Semver here would be decoration pretending to be a contract.

`api_version` stays **semver**, because that one really is a contract.

Handy: `26.8.1` also parses and sorts as valid semver, so update checks work
with no special case.

## Rights, and why the field is required

Plugins ship crops of copyrighted game UI. The exposure is small but not zero,
so `rights.asset_provenance` is mandatory — the question gets answered once, by
you, instead of never. In order of preference:

* **`hashes-only`** — ship perceptual descriptors, no imagery. Always best when
  it works.
* **`user-generated`** — templates are captured on first run from the user's own
  installation. Almost as good, and more robust across game versions.
* **`bundled-crops`** — small UI fragments ship in the package. Keep them
  minimal. Never wholesale artwork.

## Declare what you tested

`game.tested_versions` drives the compatibility banner. An empty list is
displayed as "never verified" — silence is not a claim of support. Marking a
version `broken` makes the host refuse to start on it, which is far kinder than
letting it flail against a UI that moved.

## Publishing

1. Tag a release in your repository (FerrFlow does this).
2. `sha256sum plugin.zip`
3. Open a pull request on `IdleWarden/registry` adding
   `plugins/<your.id>.json` — see `../plugins/EXAMPLE.json.template`.

We index plugins; we do not host them. Your code stays yours.

## Before you write anything

Read [`../POLICY.md`](../POLICY.md). Games with any competitive or ranked
multiplayer mode are refused, and `multiplayer: true` is an automatic decline.
