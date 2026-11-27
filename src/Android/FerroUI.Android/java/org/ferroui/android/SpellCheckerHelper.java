package org.ferroui.android;

import android.content.Context;
import android.content.Intent;
import android.view.textservice.SentenceSuggestionsInfo;
import android.view.textservice.SpellCheckerSession;
import android.view.textservice.SuggestionsInfo;
import android.view.textservice.TextInfo;
import android.view.textservice.TextServicesManager;

import java.util.ArrayList;
import java.util.Locale;

/**
 * A session of the spell checker of the system for the spell checker of the native side: opens
 * the session, sends it the texts to check, and gives the native side the misspelled words of each
 * result with their suggestions. The session answers on the looper of the thread that opened it,
 * the UI thread, so the native side receives every result there.
 *
 * The calls into the system can throw; each method catches what its call throws and answers with
 * null or false, which the native side takes as "no result".
 */
final class SpellCheckerHelper implements SpellCheckerSession.SpellCheckerSessionListener {
    /**
     * {@code Settings.ACTION_USER_DICTIONARY_INSERT}: the dialog of the settings that adds a word
     * to the user's dictionary, which the text views of the system open for "Add to dictionary".
     */
    private static final String ACTION_USER_DICTIONARY_INSERT = "com.android.settings.USER_DICTIONARY_INSERT";

    private final long nativeHandle;
    private SpellCheckerSession session;

    private SpellCheckerHelper(long nativeHandle) {
        this.nativeHandle = nativeHandle;
    }

    /**
     * Opens a session for a language (a BCP 47 tag), or for the language of the settings of the
     * spell checker when the tag is null or the spell checker has no dictionary for it. Null when
     * the system has no spell checker or the user turned it off.
     */
    static SpellCheckerHelper open(Context context, String languageTag, long nativeHandle) {
        try {
            TextServicesManager manager =
                    (TextServicesManager) context.getSystemService(Context.TEXT_SERVICES_MANAGER_SERVICE);
            if (manager == null) {
                return null;
            }
            SpellCheckerHelper helper = new SpellCheckerHelper(nativeHandle);
            if (languageTag != null) {
                helper.session = newSession(manager, Locale.forLanguageTag(languageTag), helper);
            }
            if (helper.session == null) {
                helper.session = newSession(manager, null, helper);
            }
            return helper.session == null ? null : helper;
        } catch (RuntimeException e) {
            return null;
        }
    }

    /**
     * A session whose results come on the looper of this thread. Without a locale it checks in the
     * language of the settings of the spell checker.
     */
    private static SpellCheckerSession newSession(TextServicesManager manager, Locale locale,
            SpellCheckerHelper listener) {
        return manager.newSpellCheckerSession(null, locale, listener, locale == null);
    }

    /**
     * Sends a text to the session; its result comes back with the sequence number. False when the
     * text could not be sent (there is no result then).
     */
    boolean check(String text, int sequence, int suggestionsLimit) {
        try {
            TextInfo[] texts = { new TextInfo(text, 0, sequence) };
            session.getSentenceSuggestions(texts, suggestionsLimit);
            return true;
        } catch (RuntimeException e) {
            return false;
        }
    }

    /** Closes the session; it sends no more results. */
    void close() {
        try {
            session.close();
        } catch (RuntimeException e) {
            // The session is gone either way.
        }
    }

    /**
     * Opens the dialog of the settings that adds a word to the user's dictionary (an application
     * cannot write that dictionary itself). False when the system has no such dialog.
     */
    static boolean addToDictionary(Context context, String word) {
        try {
            Intent intent = new Intent(ACTION_USER_DICTIONARY_INSERT);
            intent.putExtra("word", word);
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            context.startActivity(intent);
            return true;
        } catch (RuntimeException e) {
            // ActivityNotFoundException among them.
            return false;
        }
    }

    @Override
    public void onGetSuggestions(SuggestionsInfo[] results) {
        // Only sentences are checked.
    }

    /**
     * The result of a text: the words that look like typos, as pairs of offset and length in the
     * text, the number of suggestions of each, and the suggestions one after the other. The
     * sequence number is that of the text, or -1 when the result has no words to tell it by.
     */
    @Override
    public void onGetSentenceSuggestions(SentenceSuggestionsInfo[] results) {
        int sequence = -1;
        ArrayList<Integer> ranges = new ArrayList<>();
        ArrayList<Integer> counts = new ArrayList<>();
        ArrayList<String> suggestions = new ArrayList<>();
        if (results != null) {
            for (SentenceSuggestionsInfo sentence : results) {
                if (sentence == null) {
                    continue;
                }
                for (int i = 0; i < sentence.getSuggestionsCount(); i++) {
                    SuggestionsInfo info = sentence.getSuggestionsInfoAt(i);
                    if (info == null) {
                        continue;
                    }
                    if (sequence == -1) {
                        sequence = info.getSequence();
                    }
                    if ((info.getSuggestionsAttributes() & SuggestionsInfo.RESULT_ATTR_LOOKS_LIKE_TYPO) == 0) {
                        continue;
                    }
                    ranges.add(sentence.getOffsetAt(i));
                    ranges.add(sentence.getLengthAt(i));
                    int count = 0;
                    for (int j = 0; j < info.getSuggestionsCount(); j++) {
                        String suggestion = info.getSuggestionAt(j);
                        if (suggestion != null) {
                            suggestions.add(suggestion);
                            count++;
                        }
                    }
                    counts.add(count);
                }
            }
        }
        nativeOnSentenceSuggestions(nativeHandle, sequence, toArray(ranges), toArray(counts),
                suggestions.toArray(new String[0]));
    }

    private static int[] toArray(ArrayList<Integer> values) {
        int[] array = new int[values.size()];
        for (int i = 0; i < array.length; i++) {
            array[i] = values.get(i);
        }
        return array;
    }

    private static native void nativeOnSentenceSuggestions(long handle, int sequence, int[] ranges,
            int[] suggestionCounts, String[] suggestions);
}
