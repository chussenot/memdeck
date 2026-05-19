# memdeck MCP server

`bin/memdeck-mcp` is a [Model Context Protocol](https://modelcontextprotocol.io/) server that gives an LLM agent (e.g. Claude Code) typed access to MemDeck's ABC parser and audio engine.

Pure C, single binary, stdio transport, JSON-RPC 2.0. Vendors [yyjson](https://github.com/ibireme/yyjson) (MIT) for JSON I/O. Links the existing C engine sources directly — no separate library, no Python or Node dependency, no IPC beyond stdio.

## Build

```sh
make mcp
```

Output: `bin/memdeck-mcp`. Requires nothing the rest of the project doesn't already need (cc, libm).

## Register with Claude Code

```sh
claude mcp add --transport stdio memdeck -- /absolute/path/to/memdeck/bin/memdeck-mcp
```

The server must be launched from the project root so the demo-enumeration tool can find `data/music/`. If you register it from elsewhere, prefix with `sh -c 'cd /path/to/memdeck && exec bin/memdeck-mcp'` or similar.

Verify with `claude mcp list` — `memdeck` should appear with status `Connected`.

## Tools

| Tool | Purpose |
| --- | --- |
| `memdeck_inspect_abc` | Parse a `.abc` through the engine. Returns title, key, tempo, swing, all instruments (with ladder targets), all FX buses (with ladder settings), patterns, arrangement, and voices with note counts. Use this instead of re-reading the raw file when you need a structured view. |
| `memdeck_render_stats` | Render a song through the audio engine and return deterministic stats: duration, sample count, hex checksum, clipping count, peak, min/max sample. Does not write audio to disk. |
| `memdeck_validate_abc` | Validate an ABC text blob passed as a string (no disk write). On success returns the headline counts plus `expected_steps` (sum of pattern lengths in the arrangement) and a `voices[]` block with per-voice `{note_count, expected_steps, aligned, delta_steps}` — `all_voices_aligned: false` flags the common bug where a voice has too few bars and the engine silently truncates. Use in a tight draft-and-validate loop. |
| `memdeck_pitch_resolve` | Resolve ABC pitch tokens (`A,,`, `c'`, `^F`, …) to MIDI number, Hz, scientific name (`A4`, `C#5`), and octave. Pass either a single `token` or an array `tokens`. Pure math — no engine load. Removes the octave-counting friction that comes with `A=69 / a=81 / A,,=45`. |
| `memdeck_chord_tones` | Spell a chord at a chosen register and return ready-to-paste ABC. Input: `chord` (e.g. `Am`, `F#m`, `Bb`, `G7`, `Csus4`, `Bdim`) + `register` (`bass` / `stab` / `arp` / `high`) + `steps_per_bar`. Output: tones as `{abc, midi, hz, scientific}` plus a one-bar `arpeggio_one_bar` string sized to the requested step count. Lets you bypass the mechanical work of spelling chord stabs/arps in the right octave. |
| `memdeck_list_demos` | Enumerate `data/music/*.abc`, skipping `menu*` UI sounds. Returns `[{key, path, title}]`, sorted. |
| `memdeck_directive_help` | Reference card for the `%%` directives the parser understands and their parameter ranges. Use this instead of grepping headers when writing new songs. |
| `memdeck_duration_calc` | Pure math: given `bpm`, `l_denom`, `total_steps`, returns the resulting duration in seconds plus whether it fits the engine's timeline cap. Plan a song's length before writing 8 voices' worth of bars. |
| `memdeck_engine_caps` | All the `SEQ_MAX_*` / `ABC_MAX_*` / `SAMPLE_RATE_ABC` constants plus the ladder param ranges. Saves you a `grep -n MAX src/memdeck.h`. |

## Smoke test

```sh
make mcp-smoke
```

Pipes `initialize`, `tools/list`, and `tools/call memdeck_engine_caps` through stdio and prints the JSON-RPC responses. Useful when verifying the binary after a change.

## Adding a tool

Tools are defined twice in `tools/memdeck_mcp.c`:

1. The schema is registered in `handle_tools_list` via `add_tool_def(...)` + `add_prop(...)` calls.
2. The handler is implemented as `static void tool_<name>(yyjson_val *id, yyjson_val *args)` and dispatched in `handle_tools_call`.

Both sides must stay in sync — the `name` string is the lookup key. When passing strings into the response doc, use the `*_strcpy` yyjson variants for any value whose lifetime isn't `'static`: `yyjson_mut_obj_add_str` stores the pointer by reference, so loop-local stack buffers must be copied.

## Why C instead of a Rust/Node/Python server

- the existing engine is C99 with integer fixed-point math; the MCP layer links it directly with no FFI shim
- one binary, no language runtime to install on the agent's host
- yyjson handles the JSON-RPC envelope at ~1 GB/s without RTTI overhead
- the tool surface is small (7 tools) and stable; no need for a heavyweight framework

See `docs/adr-0001-gui-direct-audio-playback.md` for the parallel decision in the GUI playback layer (also kept in the C engine path).
