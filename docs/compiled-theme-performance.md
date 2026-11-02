# Compiled theme performance

Compare the same XAML resources and consumer on the same Rust toolchain and release profile. Record generated Rust, binary assets, stripped executable bytes, build wall time and first resource access separately. Record which dependencies were warm for build timings. A full-theme consumer and a small consumer expose different reachability costs.

The existing compiled-theme API is the compatibility baseline. Resource lookup order, deferred references, template construction and dynamic lookup must remain observable through their normal entry points. External binary assets need version and bounds checks before any update becomes visible to a consumer.
