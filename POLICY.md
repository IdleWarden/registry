# Registry policy

The engine is generic and cannot police what people build. This registry is
curated, and that distinction is the project's whole defence. See
[the full plugin policy](https://github.com/idlewarden/idlewarden/blob/main/PLUGIN_POLICY.md)
in the main repository — this file exists so the rules travel with the registry,
and the main repository is authoritative if they ever diverge.

## Accepted

Single-player and offline games. Idle, incremental, clicker and management
games. Games whose terms explicitly permit automation. Games with an official,
documented automation or modding interface.

## Refused, without exception

* Any game with a competitive or ranked multiplayer mode.
* Anything circumventing, disabling or probing an anti-cheat or protection
  mechanism.
* Plugins requiring memory reading, code injection or driver installation — the
  plugin model provides no mechanism for these.
* Games whose terms forbid automation.
* Anything aimed at advantage over other human players, or at acquiring
  tradeable goods for sale.

## Review checklist

- [ ] `multiplayer` is `false` **and** the reviewer verified the claim.
- [ ] The game matcher is specific enough not to capture unrelated windows.
- [ ] Requested capabilities are justified by the description. `net` gets extra
      scrutiny; `fs.read` must name a real directory.
- [ ] The release URL is immutable (a tag, not a branch) and the checksum
      matches.
- [ ] Template assets are small UI fragments, not wholesale game artwork.
- [ ] The declared licence is present in the package.

## Grey areas

Co-op-only multiplayer, single-player with online leaderboards, terms that are
silent rather than permissive. Open a discussion **before** writing the plugin.
Decisions are made in public and recorded here.
