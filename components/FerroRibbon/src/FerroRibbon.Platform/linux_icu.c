// Resolve ICU's entry points through its public headers on Linux. The
// installed headers select the symbol names for that distribution's ICU.
#include <unicode/ucol.h>
#include <unicode/unum.h>
#include <unicode/usearch.h>
#include <unicode/uversion.h>

UCollator *fr_icu_ucol_open(const char *locale, UErrorCode *status) {
    return ucol_open(locale, status);
}

void fr_icu_ucol_close(UCollator *collator) {
    ucol_close(collator);
}

const UChar *fr_icu_ucol_getRules(const UCollator *collator, int32_t *length) {
    return ucol_getRules(collator, length);
}

UCollator *fr_icu_ucol_openRules(const UChar *rules, int32_t length,
                                UColAttributeValue normalization, UCollationStrength strength,
                                UParseError *parse_error, UErrorCode *status) {
    return ucol_openRules(rules, length, normalization, strength, parse_error, status);
}

UCollationResult fr_icu_ucol_strcoll(const UCollator *collator, const UChar *left,
                                    int32_t left_length, const UChar *right,
                                    int32_t right_length) {
    return ucol_strcoll(collator, left, left_length, right, right_length);
}

UStringSearch *fr_icu_usearch_openFromCollator(const UChar *pattern, int32_t pattern_length,
                                             const UChar *text, int32_t text_length,
                                             const UCollator *collator,
                                             UBreakIterator *break_iterator,
                                             UErrorCode *status) {
    return usearch_openFromCollator(pattern, pattern_length, text, text_length, collator,
                                  break_iterator, status);
}

int32_t fr_icu_usearch_first(UStringSearch *search, UErrorCode *status) {
    return usearch_first(search, status);
}

void fr_icu_usearch_close(UStringSearch *search) {
    usearch_close(search);
}

void fr_icu_u_getVersion(UVersionInfo version) {
    u_getVersion(version);
}

UNumberFormat *fr_icu_unum_open(UNumberFormatStyle style, const UChar *pattern, int32_t length,
                              const char *locale, UParseError *parse_error, UErrorCode *status) {
    return unum_open(style, pattern, length, locale, parse_error, status);
}

void fr_icu_unum_close(UNumberFormat *format) {
    unum_close(format);
}

int32_t fr_icu_unum_getAttribute(const UNumberFormat *format, UNumberFormatAttribute attribute) {
    return unum_getAttribute(format, attribute);
}

int32_t fr_icu_unum_getSymbol(const UNumberFormat *format, UNumberFormatSymbol symbol,
                             UChar *buffer, int32_t capacity, UErrorCode *status) {
    return unum_getSymbol(format, symbol, buffer, capacity, status);
}

int32_t fr_icu_unum_toPattern(const UNumberFormat *format, UBool localized,
                             UChar *buffer, int32_t capacity, UErrorCode *status) {
    return unum_toPattern(format, localized, buffer, capacity, status);
}
