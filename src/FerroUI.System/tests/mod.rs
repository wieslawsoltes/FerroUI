//! The unit tests of the crate, as a module tree of the library.

#[cfg(any(ferro_tls_table, target_os = "android", target_os = "linux", target_os = "macos"))]
mod ferro_thread_local_tests;
mod task_tests;
mod executor_tests;
mod oneshot_tests;
mod shared_tests;
mod cancellation_tests;
mod async_ordering_tests;
mod synchronization_context_tests;
mod slot_order_map_tests;
mod property_path_tests;
mod observable_collection_tests;
mod subscription_tests;

mod observable_collection_untyped_tests;
