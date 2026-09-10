import QtQuick
import qs.Ui

BarWidget {
  id: root
  moduleName: "org.tide.launcher"

  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  BarIconButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    text: "\uf121"
    tooltipText: "Open TIDE"
    onPressed: root.bar.run("omarchy launch or focus tui --app-id=org.tide.TIDE tide")
  }
}
