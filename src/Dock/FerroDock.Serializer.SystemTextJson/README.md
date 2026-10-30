# FerroDock JSON serializer

The resolver-based DockSerializer overload implements the original generic serialization
pipeline. Supply compiled JsonTypeInfo descriptors with GeneratedJsonTypeInfoResolver.
The serializer preserves original class references, null/failure distinctions and stream
disposal. Core operations return SerializationResult following the FerroUI porting guide.

Default model discovery, original source-generation emitter, configured list factories,
polymorphic interfaces and the full primitive/runtime surface are not yet exposed. This
library is partial; it does not yet roundtrip the original sample layout. The remaining
requirements and source evidence are recorded in ../../../docs/porting/dock/SERIALIZATION.md.
