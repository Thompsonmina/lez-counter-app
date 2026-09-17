import QtQuick

// Design tokens.
//
// NEVER name this `Theme`. Basecamp injects a host object under that exact name
// into plugin QML and it silently shadows a same-named type or singleton —
// reads come back `undefined` with no error and no warning.
// `AppTheme` is a name the host cannot own.
QtObject {
    id: root

    // Follows the host when the host says; otherwise dark, which is what
    // Basecamp ships by default.
    property bool dark: true

    readonly property color bg:         dark ? "#0f1115" : "#f5f6f8"
    readonly property color panel:      dark ? "#171a21" : "#ffffff"
    readonly property color panelAlt:   dark ? "#1d212a" : "#f0f2f5"
    readonly property color border:     dark ? "#272c37" : "#dfe3ea"
    readonly property color text:       dark ? "#e6e9ef" : "#14161a"
    readonly property color textDim:    dark ? "#8b93a3" : "#5d6675"
    readonly property color accent:     dark ? "#6ea8fe" : "#1f6feb"
    readonly property color good:       dark ? "#4ec9a5" : "#118d68"
    readonly property color warn:       dark ? "#e3b341" : "#9a6700"
    readonly property color bad:        dark ? "#f2777a" : "#c93c3c"

    readonly property int   pad:        16
    readonly property int   gap:        10
    readonly property int   radius:     10
    readonly property int   fontSmall:  11
    readonly property int   fontBody:   13
    readonly property int   fontLabel:  12
    readonly property int   fontHuge:   64

    // A monospace family that exists everywhere Qt does.
    readonly property string mono: "monospace"
}
