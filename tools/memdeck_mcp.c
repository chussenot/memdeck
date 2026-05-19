/*
 * memdeck_mcp - Model Context Protocol server for MemDeck.
 *
 * Speaks JSON-RPC 2.0 over stdio (newline-delimited). Exposes a handful
 * of tools focused on ABC songwriting ergonomics: parse and inspect a
 * song, render render-only stats, validate an ABC text blob without
 * touching disk, enumerate the showcase demos, get the directive
 * vocabulary, compute timing from BPM, and report engine constants.
 *
 * Registered with Claude Code via:
 *   claude mcp add --transport stdio memdeck -- /path/to/bin/memdeck-mcp
 */

#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "yyjson.h"

#include "memdeck.h"
#include "audio_engine.h"
#include "audio_seq.h"

#define MCP_PROTOCOL_VERSION "2024-11-05"
#define SERVER_NAME          "memdeck"
#define SERVER_VERSION       "0.1.0"

#define ERR_PARSE       -32700
#define ERR_INVALID_REQ -32600
#define ERR_METHOD      -32601
#define ERR_PARAMS      -32602
#define ERR_INTERNAL    -32603

/* ============================================================
 *   JSON / protocol helpers
 * ============================================================ */

static const char *waveform_name(int w)
{
    switch (w) {
        case 0: return "square";
        case 1: return "pulse";
        case 2: return "triangle";
        case 3: return "noise";
        default: return "unknown";
    }
}

static void write_envelope(yyjson_val *id, yyjson_mut_doc *doc,
                           yyjson_mut_val *body, int is_error)
{
    yyjson_mut_val *root = yyjson_mut_obj(doc);
    yyjson_mut_obj_add_str(doc, root, "jsonrpc", "2.0");
    if (id && !yyjson_is_null(id)) {
        yyjson_mut_val *id_mut = yyjson_val_mut_copy(doc, id);
        yyjson_mut_obj_add_val(doc, root, "id", id_mut);
    } else {
        yyjson_mut_obj_add_null(doc, root, "id");
    }
    yyjson_mut_obj_add_val(doc, root, is_error ? "error" : "result", body);
    yyjson_mut_doc_set_root(doc, root);

    size_t len = 0;
    char *json = yyjson_mut_write(doc, 0, &len);
    if (json) {
        fwrite(json, 1, len, stdout);
        fputc('\n', stdout);
        fflush(stdout);
        free(json);
    }
}

static void write_jsonrpc_error(yyjson_val *id, int code, const char *msg)
{
    yyjson_mut_doc *doc = yyjson_mut_doc_new(NULL);
    yyjson_mut_val *err = yyjson_mut_obj(doc);
    yyjson_mut_obj_add_int(doc, err, "code", code);
    yyjson_mut_obj_add_str(doc, err, "message", msg);
    write_envelope(id, doc, err, 1);
    yyjson_mut_doc_free(doc);
}

static void send_tool_payload(yyjson_val *id, yyjson_mut_doc *payload_doc,
                              yyjson_mut_val *payload, int is_error)
{
    /* Serialize the payload as the text of a content block. */
    size_t plen = 0;
    char *payload_str = yyjson_mut_val_write(payload, YYJSON_WRITE_PRETTY_TWO_SPACES, &plen);
    if (!payload_str) {
        yyjson_mut_doc_free(payload_doc);
        write_jsonrpc_error(id, ERR_INTERNAL, "failed to serialize tool payload");
        return;
    }

    yyjson_mut_doc *doc = yyjson_mut_doc_new(NULL);
    yyjson_mut_val *block = yyjson_mut_obj(doc);
    yyjson_mut_obj_add_str(doc, block, "type", "text");
    yyjson_mut_obj_add_strn(doc, block, "text", payload_str, plen);

    yyjson_mut_val *content = yyjson_mut_arr(doc);
    yyjson_mut_arr_append(content, block);

    yyjson_mut_val *result = yyjson_mut_obj(doc);
    yyjson_mut_obj_add_val(doc, result, "content", content);
    yyjson_mut_obj_add_bool(doc, result, "isError", is_error ? true : false);

    write_envelope(id, doc, result, 0);
    free(payload_str);
    yyjson_mut_doc_free(doc);
    yyjson_mut_doc_free(payload_doc);
}

static void send_tool_text_error(yyjson_val *id, const char *msg)
{
    yyjson_mut_doc *doc = yyjson_mut_doc_new(NULL);
    yyjson_mut_val *payload = yyjson_mut_obj(doc);
    yyjson_mut_obj_add_bool(doc, payload, "ok", false);
    yyjson_mut_obj_add_str(doc, payload, "error", msg);
    send_tool_payload(id, doc, payload, 1);
}

/* ============================================================
 *   Schema builder helpers
 * ============================================================ */

static yyjson_mut_val *
add_tool_def(yyjson_mut_doc *doc, yyjson_mut_val *arr,
             const char *name, const char *description)
{
    yyjson_mut_val *tool = yyjson_mut_obj(doc);
    yyjson_mut_obj_add_str(doc, tool, "name", name);
    yyjson_mut_obj_add_str(doc, tool, "description", description);

    yyjson_mut_val *schema = yyjson_mut_obj(doc);
    yyjson_mut_obj_add_str(doc, schema, "type", "object");
    yyjson_mut_val *props = yyjson_mut_obj(doc);
    yyjson_mut_obj_add_val(doc, schema, "properties", props);
    yyjson_mut_val *required = yyjson_mut_arr(doc);
    yyjson_mut_obj_add_val(doc, schema, "required", required);

    yyjson_mut_obj_add_val(doc, tool, "inputSchema", schema);
    yyjson_mut_arr_append(arr, tool);
    /* return the properties object so caller can add fields */
    return tool;
}

static void
add_prop(yyjson_mut_doc *doc, yyjson_mut_val *tool,
         const char *name, const char *type, const char *desc,
         int required)
{
    yyjson_mut_val *schema = yyjson_mut_obj_get(tool, "inputSchema");
    yyjson_mut_val *props = yyjson_mut_obj_get(schema, "properties");
    yyjson_mut_val *req_arr = yyjson_mut_obj_get(schema, "required");

    yyjson_mut_val *p = yyjson_mut_obj(doc);
    yyjson_mut_obj_add_str(doc, p, "type", type);
    yyjson_mut_obj_add_str(doc, p, "description", desc);
    yyjson_mut_obj_add_val(doc, props, name, p);

    if (required)
        yyjson_mut_arr_add_str(doc, req_arr, name);
}

/* ============================================================
 *   tools/list
 * ============================================================ */

static void handle_tools_list(yyjson_val *id)
{
    yyjson_mut_doc *doc = yyjson_mut_doc_new(NULL);
    yyjson_mut_val *result = yyjson_mut_obj(doc);
    yyjson_mut_val *tools = yyjson_mut_arr(doc);

    yyjson_mut_val *t;

    t = add_tool_def(doc, tools, "memdeck_inspect_abc",
        "Parse an ABC song through the MemDeck engine and return its full "
        "structure: title, key, tempo, instruments (incl. ladder targets), "
        "FX buses (incl. ladder settings), voices (with note counts), "
        "patterns, and arrangement. Use this to understand an existing "
        "song's layout without re-reading the raw file.");
    add_prop(doc, t, "path", "string",
             "Path to a .abc file. Absolute or relative to the project root.", 1);

    t = add_tool_def(doc, tools, "memdeck_render_stats",
        "Render an ABC song through the audio engine and return render "
        "stats: duration, sample_count, checksum (hex), clipping_count, "
        "peak, min/max sample. Does not write audio to disk.");
    add_prop(doc, t, "path", "string", "Path to a .abc file.", 1);

    t = add_tool_def(doc, tools, "memdeck_validate_abc",
        "Validate an ABC text blob (passed as a string, no disk write). "
        "Returns {ok, voice_count?, pattern_count?, arrangement_length?, "
        "error?}. Use this in a tight write-validate loop while drafting.");
    add_prop(doc, t, "content", "string", "Raw ABC source text.", 1);

    t = add_tool_def(doc, tools, "memdeck_list_demos",
        "List all showcase demos in data/music/ with their parsed titles. "
        "Files whose stem starts with 'menu' are skipped (UI sounds).");

    t = add_tool_def(doc, tools, "memdeck_directive_help",
        "Return a reference card for the %% directives the parser "
        "understands (%%instrument, %%effect, %%pattern, %%arrangement, "
        "%%swing) and their parameter ranges. Use this instead of "
        "grepping headers when writing new songs.");

    t = add_tool_def(doc, tools, "memdeck_duration_calc",
        "Compute the duration in seconds of a song given its timing "
        "parameters. Returns {duration_sec, total_steps, steps_per_sec}.");
    add_prop(doc, t, "bpm", "integer", "Tempo in BPM (e.g. 100, 140).", 1);
    add_prop(doc, t, "l_denom", "integer",
             "Default-note denominator from L: directive. L:1/8 -> 8, "
             "L:1/16 -> 16.", 1);
    add_prop(doc, t, "total_steps", "integer",
             "Sum of pattern lengths across the arrangement (e.g. 16 "
             "patterns * 64 = 1024).", 1);

    t = add_tool_def(doc, tools, "memdeck_engine_caps",
        "Return engine constants: SEQ_MAX_TRACKS, SEQ_MAX_PATTERNS, "
        "ABC_MAX_VOICES, ABC_MAX_NOTES, ABC_MAX_FX_BUSES, "
        "SEQ_MAX_TIMELINE_STEPS, SAMPLE_RATE_ABC, and the ladder param "
        "ranges.");

    yyjson_mut_obj_add_val(doc, result, "tools", tools);
    write_envelope(id, doc, result, 0);
    yyjson_mut_doc_free(doc);
}

/* ============================================================
 *   tool: memdeck_inspect_abc
 * ============================================================ */

static void tool_inspect_abc(yyjson_val *id, yyjson_val *args)
{
    const char *path = yyjson_get_str(yyjson_obj_get(args, "path"));
    if (!path) {
        write_jsonrpc_error(id, ERR_PARAMS, "missing 'path' argument");
        return;
    }

    AbcMusic music;
    if (abc_load(path, &music) != 0) {
        send_tool_text_error(id, "failed to parse ABC file");
        return;
    }

    yyjson_mut_doc *doc = yyjson_mut_doc_new(NULL);
    yyjson_mut_val *root = yyjson_mut_obj(doc);
    yyjson_mut_obj_add_bool(doc, root, "ok", true);
    yyjson_mut_obj_add_str(doc, root, "path", path);
    yyjson_mut_obj_add_str(doc, root, "title", music.title);
    yyjson_mut_obj_add_int(doc, root, "bpm", music.bpm);
    yyjson_mut_obj_add_int(doc, root, "step_ms", music.step_ms);
    yyjson_mut_obj_add_int(doc, root, "swing_pct", music.swing_pct);
    yyjson_mut_obj_add_int(doc, root, "voice_count", music.voice_count);
    yyjson_mut_obj_add_int(doc, root, "instrument_count", music.instrument_count);
    yyjson_mut_obj_add_int(doc, root, "pattern_count", music.pattern_count);
    yyjson_mut_obj_add_int(doc, root, "arrangement_length", music.arrangement_length);
    yyjson_mut_obj_add_int(doc, root, "fx_bus_count", music.fx_bus_count);

    /* instruments */
    yyjson_mut_val *insts = yyjson_mut_arr(doc);
    for (int i = 0; i < music.instrument_count; i++) {
        const AbcInstrument *ins = &music.instruments[i];
        yyjson_mut_val *o = yyjson_mut_obj(doc);
        yyjson_mut_obj_add_str(doc, o, "name", ins->name);
        yyjson_mut_obj_add_str(doc, o, "preset", ins->preset);
        yyjson_mut_obj_add_str(doc, o, "waveform", waveform_name(ins->waveform));
        yyjson_mut_obj_add_int(doc, o, "amplitude", ins->amplitude);
        yyjson_mut_obj_add_int(doc, o, "duty_cycle", ins->duty_cycle);
        yyjson_mut_obj_add_int(doc, o, "attack_ms", ins->attack_ms);
        yyjson_mut_obj_add_int(doc, o, "decay_ms", ins->decay_ms);
        yyjson_mut_obj_add_int(doc, o, "sustain_level", ins->sustain_level);
        yyjson_mut_obj_add_int(doc, o, "release_ms", ins->release_ms);
        yyjson_mut_obj_add_int(doc, o, "gate_percent", ins->gate_percent);
        yyjson_mut_obj_add_int(doc, o, "vibrato_cents", ins->vibrato_cents);
        yyjson_mut_obj_add_int(doc, o, "glide_ms", ins->glide_ms);
        yyjson_mut_obj_add_int(doc, o, "fx_bus", ins->fx_bus);
        yyjson_mut_arr_append(insts, o);
    }
    yyjson_mut_obj_add_val(doc, root, "instruments", insts);

    /* fx buses */
    yyjson_mut_val *buses = yyjson_mut_arr(doc);
    for (int i = 0; i < music.fx_bus_count; i++) {
        const AbcFxBus *b = &music.fx_buses[i];
        yyjson_mut_val *o = yyjson_mut_obj(doc);
        yyjson_mut_obj_add_int(doc, o, "index", i);
        yyjson_mut_obj_add_bool(doc, o, "enabled", b->enabled ? true : false);
        yyjson_mut_obj_add_int(doc, o, "delay_steps", b->delay_steps);
        yyjson_mut_obj_add_int(doc, o, "delay_feedback", b->delay_feedback);
        yyjson_mut_obj_add_int(doc, o, "delay_mix", b->delay_mix);
        yyjson_mut_obj_add_int(doc, o, "drive_amount", b->drive_amount);
        yyjson_mut_obj_add_int(doc, o, "lowpass_amount", b->lowpass_amount);
        yyjson_mut_obj_add_int(doc, o, "sidechain_amount", b->sidechain_amount);
        yyjson_mut_obj_add_int(doc, o, "sidechain_release_ms", b->sidechain_release_ms);
        yyjson_mut_obj_add_int(doc, o, "mix_percent", b->mix_percent);
        yyjson_mut_obj_add_int(doc, o, "ladder_amount", b->ladder_amount);
        yyjson_mut_obj_add_int(doc, o, "ladder_cutoff", b->ladder_cutoff);
        yyjson_mut_obj_add_int(doc, o, "ladder_resonance", b->ladder_resonance);
        yyjson_mut_arr_append(buses, o);
    }
    yyjson_mut_obj_add_val(doc, root, "fx_buses", buses);

    /* patterns */
    yyjson_mut_val *patterns = yyjson_mut_arr(doc);
    for (int i = 0; i < music.pattern_count; i++) {
        const AbcPattern *p = &music.patterns[i];
        if (!p->defined) continue;
        yyjson_mut_val *o = yyjson_mut_obj(doc);
        yyjson_mut_obj_add_str(doc, o, "name", p->name);
        yyjson_mut_obj_add_int(doc, o, "length", p->length);
        yyjson_mut_arr_append(patterns, o);
    }
    yyjson_mut_obj_add_val(doc, root, "patterns", patterns);

    /* arrangement */
    yyjson_mut_val *arr = yyjson_mut_arr(doc);
    for (int i = 0; i < music.arrangement_length; i++)
        yyjson_mut_arr_add_str(doc, arr, music.arrangement[i]);
    yyjson_mut_obj_add_val(doc, root, "arrangement", arr);

    /* voices (track-level summary) */
    yyjson_mut_val *voices = yyjson_mut_arr(doc);
    for (int i = 0; i < music.voice_count; i++) {
        const AbcVoice *v = &music.voices[i];
        yyjson_mut_val *o = yyjson_mut_obj(doc);
        yyjson_mut_obj_add_str(doc, o, "name", v->name);
        yyjson_mut_obj_add_str(doc, o, "instrument_ref", v->instrument_ref);
        yyjson_mut_obj_add_int(doc, o, "note_count", v->note_count);
        yyjson_mut_obj_add_int(doc, o, "fx_bus", v->fx_bus);
        yyjson_mut_arr_append(voices, o);
    }
    yyjson_mut_obj_add_val(doc, root, "voices", voices);

    send_tool_payload(id, doc, root, 0);
}

/* ============================================================
 *   tool: memdeck_render_stats
 * ============================================================ */

static void tool_render_stats(yyjson_val *id, yyjson_val *args)
{
    const char *path = yyjson_get_str(yyjson_obj_get(args, "path"));
    if (!path) {
        write_jsonrpc_error(id, ERR_PARAMS, "missing 'path' argument");
        return;
    }

    /* Prime title (audio engine only fills numeric stats). */
    AbcMusic music;
    int loaded = (abc_load(path, &music) == 0);

    int pcm_len = 0;
    AudioRenderStats stats;
    unsigned char *pcm = audio_engine_render_abc_file(path, SAMPLE_RATE_ABC,
                                                      &pcm_len, &stats);
    if (!pcm || pcm_len <= 0) {
        if (pcm) audio_engine_free_buffer(pcm);
        send_tool_text_error(id, "render failed");
        return;
    }

    yyjson_mut_doc *doc = yyjson_mut_doc_new(NULL);
    yyjson_mut_val *root = yyjson_mut_obj(doc);
    yyjson_mut_obj_add_bool(doc, root, "ok", true);
    yyjson_mut_obj_add_str(doc, root, "path", path);
    yyjson_mut_obj_add_str(doc, root, "title", loaded ? music.title : "");
    yyjson_mut_obj_add_real(doc, root, "duration_sec", stats.duration_ms / 1000.0);
    yyjson_mut_obj_add_real(doc, root, "duration_ms", stats.duration_ms);
    yyjson_mut_obj_add_int(doc, root, "sample_count", (int64_t)stats.sample_count);
    yyjson_mut_obj_add_int(doc, root, "sample_rate", SAMPLE_RATE_ABC);
    yyjson_mut_obj_add_int(doc, root, "pcm_len", pcm_len);
    yyjson_mut_obj_add_int(doc, root, "min_sample", stats.min_sample);
    yyjson_mut_obj_add_int(doc, root, "max_sample", stats.max_sample);
    yyjson_mut_obj_add_int(doc, root, "peak", stats.peak);
    yyjson_mut_obj_add_int(doc, root, "clipping_count", (int64_t)stats.clipping_count);
    yyjson_mut_obj_add_real(doc, root, "render_time_ms", stats.render_time_ms);

    char checksum_hex[24];
    snprintf(checksum_hex, sizeof(checksum_hex),
             "0x%016llx", (unsigned long long)stats.checksum);
    yyjson_mut_obj_add_str(doc, root, "checksum", checksum_hex);

    audio_engine_free_buffer(pcm);
    send_tool_payload(id, doc, root, 0);
}

/* ============================================================
 *   tool: memdeck_validate_abc
 * ============================================================ */

static int write_tmp_abc(const char *content, size_t len, char *path_buf, size_t buf_len)
{
    snprintf(path_buf, buf_len, "/tmp/memdeck-mcp-validate-%d-%ld.abc",
             (int)getpid(), (long)time(NULL));
    FILE *f = fopen(path_buf, "wb");
    if (!f) return -1;
    size_t wrote = fwrite(content, 1, len, f);
    fclose(f);
    return wrote == len ? 0 : -1;
}

static void tool_validate_abc(yyjson_val *id, yyjson_val *args)
{
    yyjson_val *content_val = yyjson_obj_get(args, "content");
    if (!content_val || !yyjson_is_str(content_val)) {
        write_jsonrpc_error(id, ERR_PARAMS, "missing 'content' argument");
        return;
    }
    const char *content = yyjson_get_str(content_val);
    size_t content_len = yyjson_get_len(content_val);

    char tmp_path[256];
    if (write_tmp_abc(content, content_len, tmp_path, sizeof(tmp_path)) != 0) {
        write_jsonrpc_error(id, ERR_INTERNAL, "failed to write temp file");
        return;
    }

    AbcMusic music;
    int ok = (abc_load(tmp_path, &music) == 0);
    unlink(tmp_path);

    yyjson_mut_doc *doc = yyjson_mut_doc_new(NULL);
    yyjson_mut_val *root = yyjson_mut_obj(doc);
    yyjson_mut_obj_add_bool(doc, root, "ok", ok ? true : false);
    if (ok) {
        yyjson_mut_obj_add_str(doc, root, "title", music.title);
        yyjson_mut_obj_add_int(doc, root, "voice_count", music.voice_count);
        yyjson_mut_obj_add_int(doc, root, "pattern_count", music.pattern_count);
        yyjson_mut_obj_add_int(doc, root, "arrangement_length", music.arrangement_length);
        yyjson_mut_obj_add_int(doc, root, "fx_bus_count", music.fx_bus_count);
        yyjson_mut_obj_add_int(doc, root, "bpm", music.bpm);
    } else {
        yyjson_mut_obj_add_str(doc, root, "error",
                               "abc_load returned non-zero; "
                               "check directives, voice content, and pattern caps");
    }
    send_tool_payload(id, doc, root, ok ? 0 : 1);
}

/* ============================================================
 *   tool: memdeck_list_demos
 * ============================================================ */

static int read_title(const char *path, char *out, size_t out_len)
{
    FILE *f = fopen(path, "r");
    if (!f) return -1;
    char buf[1024];
    out[0] = '\0';
    while (fgets(buf, sizeof(buf), f)) {
        if (buf[0] == 'T' && buf[1] == ':') {
            const char *p = buf + 2;
            while (*p == ' ' || *p == '\t') p++;
            size_t i = 0;
            while (*p && *p != '\n' && *p != '\r' && i + 1 < out_len) {
                out[i++] = *p++;
            }
            out[i] = '\0';
            fclose(f);
            return 0;
        }
    }
    fclose(f);
    return -1;
}

static int cmp_strs(const void *a, const void *b)
{
    return strcmp(*(const char **)a, *(const char **)b);
}

static void tool_list_demos(yyjson_val *id, yyjson_val *args)
{
    (void)args;
    const char *music_dir = "data/music";
    DIR *d = opendir(music_dir);
    if (!d) {
        send_tool_text_error(id, "data/music/ not readable from cwd");
        return;
    }

    /* Collect matching filenames first so we can sort. */
    char **names = NULL;
    size_t count = 0, cap = 0;
    struct dirent *ent;
    while ((ent = readdir(d)) != NULL) {
        size_t nlen = strlen(ent->d_name);
        if (nlen < 5) continue;
        if (strcmp(ent->d_name + nlen - 4, ".abc") != 0) continue;
        if (strncmp(ent->d_name, "menu", 4) == 0) continue;
        if (count == cap) {
            cap = cap ? cap * 2 : 16;
            names = realloc(names, cap * sizeof(*names));
        }
        names[count++] = strdup(ent->d_name);
    }
    closedir(d);
    qsort(names, count, sizeof(*names), cmp_strs);

    yyjson_mut_doc *doc = yyjson_mut_doc_new(NULL);
    yyjson_mut_val *root = yyjson_mut_obj(doc);
    yyjson_mut_val *demos = yyjson_mut_arr(doc);

    for (size_t i = 0; i < count; i++) {
        char path[512];
        snprintf(path, sizeof(path), "%s/%s", music_dir, names[i]);
        char title[160] = "";
        read_title(path, title, sizeof(title));

        /* key = filename without .abc */
        char key[256];
        size_t klen = strlen(names[i]) - 4;
        if (klen >= sizeof(key)) klen = sizeof(key) - 1;
        memcpy(key, names[i], klen);
        key[klen] = '\0';

        yyjson_mut_val *o = yyjson_mut_obj(doc);
        yyjson_mut_obj_add_strcpy(doc, o, "key", key);
        yyjson_mut_obj_add_strcpy(doc, o, "path", path);
        yyjson_mut_obj_add_strcpy(doc, o, "title", title);
        yyjson_mut_arr_append(demos, o);
        free(names[i]);
    }
    free(names);

    yyjson_mut_obj_add_val(doc, root, "demos", demos);
    yyjson_mut_obj_add_int(doc, root, "count", (int64_t)count);
    send_tool_payload(id, doc, root, 0);
}

/* ============================================================
 *   tool: memdeck_directive_help
 * ============================================================ */

static void tool_directive_help(yyjson_val *id, yyjson_val *args)
{
    (void)args;
    yyjson_mut_doc *doc = yyjson_mut_doc_new(NULL);
    yyjson_mut_val *root = yyjson_mut_obj(doc);

    yyjson_mut_val *arr = yyjson_mut_arr(doc);

    /* %%instrument */
    {
        yyjson_mut_val *o = yyjson_mut_obj(doc);
        yyjson_mut_obj_add_str(doc, o, "directive", "%%instrument <name> wave=<X> amp=<0..127> ...");
        yyjson_mut_obj_add_str(doc, o, "purpose",
            "Define a synth voice. Subsequent V:<voice> instrument=<name> "
            "lines bind voices to instruments.");
        yyjson_mut_val *params = yyjson_mut_arr(doc);
        yyjson_mut_arr_add_str(doc, params, "wave={square|pulse|triangle|noise}");
        yyjson_mut_arr_add_str(doc, params, "amp=<0..127> output amplitude");
        yyjson_mut_arr_add_str(doc, params, "duty=<1..99> pulse duty cycle %");
        yyjson_mut_arr_add_str(doc, params, "attack=<ms> envelope attack");
        yyjson_mut_arr_add_str(doc, params, "decay=<ms> envelope decay");
        yyjson_mut_arr_add_str(doc, params, "sustain=<0..100> sustain level");
        yyjson_mut_arr_add_str(doc, params, "release=<ms> envelope release");
        yyjson_mut_arr_add_str(doc, params, "gate=<1..100> note gate %");
        yyjson_mut_arr_add_str(doc, params, "vibrato=<cents>");
        yyjson_mut_arr_add_str(doc, params, "glide=<ms>");
        yyjson_mut_arr_add_str(doc, params, "fx=<0..3> route to FX bus N");
        yyjson_mut_arr_add_str(doc, params, "preset=<name> optional preset label");
        yyjson_mut_obj_add_val(doc, o, "params", params);
        yyjson_mut_arr_append(arr, o);
    }

    /* %%effect */
    {
        yyjson_mut_val *o = yyjson_mut_obj(doc);
        yyjson_mut_obj_add_str(doc, o, "directive", "%%effect <N> delay_steps=<x> ... mix=<x> [ladder=<x> ...]");
        yyjson_mut_obj_add_str(doc, o, "purpose",
            "Configure FX bus N (0..3). Chain order: drive -> lowpass -> "
            "ladder -> delay -> sidechain -> mix.");
        yyjson_mut_val *params = yyjson_mut_arr(doc);
        yyjson_mut_arr_add_str(doc, params, "delay_steps=<0..64> delay length in steps");
        yyjson_mut_arr_add_str(doc, params, "delay_feedback=<0..95>");
        yyjson_mut_arr_add_str(doc, params, "delay_mix=<0..100>");
        yyjson_mut_arr_add_str(doc, params, "drive=<0..100> soft-clip saturation");
        yyjson_mut_arr_add_str(doc, params, "lowpass=<0..100> one-pole tilt");
        yyjson_mut_arr_add_str(doc, params, "sidechain=<0..100> kick-triggered duck");
        yyjson_mut_arr_add_str(doc, params, "sidechain_release=<10..2000> ms");
        yyjson_mut_arr_add_str(doc, params, "mix=<1..100> bus output mix");
        yyjson_mut_arr_add_str(doc, params, "ladder=<0..100> moog ladder wet, 0 disables");
        yyjson_mut_arr_add_str(doc, params, "ladder_cutoff=<1..100> %% of Nyquist");
        yyjson_mut_arr_add_str(doc, params, "ladder_resonance=<0..100> near 100 self-oscillates");
        yyjson_mut_obj_add_val(doc, o, "params", params);
        yyjson_mut_arr_append(arr, o);
    }

    /* %%pattern */
    {
        yyjson_mut_val *o = yyjson_mut_obj(doc);
        yyjson_mut_obj_add_str(doc, o, "directive", "%%pattern <name> length=<steps>");
        yyjson_mut_obj_add_str(doc, o, "purpose",
            "Declare a pattern (A, B, ...) with its length in default-note "
            "steps. Pattern length <= SEQ_MAX_STEPS (64).");
        yyjson_mut_arr_append(arr, o);
    }

    /* %%arrangement */
    {
        yyjson_mut_val *o = yyjson_mut_obj(doc);
        yyjson_mut_obj_add_str(doc, o, "directive", "%%arrangement A B C ...");
        yyjson_mut_obj_add_str(doc, o, "purpose",
            "Sequence of pattern names. Voice content is consumed linearly: "
            "each arrangement slot advances the cursor by its pattern length, "
            "so reusing 'A' twice still reads two separate chunks from each "
            "voice. Max ABC_MAX_ARRANGEMENT (32) entries.");
        yyjson_mut_arr_append(arr, o);
    }

    /* %%swing */
    {
        yyjson_mut_val *o = yyjson_mut_obj(doc);
        yyjson_mut_obj_add_str(doc, o, "directive", "%%swing <50..75>");
        yyjson_mut_obj_add_str(doc, o, "purpose",
            "Swing ratio. 50 = straight, 67 = triplet feel.");
        yyjson_mut_arr_append(arr, o);
    }

    yyjson_mut_obj_add_val(doc, root, "directives", arr);
    yyjson_mut_obj_add_str(doc, root, "header_fields",
        "X: index, T: title, C: composer, M: meter (e.g. 4/4), "
        "L: default note (e.g. 1/8), Q: tempo (e.g. 1/4=120), K: key (e.g. Am, Cm, A).");
    send_tool_payload(id, doc, root, 0);
}

/* ============================================================
 *   tool: memdeck_duration_calc
 * ============================================================ */

static void tool_duration_calc(yyjson_val *id, yyjson_val *args)
{
    yyjson_val *bpm_v = yyjson_obj_get(args, "bpm");
    yyjson_val *l_v   = yyjson_obj_get(args, "l_denom");
    yyjson_val *st_v  = yyjson_obj_get(args, "total_steps");
    if (!bpm_v || !l_v || !st_v) {
        write_jsonrpc_error(id, ERR_PARAMS, "missing bpm / l_denom / total_steps");
        return;
    }
    int bpm = (int)yyjson_get_int(bpm_v);
    int l_denom = (int)yyjson_get_int(l_v);
    int total_steps = (int)yyjson_get_int(st_v);
    if (bpm <= 0 || l_denom <= 0 || total_steps <= 0) {
        send_tool_text_error(id, "bpm, l_denom, total_steps must all be > 0");
        return;
    }

    /* L:1/N means each step is a 1/N note. At BPM (quarter notes/min):
     *   steps per second = (BPM/60) * (N/4)
     * Duration = total_steps / steps_per_sec. */
    double steps_per_sec = ((double)bpm / 60.0) * ((double)l_denom / 4.0);
    double duration_sec = (double)total_steps / steps_per_sec;

    yyjson_mut_doc *doc = yyjson_mut_doc_new(NULL);
    yyjson_mut_val *root = yyjson_mut_obj(doc);
    yyjson_mut_obj_add_bool(doc, root, "ok", true);
    yyjson_mut_obj_add_int(doc, root, "bpm", bpm);
    yyjson_mut_obj_add_int(doc, root, "l_denom", l_denom);
    yyjson_mut_obj_add_int(doc, root, "total_steps", total_steps);
    yyjson_mut_obj_add_real(doc, root, "steps_per_sec", steps_per_sec);
    yyjson_mut_obj_add_real(doc, root, "duration_sec", duration_sec);
    yyjson_mut_obj_add_int(doc, root, "max_timeline_steps", SEQ_MAX_TIMELINE_STEPS);
    yyjson_mut_obj_add_bool(doc, root, "fits_engine_cap",
                            total_steps <= SEQ_MAX_TIMELINE_STEPS ? true : false);
    send_tool_payload(id, doc, root, 0);
}

/* ============================================================
 *   tool: memdeck_engine_caps
 * ============================================================ */

static void tool_engine_caps(yyjson_val *id, yyjson_val *args)
{
    (void)args;
    yyjson_mut_doc *doc = yyjson_mut_doc_new(NULL);
    yyjson_mut_val *root = yyjson_mut_obj(doc);

    yyjson_mut_obj_add_int(doc, root, "seq_max_tracks", SEQ_MAX_TRACKS);
    yyjson_mut_obj_add_int(doc, root, "seq_max_patterns", SEQ_MAX_PATTERNS);
    yyjson_mut_obj_add_int(doc, root, "seq_max_steps", SEQ_MAX_STEPS);
    yyjson_mut_obj_add_int(doc, root, "seq_max_arrangement", SEQ_MAX_ARRANGEMENT);
    yyjson_mut_obj_add_int(doc, root, "seq_max_timeline_steps", SEQ_MAX_TIMELINE_STEPS);
    yyjson_mut_obj_add_int(doc, root, "seq_max_instruments", SEQ_MAX_INSTRUMENTS);
    yyjson_mut_obj_add_int(doc, root, "seq_max_fx_buses", SEQ_MAX_FX_BUSES);
    yyjson_mut_obj_add_int(doc, root, "abc_max_voices", ABC_MAX_VOICES);
    yyjson_mut_obj_add_int(doc, root, "abc_max_notes", ABC_MAX_NOTES);
    yyjson_mut_obj_add_int(doc, root, "abc_max_patterns", ABC_MAX_PATTERNS);
    yyjson_mut_obj_add_int(doc, root, "abc_max_arrangement", ABC_MAX_ARRANGEMENT);
    yyjson_mut_obj_add_int(doc, root, "abc_max_fx_buses", ABC_MAX_FX_BUSES);
    yyjson_mut_obj_add_int(doc, root, "sample_rate_abc", SAMPLE_RATE_ABC);

    yyjson_mut_val *ladder = yyjson_mut_obj(doc);
    yyjson_mut_obj_add_str(doc, ladder, "amount", "0..100 (wet mix; 0 disables)");
    yyjson_mut_obj_add_str(doc, ladder, "cutoff", "1..100 (percent of Nyquist)");
    yyjson_mut_obj_add_str(doc, ladder, "resonance", "0..100 (self-oscillates near 100)");
    yyjson_mut_obj_add_str(doc, ladder, "position", "between lowpass and delay in bus chain");
    yyjson_mut_obj_add_val(doc, root, "ladder", ladder);

    send_tool_payload(id, doc, root, 0);
}

/* ============================================================
 *   tools/call dispatcher
 * ============================================================ */

static void handle_tools_call(yyjson_val *id, yyjson_val *params)
{
    yyjson_val *name = params ? yyjson_obj_get(params, "name") : NULL;
    yyjson_val *args = params ? yyjson_obj_get(params, "arguments") : NULL;
    if (!name || !yyjson_is_str(name)) {
        write_jsonrpc_error(id, ERR_PARAMS, "missing tool 'name'");
        return;
    }
    const char *n = yyjson_get_str(name);

    if (strcmp(n, "memdeck_inspect_abc") == 0)      tool_inspect_abc(id, args);
    else if (strcmp(n, "memdeck_render_stats") == 0) tool_render_stats(id, args);
    else if (strcmp(n, "memdeck_validate_abc") == 0) tool_validate_abc(id, args);
    else if (strcmp(n, "memdeck_list_demos") == 0)   tool_list_demos(id, args);
    else if (strcmp(n, "memdeck_directive_help") == 0) tool_directive_help(id, args);
    else if (strcmp(n, "memdeck_duration_calc") == 0) tool_duration_calc(id, args);
    else if (strcmp(n, "memdeck_engine_caps") == 0)  tool_engine_caps(id, args);
    else {
        char msg[128];
        snprintf(msg, sizeof(msg), "unknown tool: %s", n);
        write_jsonrpc_error(id, ERR_METHOD, msg);
    }
}

/* ============================================================
 *   initialize
 * ============================================================ */

static void handle_initialize(yyjson_val *id)
{
    yyjson_mut_doc *doc = yyjson_mut_doc_new(NULL);
    yyjson_mut_val *result = yyjson_mut_obj(doc);

    yyjson_mut_obj_add_str(doc, result, "protocolVersion", MCP_PROTOCOL_VERSION);

    yyjson_mut_val *capabilities = yyjson_mut_obj(doc);
    yyjson_mut_val *tools_cap = yyjson_mut_obj(doc);
    yyjson_mut_obj_add_bool(doc, tools_cap, "listChanged", false);
    yyjson_mut_obj_add_val(doc, capabilities, "tools", tools_cap);
    yyjson_mut_obj_add_val(doc, result, "capabilities", capabilities);

    yyjson_mut_val *info = yyjson_mut_obj(doc);
    yyjson_mut_obj_add_str(doc, info, "name", SERVER_NAME);
    yyjson_mut_obj_add_str(doc, info, "version", SERVER_VERSION);
    yyjson_mut_obj_add_val(doc, result, "serverInfo", info);

    write_envelope(id, doc, result, 0);
    yyjson_mut_doc_free(doc);
}

/* ============================================================
 *   dispatch
 * ============================================================ */

static void dispatch(const char *line, size_t line_len)
{
    yyjson_doc *doc = yyjson_read(line, line_len, 0);
    if (!doc) {
        write_jsonrpc_error(NULL, ERR_PARSE, "invalid JSON");
        return;
    }
    yyjson_val *root = yyjson_doc_get_root(doc);
    if (!yyjson_is_obj(root)) {
        write_jsonrpc_error(NULL, ERR_INVALID_REQ, "request not an object");
        yyjson_doc_free(doc);
        return;
    }

    yyjson_val *id     = yyjson_obj_get(root, "id");
    yyjson_val *method = yyjson_obj_get(root, "method");
    yyjson_val *params = yyjson_obj_get(root, "params");

    if (!method || !yyjson_is_str(method)) {
        write_jsonrpc_error(id, ERR_INVALID_REQ, "missing method");
        yyjson_doc_free(doc);
        return;
    }
    const char *m = yyjson_get_str(method);

    if (strcmp(m, "initialize") == 0) {
        handle_initialize(id);
    } else if (strcmp(m, "tools/list") == 0) {
        handle_tools_list(id);
    } else if (strcmp(m, "tools/call") == 0) {
        handle_tools_call(id, params);
    } else if (strcmp(m, "ping") == 0) {
        yyjson_mut_doc *rdoc = yyjson_mut_doc_new(NULL);
        yyjson_mut_val *r = yyjson_mut_obj(rdoc);
        write_envelope(id, rdoc, r, 0);
        yyjson_mut_doc_free(rdoc);
    } else if (strncmp(m, "notifications/", 14) == 0) {
        /* notifications expect no response */
    } else {
        char buf[128];
        snprintf(buf, sizeof(buf), "method not found: %s", m);
        write_jsonrpc_error(id, ERR_METHOD, buf);
    }

    yyjson_doc_free(doc);
}

/* ============================================================
 *   main
 * ============================================================ */

int main(void)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    char *line = NULL;
    size_t cap = 0;
    ssize_t n;
    while ((n = getline(&line, &cap, stdin)) != -1) {
        /* trim newline */
        while (n > 0 && (line[n - 1] == '\n' || line[n - 1] == '\r'))
            line[--n] = '\0';
        if (n == 0) continue;
        dispatch(line, (size_t)n);
    }
    free(line);
    return 0;
}
