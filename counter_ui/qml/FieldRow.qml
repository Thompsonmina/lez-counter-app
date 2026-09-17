import QtQuick
import QtQuick.Layouts

// One label/value line. `theme` is typed rather than `var` so a misspelled
// token is a lint error instead of a silent `undefined`.
RowLayout {
    id: root

    required property AppTheme theme
    required property string label
    property string value: ""
    property bool mono: false
    property bool elide: true

    spacing: theme.gap

    Text {
        Layout.preferredWidth: 96
        Layout.alignment: Qt.AlignTop
        text: root.label
        color: root.theme.textDim
        font.pixelSize: root.theme.fontLabel
    }

    Text {
        Layout.fillWidth: true
        text: root.value.length > 0 ? root.value : "—"
        color: root.theme.text
        font.pixelSize: root.theme.fontLabel
        font.family: root.mono ? root.theme.mono : font.family
        elide: root.elide ? Text.ElideMiddle : Text.ElideNone
        wrapMode: root.elide ? Text.NoWrap : Text.WrapAnywhere
        maximumLineCount: root.elide ? 1 : 3
    }
}
