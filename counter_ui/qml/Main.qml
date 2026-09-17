import QtQuick
import QtQuick.Layouts

// LEZ Counter — the `lez_counter` program's state, read from the public
// testnet through counter_reader.
//
// QML-only: no .rep, no C++. The whole surface is four values and a refresh, so
// the typed-backend seam would be cost without benefit, and the app stays at
// zero hand-written C++.
Item {
    id: root

    implicitWidth: 560
    implicitHeight: 720

    AppTheme {
        id: appTheme
        // Follow the host's theme when it offers one. `Theme` is Basecamp's own
        // injected object — present in the host, absent in the
        // standalone — so it is read defensively and never depended on.
        dark: {
            try {
                if (typeof Theme !== "undefined" && Theme !== null
                        && typeof Theme.isDark === "boolean")
                    return Theme.isDark
            } catch (e) { /* host has no theme; ours stands */ }
            return true
        }
    }

    ChainReader {
        id: reader

        onCounterMoved: function (newCount, atBlock) {
            pulse.restart()
        }
    }

    Connections {
        target: (typeof logos !== "undefined") ? logos : null

        // The core emits counter_changed only when a read observes a count
        // different from the last one, so this fires on an actual on-chain
        // change rather than on the act of polling.
        function onModuleEventReceived(moduleName, eventName, data) {
            if (moduleName !== reader.moduleName) return
            if (eventName !== "counter_changed") return
            reader.events += 1
            pulse.restart()
        }
    }

    Component.onCompleted: {
        if (typeof logos !== "undefined" && logos !== null) {
            try { logos.onModuleEvent(reader.moduleName, "counter_changed") }
            catch (e) { /* subscription is a nicety; polling still carries us */ }
        }
        reader.refreshConfig()
        reader.refresh()
    }

    Rectangle {
        anchors.fill: parent
        color: appTheme.bg
    }

    Flickable {
        anchors.fill: parent
        contentWidth: width
        contentHeight: body.implicitHeight + appTheme.pad * 2
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        ColumnLayout {
            id: body
            x: appTheme.pad
            y: appTheme.pad
            width: parent.width - appTheme.pad * 2
            spacing: appTheme.pad

            // ── Header ───────────────────────────────────────────────────
            RowLayout {
                Layout.fillWidth: true
                spacing: appTheme.gap

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        text: "LEZ Counter"
                        color: appTheme.text
                        font.pixelSize: 20
                        font.bold: true
                    }
                    Text {
                        text: "lez_counter on the public testnet · read-only"
                        color: appTheme.textDim
                        font.pixelSize: appTheme.fontSmall
                    }
                }

                // Liveness: green once a read has landed, amber while the first
                // one is in flight, red when the last one refused.
                Rectangle {
                    width: 10; height: 10; radius: 5
                    Layout.alignment: Qt.AlignVCenter
                    color: reader.phase === "ok" ? appTheme.good
                         : reader.phase === "error" ? appTheme.bad
                         : appTheme.warn
                    opacity: reader.busy ? 0.45 : 1.0
                    Behavior on opacity { NumberAnimation { duration: 180 } }
                }
            }

            // ── The count ────────────────────────────────────────────────
            Panel {
                Layout.fillWidth: true
                theme: appTheme

                // Centred by letting each line fill the panel and centring the
                // text inside it, rather than by aligning a narrow item in a
                // wide cell. A nested ColumnLayout here does NOT honour
                // Layout.fillWidth — it collapses to its widest child, and the
                // children then centre inside that, which merely looked right
                // while a long line happened to be present.
                Text {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                    text: "COUNT"
                    color: appTheme.textDim
                    font.pixelSize: appTheme.fontSmall
                    font.letterSpacing: 2
                }

                Text {
                    id: countText
                    Layout.fillWidth: true
                    Layout.topMargin: -2
                    horizontalAlignment: Text.AlignHCenter
                    text: reader.count === undefined ? "—" : String(reader.count)
                    // A change is worth noticing: the number is the whole point
                    // of the app and it moves rarely. Driven off a flag rather
                    // than animated directly, so the binding to the theme
                    // survives the animation.
                    color: pulse.running ? appTheme.accent : appTheme.text
                    Behavior on color { ColorAnimation { duration: 260 } }
                    font.pixelSize: appTheme.fontHuge
                    font.bold: true
                    font.letterSpacing: -1

                    Timer { id: pulse; interval: 1100 }
                }

                Text {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                    visible: reader.lastUpdated !== undefined
                    text: "last changed at block " + reader.lastUpdated
                          + (reader.blockHeight !== undefined && reader.lastUpdated !== undefined
                             ? "  ·  " + (reader.blockHeight - reader.lastUpdated) + " blocks ago"
                             : "")
                    color: appTheme.textDim
                    font.pixelSize: appTheme.fontSmall
                }
            }

            // ── Refusal, when there is one ───────────────────────────────
            Panel {
                Layout.fillWidth: true
                theme: appTheme
                visible: reader.phase === "error"
                color: appTheme.panel
                border.color: appTheme.bad

                RowLayout {
                    Layout.fillWidth: true
                    spacing: appTheme.gap

                    Rectangle {
                        Layout.alignment: Qt.AlignTop
                        width: 3
                        Layout.preferredHeight: errCol.implicitHeight
                        color: appTheme.bad
                        radius: 2
                    }

                    ColumnLayout {
                        id: errCol
                        Layout.fillWidth: true
                        spacing: 4

                        Text {
                            text: reader.labelFor(reader.errorKind)
                            color: appTheme.bad
                            font.pixelSize: appTheme.fontBody
                            font.bold: true
                        }
                        Text {
                            Layout.fillWidth: true
                            text: reader.errorText
                            color: appTheme.text
                            font.pixelSize: appTheme.fontSmall
                            font.family: appTheme.mono
                            wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                        }
                        Text {
                            Layout.fillWidth: true
                            text: reader.remedyFor(reader.errorKind)
                            color: appTheme.textDim
                            font.pixelSize: appTheme.fontSmall
                            wrapMode: Text.WordWrap
                        }
                    }
                }
            }

            // ── Where it came from ───────────────────────────────────────
            Panel {
                Layout.fillWidth: true
                theme: appTheme

                Text {
                    text: "ACCOUNT"
                    color: appTheme.textDim
                    font.pixelSize: appTheme.fontSmall
                    font.letterSpacing: 2
                }
                FieldRow { Layout.fillWidth: true; theme: appTheme
                           label: "address"; value: reader.pda; mono: true }
                FieldRow { Layout.fillWidth: true; theme: appTheme
                           label: "owner"; value: reader.owner; mono: true }
                FieldRow { Layout.fillWidth: true; theme: appTheme
                           label: "chain head"
                           value: reader.blockHeight === undefined ? "" : String(reader.blockHeight) }

                Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1
                            Layout.topMargin: 4; Layout.bottomMargin: 4
                            color: appTheme.border }

                Text {
                    text: "SOURCE"
                    color: appTheme.textDim
                    font.pixelSize: appTheme.fontSmall
                    font.letterSpacing: 2
                }
                FieldRow { Layout.fillWidth: true; theme: appTheme
                           label: "sequencer"; value: reader.endpoint }
                FieldRow { Layout.fillWidth: true; theme: appTheme
                           label: "program"; value: reader.programId; mono: true }
            }

            // ── Controls ─────────────────────────────────────────────────
            RowLayout {
                Layout.fillWidth: true
                spacing: appTheme.gap

                ActionButton {
                    theme: appTheme
                    primary: true
                    text: reader.busy ? "Reading…" : "Refresh"
                    enabled: !reader.busy
                    onClicked: reader.refresh()
                }

                ToggleChip {
                    theme: appTheme
                    text: "Auto"
                    checked: reader.autoRefresh
                    onToggled: function (value) { reader.autoRefresh = value }
                }

                Item { Layout.fillWidth: true }

                ActionButton {
                    theme: appTheme
                    text: diagnostics.visible ? "Hide diagnostics" : "Diagnostics"
                    onClicked: diagnostics.visible = !diagnostics.visible
                }
            }

            // When the screen was last true. Without it a stale panel and a
            // fresh one look identical, which is the thing a poller has to
            // answer for.
            Text {
                Layout.fillWidth: true
                text: {
                    if (reader.reads === 0) return "not read yet"
                    var when = "read " + ago.stamp
                    return reader.autoRefresh
                        ? when + " · refreshing every "
                                + Math.round(reader.intervalMs / 1000) + "s"
                        : when + " · auto-refresh off"
                }
                color: appTheme.textDim
                font.pixelSize: appTheme.fontSmall

                QtObject {
                    id: ago
                    property string stamp: "just now"
                }

                Timer {
                    interval: 1000
                    repeat: true
                    running: true
                    onTriggered: {
                        if (reader.readAt === 0) { ago.stamp = "just now"; return }
                        var secs = Math.max(0, Math.round((Date.now() - reader.readAt) / 1000))
                        ago.stamp = secs < 5 ? "just now"
                                  : secs < 60 ? secs + "s ago"
                                  : Math.round(secs / 60) + "m ago"
                    }
                }
            }

            DiagnosticOverlay {
                id: diagnostics
                Layout.fillWidth: true
                theme: appTheme
                reader: reader
                visible: false
            }
        }
    }
}
