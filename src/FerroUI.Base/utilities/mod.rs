//! Internal and public helper types.

mod boolean_boxes;
mod byte_size_helper;
mod handler_list;
mod i_weak_event_subscriber;
mod weak_event;
mod weak_event_handler_manager;
mod weak_events;
mod weak_hash_list;
mod synchronous_completion_async_result;

pub use boolean_boxes::BooleanBoxes;
pub use byte_size_helper::ByteSizeHelper;
pub use handler_list::HandlerList;
pub use i_weak_event_subscriber::{
    IWeakEventSubscriber, TargetWeakEventSubscriber, WeakEventSubscriber, WeakEventSubscriberHandler,
};
pub use weak_event::{WeakEvent, WeakEventArgs, WeakEventHandler, WeakEventSender};
pub use weak_event_handler_manager::{WeakEventHandlerManager, WeakEventHandlerTarget};
pub use weak_events::WeakEvents;
pub use weak_hash_list::WeakHashList;
pub use synchronous_completion_async_result::{
    SynchronousCompletionAsyncResult, SynchronousCompletionAsyncResultSource,
};

pub mod math_utilities;
pub mod span_helpers;
pub mod span_string_tokenizer;
pub mod spring_solver;

pub use math_utilities::MathUtilities;
pub use span_string_tokenizer::SpanStringTokenizer as StringTokenizer;
pub use spring_solver::SpringSolver;
pub use span_string_tokenizer::{FormatError, SpanStringTokenizer};

mod uri;

pub use uri::{Uri, UriFormatError, UriKind};

mod ref_countable;
mod uri_extensions;

pub use ref_countable::{RefCountable, RefCounted};
pub use uri_extensions::UriExtensions;

mod cancel_event_args;
mod event_args;
pub mod character_reader;
pub mod identifier_parser;
pub mod keyword_parser;
pub mod style_class_parser;
pub use cancel_event_args::CancelEventArgs;
pub use event_args::EventArgs;
pub use character_reader::CharacterReader;

// --- text support ---

mod array_builder;
mod array_slice;
mod culture_info;
mod read_only_memory;
pub(crate) mod span;
mod value_span;

pub use array_builder::ArrayBuilder;
pub use array_slice::ArraySlice;
pub use culture_info::CultureInfo;
mod compare_info;
pub use compare_info::{CompareInfo, CompareOptions, ICompareRules, StringComparison};
pub use read_only_memory::ReadOnlyMemory;
pub use value_span::ValueSpan;

// --- collections ---

mod bidi_dictionary;
mod binary_search_extension;
mod ferro_property_dictionary;
mod mapped_array_slice;
mod object_pool;
mod pooled_inline_list;
mod ref_counting_small_dictionary;
mod ref_tracking_dictionary;
mod single_or_queue;
mod small_dictionary;
mod value_single_or_list;

#[cfg(test)]
mod string_splitter_tests;

pub use bidi_dictionary::BidiDictionary;
pub use binary_search_extension::BinarySearchExtension;
pub use ferro_property_dictionary::FerroPropertyDictionary;
pub use mapped_array_slice::MappedArraySlice;
pub use object_pool::ObjectPool;
pub use pooled_inline_list::{PooledInlineList, PooledInlineListEnumerator, PooledInlineListRawState};
pub use ref_counting_small_dictionary::RefCountingSmallDictionary;
pub use ref_tracking_dictionary::RefTrackingDictionary;
pub use single_or_queue::SingleOrQueue;
pub use small_dictionary::{InlineDictionary, InlineDictionaryEnumerator};
pub use value_single_or_list::ValueSingleOrList;

// --- date and time ---

mod date_time;
mod date_time_format;
mod date_time_format_info;
mod date_time_kind;
mod date_time_offset;
mod date_time_styles;
mod day_of_week;
mod gregorian_calendar;
mod i_culture_data_provider;
#[cfg(any(test, feature = "testing"))]
mod test_culture_data_provider;
mod text_info;
mod time_zone_info;

#[cfg(test)]
mod date_time_net_data;
#[cfg(test)]
mod date_time_net_tests;
#[cfg(test)]
mod date_time_tests;

pub use date_time::{DateTime, UtcNowProvider};
pub use date_time_format_info::{CalendarWeekRule, DateTimeFormatInfo, DateTimeFormatInfoRef, DateTimeFormatProvider};
pub use date_time_kind::DateTimeKind;
pub use date_time_offset::DateTimeOffset;
pub use date_time_styles::DateTimeStyles;
pub use day_of_week::DayOfWeek;
pub use gregorian_calendar::{GregorianCalendar, IsoWeek};
pub use i_culture_data_provider::ICultureDataProvider;
#[cfg(any(test, feature = "testing"))]
pub use test_culture_data_provider::TestCultureDataProvider;
pub use text_info::TextInfo;
pub use time_zone_info::{LocalUtcOffsetProvider, TimeZoneInfo};

// --- numbers ---

mod decimal;
pub mod number_format;
mod number_format_info;
mod number_styles;

#[cfg(test)]
mod number_format_net_tests;

pub use decimal::Decimal;
pub use number_format::NumberParseError;
pub use number_format_info::NumberFormatInfo;
pub use number_styles::NumberStyles;
