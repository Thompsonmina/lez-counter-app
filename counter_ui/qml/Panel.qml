import QtQuick
import QtQuick.Layouts

// A surface that sizes itself to a column of content. Children go straight in —
// `content` is the default property, so a Panel reads like any other container
// at the call site — and the column's implicitHeight drives the panel's, so a
// panel is never taller or shorter than what it holds.
Rectangle {
    id: root

    required property AppTheme theme
    property int padding: theme.pad

    default property alias content: col.data
    property alias spacing: col.spacing

    color: theme.panel
    border.color: theme.border
    border.width: 1
    radius: theme.radius
    implicitHeight: col.implicitHeight + padding * 2

    ColumnLayout {
        id: col
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: root.padding
        spacing: root.theme.gap
    }
}
