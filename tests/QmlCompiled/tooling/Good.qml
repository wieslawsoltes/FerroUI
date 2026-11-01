import QtQml
import QtQml.Models
import Host.Devices 2.2
import Host.Plant
import Host.Shared 2.2 as Shared

Sensor {
    id: sensor
    level: Device.High
    temperature: 19.5
    settings.gain: 2.5
    settings.scale: 1.5
    property int sampled: sample(level)
    property string label: serial
    property ListModel sourceModel: model
    property Shared.Device sharedDevice: sensor
    onLevelChanged: sensor.objectName = "changed"
    onReported: (message) => { sensor.objectName = message }
    Station { device: sensor }
}
