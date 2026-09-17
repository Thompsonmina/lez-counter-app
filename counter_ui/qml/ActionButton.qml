import QtQuick

// A button drawn from primitives rather than QtQuick.Controls.
//
// The host ships its own Qt style, so a Controls Button renders as whatever
// Basecamp's style says — which in practice was a light chip on a dark panel.
// Owning the pixels is a handful of lines and removes the whole class of
// surprise, along with the Controls import.
Rectangle {
    id: root

    property string text: ""
    required property AppTheme theme
    property bool primary: false
    // `enabled` is Item's own — redeclaring it here would shadow the built-in
    // and quietly stop propagating to the MouseArea.

    signal clicked()

    implicitWidth: label.implicitWidth + theme.pad * 2
    implicitHeight: 34
    radius: 8

    color: !enabled ? "transparent"
         : primary ? (mouse.pressed ? Qt.darker(theme.accent, 1.25)
                     : mouse.containsMouse ? Qt.lighter(theme.accent, 1.12) : theme.accent)
         : (mouse.pressed ? theme.panelAlt
            : mouse.containsMouse ? Qt.lighter(theme.panelAlt, 1.15) : "transparent")

    border.width: primary ? 0 : 1
    border.color: enabled ? theme.border : Qt.darker(theme.border, 1.2)
    opacity: enabled ? 1.0 : 0.45

    Behavior on color { ColorAnimation { duration: 90 } }

    Text {
        id: label
        anchors.centerIn: parent
        text: root.text
        color: root.primary ? (root.theme.dark ? "#0f1115" : "#ffffff") : root.theme.text
        font.pixelSize: root.theme.fontLabel
        font.bold: root.primary
    }

    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        enabled: root.enabled
        cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }
}
