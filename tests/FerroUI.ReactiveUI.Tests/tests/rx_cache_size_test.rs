//! `RxCacheSizeTest` (`ReactiveUI.Tests/RxCacheSizeTest.cs`).

use ferroui_reactiveui::*;

const CONFIGURED_SMALL_LIMIT: usize = 128;
const CONFIGURED_BIG_LIMIT: usize = 512;

// Not ported: Limits_ConcurrentFirstRead_NeverReturnZero - real threads

#[test]
fn initialize_before_first_read_is_returned() {
    RxCacheSize::reset_for_testing();

    RxCacheSize::initialize(CONFIGURED_SMALL_LIMIT, CONFIGURED_BIG_LIMIT);

    assert_eq!(RxCacheSize::small_cache_limit(), CONFIGURED_SMALL_LIMIT);
    assert_eq!(RxCacheSize::big_cache_limit(), CONFIGURED_BIG_LIMIT);
    RxCacheSize::reset_for_testing();
}

#[test]
fn initialize_after_first_read_keeps_first_limits() {
    RxCacheSize::reset_for_testing();
    let small = RxCacheSize::small_cache_limit();
    let big = RxCacheSize::big_cache_limit();

    RxCacheSize::initialize(CONFIGURED_SMALL_LIMIT, CONFIGURED_BIG_LIMIT);

    assert_eq!(RxCacheSize::small_cache_limit(), small);
    assert_eq!(RxCacheSize::big_cache_limit(), big);
    RxCacheSize::reset_for_testing();
}
