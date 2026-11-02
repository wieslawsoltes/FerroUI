import QtQml
QtObject {
    id: host
    property int base: 3
    property Component factory: Component {
        QtObject {
            required property int amount
            property int seen
            property list<int> numbers: []
            property var payload
            property var map
            Component.onCompleted: seen = amount
        }
    }
    function probe(): string {
        const data = {tag:"initial"}
        const map = new Map()
        const obj = factory.createObject(host, {amount:Qt.binding(function(){return host.base + 4}), numbers:[5,9], payload:data, map:map})
        base = 11
        return [obj.seen,obj.amount,obj.payload === data,obj.map === map,obj.numbers.join(",")].join("/")
    }
}
