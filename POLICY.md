# Registry policy

The engine is generic and cannot police what people build. This registry is
curated, and that distinction is the project's whole defence. See
[the full plugin policy](https://github.com/idlewarden/idlewarden/blob/main/PLUGIN_POLICY.md)
in the main repository, this file exists so the rules travel with the registry,
and the main repository is authoritative if they ever diverge.

## Accepted

Single-player and offline games. Idle, incremental, clicker and management
games. Games whose terms explicitly permit automation. Games with an official,
documented automation or modding interface.

## Refused, without exception

* Any game with a competitive or ranked multiplayer mode.
* Anything circumventing, disabling or probing an anti-cheat or protection
  mechanism.
* Plugins requiring memory reading, code injection or driver installation from
  the Core itself. There is no mechanism for these and there will not be one.
* Games whose terms forbid automation.
* Anything aimed at advantage over other human players, or at acquiring
  tradeable goods for sale.

## Mods are indexed too, under a different promise

A **mod** runs inside the game process and exposes a bridge endpoint
(ADR-0014). It is a binary, so nobody is going to read it, and pretending
otherwise would empty the `verified` badge of its meaning.

So a mod entry is not verified by review. It is verified by **provenance**:

- `source.repository` is public and is where the artefact comes from,
- every version carries an `attestation.repository`, and CI runs
  `gh attestation verify` against the downloaded artefact,
- the declared `sha256` matches what CI downloads.

`verified` on a mod claims exactly one thing: *this binary was built by a public
CI run from that public repository*. It is not a code review and must never be
presented as one.

The registry never hosts a mod binary, exactly as it never hosts a plugin
package.

### Mod review checklist

- [ ] The source repository is public and actually contains the mod.
- [ ] The attestation repository matches the source repository.
- [ ] `plugin` names a real entry under `plugins/`, and that entry declares
      `bridge:<name>` matching this mod's `bridge.name`.
- [ ] `loader` is accurate; `manual` is only for a documented file drop.
- [ ] `game_versions` is populated. A mod is tied to a game build, not a title.
- [ ] The game is one the plugin policy already accepts. A bridge does not buy
      an exemption from anything above.

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
