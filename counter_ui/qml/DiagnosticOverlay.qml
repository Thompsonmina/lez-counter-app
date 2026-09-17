pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

// The only debug channel a Basecamp-hosted view actually has.
//
// `console.log` does not reach basecamp.log and the ui-host's stderr is a pipe
// the host discards, so an on-screen readout is the instrument. Everything here
// is read through `typeof` and guarded, because the failure mode in this host
// is `undefined`, not an exception.
Rectangle {
    id: root

    required property AppTheme theme
    required property ChainReader reader

    color: theme.panelAlt
    border.color: theme.border
    border.width: 1
    radius: theme.radius
    implicitHeight: col.implicitHeight + theme.pad * 2

    function probe(expr) {
        try { return expr() } catch (e) { return "threw: " + e }
    }

    ColumnLayout {
        id: col
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: root.theme.pad
        spacing: 4

        Text {
            text: "diagnostics"
            color: root.theme.textDim
            font.pixelSize: root.theme.fontSmall
            font.letterSpacing: 1
        }

        Repeater {
            model: [
                { k: "typeof logos",   v: root.probe(function () { return typeof logos }) },
                { k: "callModuleAsync", v: root.probe(function () {
                        return (typeof logos !== "undefined" && logos !== null)
                            ? typeof logos.callModuleAsync : "n/a" }) },
                { k: "phase",          v: root.reader.phase },
                { k: "reads / failed", v: root.reader.reads + " / " + root.reader.failures },
                { k: "events",         v: String(root.reader.events) },
                { k: "last call",      v: root.reader.lastMs + " ms at " + (root.reader.lastAt || "—") },
                { k: "unwrap depth",   v: String(root.reader.lastDepth) },
                { k: "auto-refresh",   v: root.reader.autoRefresh
                        ? ("on, every " + Math.round(root.reader.intervalMs / 1000) + "s")
                        : "off" }
            ]

            delegate: RowLayout {
                id: del
                required property var modelData
                Layout.fillWidth: true
                spacing: root.theme.gap
                Text {
                    Layout.preferredWidth: 110
                    text: del.modelData.k
                    color: root.theme.textDim
                    font.pixelSize: root.theme.fontSmall
                    font.family: root.theme.mono
                }
                Text {
                    Layout.fillWidth: true
                    text: String(del.modelData.v)
                    color: root.theme.text
                    font.pixelSize: root.theme.fontSmall
                    font.family: root.theme.mono
                    elide: Text.ElideRight
                }
            }
        }

        Text {
            Layout.fillWidth: true
            Layout.topMargin: 6
            text: "last payload"
            color: root.theme.textDim
            font.pixelSize: root.theme.fontSmall
            font.letterSpacing: 1
        }

        Text {
            Layout.fillWidth: true
            text: root.reader.lastRaw.length > 0 ? root.reader.lastRaw : "(nothing yet)"
            color: root.theme.textDim
            font.pixelSize: root.theme.fontSmall
            font.family: root.theme.mono
            wrapMode: Text.WrapAnywhere
            maximumLineCount: 6
            elide: Text.ElideRight
        }
    }
}
