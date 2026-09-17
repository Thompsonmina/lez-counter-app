import QtQuick

// A labelled on/off switch, drawn from primitives for the same reason as
// ActionButton: the host's Qt style should not get a vote.
Item {
    id: root

    property string text: ""
    required property AppTheme theme
    property bool checked: false

    signal toggled(bool value)

    implicitWidth: track.width + 8 + label.implicitWidth
    implicitHeight: 34

    Rectangle {
        id: track
        width: 34
        height: 20
        radius: 10
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        color: root.checked ? root.theme.accent : root.theme.panelAlt
        border.width: 1
        border.color: root.checked ? root.theme.accent : root.theme.border
        Behavior on color { ColorAnimation { duration: 120 } }

        Rectangle {
            width: 14
            height: 14
            radius: 7
            y: 3
            x: root.checked ? track.width - width - 3 : 3
            color: root.checked ? (root.theme.dark ? "#0f1115" : "#ffffff") : root.theme.textDim
            Behavior on x { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
        }
    }

    Text {
        id: label
        anchors.left: track.right
        anchors.leftMargin: 8
        anchors.verticalCenter: parent.verticalCenter
        text: root.text
        color: root.checked ? root.theme.text : root.theme.textDim
        font.pixelSize: root.theme.fontLabel
    }

    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        // Reports intent only. Assigning `root.checked` here would overwrite
        // the incoming binding (`checked: reader.autoRefresh`) with a static
        // value, after which the chip and the thing it describes can disagree
        // with nothing to signal it. The owner flips the source of truth and
        // the binding brings the new state back.
        onClicked: root.toggled(!root.checked)
    }
}
