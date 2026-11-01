import QtQml
import Host.Devices 1.0

Device {
    serial: "unwritable"
    settings.missing: 7
    property int bad: sample(1, 2)
}
