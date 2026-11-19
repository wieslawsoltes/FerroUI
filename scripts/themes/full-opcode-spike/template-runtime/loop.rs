    // Count only in separately launched, interposed processes. Warm the actual deferred factory,
    // its property/type caches, and the allocator before either measurement.
    let iterations = std::env::var("TEMPLATE_ITERATIONS").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(8192);
    assert!(iterations > 0);
    for _ in 0..512 { make(); }
    #[repr(C)]
    #[derive(Default)]
    struct Counts { allocations: u64, reallocations: u64, frees: u64, requested_bytes: u64 }
    extern "C" { fn dlsym(handle: *mut std::ffi::c_void, symbol: *const i8) -> *mut std::ffi::c_void; }
    let hooks = unsafe {
        let handle = (-2_isize) as *mut std::ffi::c_void; // RTLD_DEFAULT on macOS
        let begin = dlsym(handle, b"template_probe_begin\0".as_ptr().cast());
        let end = dlsym(handle, b"template_probe_end\0".as_ptr().cast());
        if !begin.is_null() && !end.is_null() {
            Some((std::mem::transmute::<_, unsafe extern "C" fn()>(begin),
                  std::mem::transmute::<_, unsafe extern "C" fn(*mut Counts)>(end)))
        } else { None }
    };
    let mut totals = Counts::default();
    let start = Instant::now();
    if let Some((begin, _)) = hooks { unsafe { begin() }; }
    for _ in 0..iterations { make(); }
    if let Some((_, end)) = hooks { unsafe { end(&mut totals) }; }
    let elapsed = start.elapsed().as_nanos();
    let repeated_ns = elapsed / iterations as u128;
    eprintln!("template_probe {iterations} {elapsed} {} {} {} {}", totals.allocations,
        totals.reallocations, totals.frees, totals.requested_bytes);
