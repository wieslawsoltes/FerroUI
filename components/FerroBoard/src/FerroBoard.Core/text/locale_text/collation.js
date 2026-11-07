// The browser's locale-aware comparison for FerroBoard's LocaleText: Intl.Collator and
// Intl.Segmenter of the JavaScript engine, which carry the browser's own ICU and CLDR data.
//
// A collator of FerroBoard compares at the tertiary strength (case and accents count) and, when
// case is ignored, at the secondary strength (accents count): Intl's sensitivities "variant" and
// "accent". Strings cross from WebAssembly as UTF-16 code units and are rebuilt unit for unit, lone
// surrogates included.

// The root collation. Intl resolves "und", an empty list and a locale it does not support to the
// default locale of the browser, not to root; CLDR's English uses the root collation unchanged, so
// root is asked for as "en".
const ROOT = "en";

function text(units) {
    let result = "";
    for (let start = 0; start < units.length; start += 8192) {
        result += String.fromCharCode.apply(null, units.subarray(start, start + 8192));
    }
    return result;
}

// An ICU locale identifier ("en_US", "sr_Latn_RS") as a BCP 47 tag, resolved the way ICU resolves
// it: a locale the browser has no collation for, or that is not well formed, falls back to root.
function resolve(locale) {
    if (locale === "") return ROOT;
    const tag = locale.replace(/_/g, "-");
    try {
        return Intl.Collator.supportedLocalesOf([tag], { localeMatcher: "lookup" }).length > 0 ? tag : ROOT;
    } catch {
        return ROOT;
    }
}

export class BrowserCollator {
    constructor(locale) {
        const tag = resolve(locale);
        this.sensitive = new Intl.Collator(tag, { usage: "sort", sensitivity: "variant" });
        this.insensitive = new Intl.Collator(tag, { usage: "sort", sensitivity: "accent" });
        this.graphemes = new Intl.Segmenter(tag, { granularity: "grapheme" });
    }

    compare(left, right, ignoreCase) {
        return (ignoreCase ? this.insensitive : this.sensitive).compare(text(left), text(right));
    }

    // Whether a run of whole characters (grapheme clusters) of the text is equal to the pattern
    // under the collator: the matches of ICU's collation search, which uses the collation of the
    // locale (not CLDR's search collation) and never splits a character.
    contains(source, pattern, ignoreCase) {
        const collator = ignoreCase ? this.insensitive : this.sensitive;
        const haystack = text(source);
        const needle = text(pattern);
        const bounds = [];
        for (const segment of this.graphemes.segment(haystack)) bounds.push(segment.index);
        bounds.push(haystack.length);
        for (let start = 0; start < bounds.length - 1; start++) {
            for (let end = start + 1; end < bounds.length; end++) {
                if (collator.compare(haystack.slice(bounds[start], bounds[end]), needle) === 0) return true;
            }
        }
        return false;
    }
}

export function openCollator(locale) {
    return new BrowserCollator(locale);
}
