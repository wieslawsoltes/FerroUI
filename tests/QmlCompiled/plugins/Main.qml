import Workshop.Gauges 1.2 as G
G.Dial {
    value: 21
    property G.Gauge other: G.Gauge { value: 3 }
    signal sample(gauge: G.Gauge)
    function echo(gauge: G.Gauge): G.Gauge { return gauge }
    property int total: reading + other.value
}
