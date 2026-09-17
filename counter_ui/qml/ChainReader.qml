import QtQuick

// Everything that talks to counter_reader, and nothing that draws.
//
// The whole surface is `refresh()` plus the properties below, so the view never
// touches the bridge directly and the awkward parts of the seam — double JSON
// encoding, the bridge's own error shape, the three distinct refusals — are
// handled in exactly one place.
QtObject {
    id: root

    // ── The module this reads ────────────────────────────────────────────
    readonly property string moduleName: "counter_reader"

    // ── State ────────────────────────────────────────────────────────────
    // "idle" before the first read; "loading" only for the first one, so a
    // background poll never blanks a screen that already has an answer.
    property string phase: "idle"          // idle | loading | ok | error
    property bool   busy: false

    property var    count: undefined       // undefined until a read succeeds
    property string owner: ""
    property string pda: ""
    property var    lastUpdated: undefined
    property var    blockHeight: undefined

    property string endpoint: ""
    property string programId: ""

    // Four kinds, four different fixes. `bridge` is ours (the host could not
    // reach the module at all); the other three come from the core.
    property string errorKind: ""          // not_initialised | not_ours | transport | bridge
    property string errorText: ""

    // ── Diagnostics a hosted view has no textual error
    //    channel, so the instrument has to be on screen ─────────────
    property int    reads: 0
    property int    failures: 0
    property int    events: 0
    property int    lastMs: 0
    // Epoch ms of the last completed read, so the view can say how stale it is.
    property double readAt: 0
    property string lastRaw: ""
    property int    lastDepth: 0
    property string lastAt: ""
    property bool   bridgePresent: false

    // Named `counterMoved`, not `countChanged`: QML already generates a
    // `countChanged()` signal for the `count` property, and a same-named signal
    // declaration collides with it.
    signal counterMoved(var newCount, var atBlock)

    // ── Polling ──────────────────────────────────────────────────────────
    property bool autoRefresh: true
    property int  intervalMs: 15000        // LEZ blocks land ~every 20s

    property Timer poller: Timer {
        interval: root.intervalMs
        repeat: true
        running: root.autoRefresh
        onTriggered: root.refresh()
    }

    // ── Wire handling ────────────────────────────────────────────────────

    // A core method returns a JSON *string*, and the bridge then serialises
    // that QString as JSON — so a structured result arrives double-encoded and
    // a scalar one arrives as a quoted literal. Unwrap until it stops being a
    // string that looks like JSON, rather than assuming a fixed depth.
    function decode(payload) {
        var v = payload
        var depth = 0
        for (var i = 0; i < 4; i++) {
            if (typeof v !== "string") break
            var t = v.trim()
            if (t.length === 0) break
            var c = t.charAt(0)
            if (c !== "{" && c !== "[" && c !== "\"") break
            try { v = JSON.parse(t) } catch (e) { break }
            depth += 1
        }
        lastDepth = depth
        return v
    }

    function hasBridge() {
        return (typeof logos !== "undefined") && logos !== null
    }

    function call(method, args, done) {
        if (!hasBridge()) {
            done({ ok: false, kind: "bridge", error: "no logos bridge in this context" })
            return
        }
        var started = Date.now()
        try {
            logos.callModuleAsync(moduleName, method, args || [], function (payload) {
                root.lastMs = Date.now() - started
                root.lastRaw = String(payload).slice(0, 400)
                root.lastAt = Qt.formatTime(new Date(), "hh:mm:ss")
                var v = root.decode(payload)
                if (v === null || typeof v !== "object") {
                    // A scalar answer (endpoint, program_id) is its own result.
                    done({ ok: true, value: v })
                    return
                }
                // The bridge's own failures carry `error` and no `ok`.
                if (v.ok === undefined && v.error !== undefined) {
                    done({ ok: false, kind: "bridge", error: String(v.error) })
                    return
                }
                done(v)
            }, 20000)
        } catch (e) {
            root.lastRaw = "threw: " + e
            done({ ok: false, kind: "bridge", error: String(e) })
        }
    }

    // ── Reads ────────────────────────────────────────────────────────────

    function refresh() {
        if (busy) return          // a slow read must not stack up behind a poll
        busy = true
        bridgePresent = hasBridge()
        if (phase === "idle" || phase === "error") phase = "loading"

        call("counter", [], function (r) {
            root.busy = false
            root.reads += 1
            root.readAt = Date.now()
            if (r.ok) {
                var moved = (root.count !== undefined && r.count !== root.count)
                root.count = r.count
                root.owner = r.owner || ""
                root.pda = r.pda || ""
                root.lastUpdated = r.lastUpdated
                root.blockHeight = r.blockHeight
                root.errorKind = ""
                root.errorText = ""
                root.phase = "ok"
                if (moved) root.counterMoved(r.count, r.lastUpdated)
            } else {
                root.failures += 1
                root.errorKind = r.kind || "transport"
                root.errorText = r.error || "unknown failure"
                root.phase = "error"
            }
        })
    }

    // Configuration, read once — it only changes if someone changes it.
    function refreshConfig() {
        call("endpoint", [], function (r) {
            if (r.ok && r.value !== undefined) root.endpoint = String(r.value)
        })
        call("program_id", [], function (r) {
            if (r.ok && r.value !== undefined) root.programId = String(r.value)
        })
    }

    // What each refusal actually means, and what to do about it. The core keeps
    // these three apart on purpose; collapsing them here would throw that away.
    function remedyFor(kind) {
        switch (kind) {
        case "not_initialised":
            return "Nobody has run `initialize` against this program yet, so the counter account does not exist. Deploy and initialise, or point at a deployment that has."
        case "not_ours":
            return "Something lives at that address but it is not our counter — another program owns it, or it is the wrong size. Usually means the program id is from a different deployment."
        case "transport":
            return "The sequencer could not be reached, or did not answer in time. The chain state is unknown, not empty."
        case "bridge":
            return "The host could not reach counter_reader. It may not be loaded yet — it is declared as a dependency, so give it a moment, or check the module list."
        default:
            return ""
        }
    }

    function labelFor(kind) {
        switch (kind) {
        case "not_initialised": return "Not initialised"
        case "not_ours":        return "Wrong account"
        case "transport":       return "Sequencer unreachable"
        case "bridge":          return "Module unreachable"
        default:                return "Failed"
        }
    }
}
