package org.ferroui.android;

import android.os.Bundle;
import android.os.Handler;
import android.view.KeyEvent;
import android.view.inputmethod.CompletionInfo;
import android.view.inputmethod.CorrectionInfo;
import android.view.inputmethod.ExtractedText;
import android.view.inputmethod.ExtractedTextRequest;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputContentInfo;

/**
 * The connection the input method of the system edits the text of the framework through.
 *
 * The class forwards the calls to the native side, which holds the logic: the text and the
 * selection are those of the text box that has the focus. What the native side answers with a
 * constant for every argument is answered here.
 */
final class FerroInputConnection implements InputConnection {
    private final long nativeHandle;

    FerroInputConnection(long nativeHandle) {
        this.nativeHandle = nativeHandle;
    }

    private static String text(CharSequence text) {
        return text != null ? text.subSequence(0, text.length()).toString() : null;
    }

    @Override
    public boolean setComposingRegion(int start, int end) {
        return nativeSetComposingRegion(nativeHandle, start, end);
    }

    @Override
    public boolean setComposingText(CharSequence text, int newCursorPosition) {
        return nativeSetComposingText(nativeHandle, text(text), newCursorPosition);
    }

    @Override
    public boolean setSelection(int start, int end) {
        return nativeSetSelection(nativeHandle, start, end);
    }

    @Override
    public boolean beginBatchEdit() {
        return nativeBeginBatchEdit(nativeHandle);
    }

    @Override
    public boolean endBatchEdit() {
        return nativeEndBatchEdit(nativeHandle);
    }

    @Override
    public boolean commitText(CharSequence text, int newCursorPosition) {
        return nativeCommitText(nativeHandle, text(text), newCursorPosition);
    }

    @Override
    public boolean deleteSurroundingText(int beforeLength, int afterLength) {
        return nativeDeleteSurroundingText(nativeHandle, beforeLength, afterLength);
    }

    @Override
    public boolean performEditorAction(int actionCode) {
        return nativePerformEditorAction(nativeHandle, actionCode);
    }

    @Override
    public ExtractedText getExtractedText(ExtractedTextRequest request, int flags) {
        return nativeGetExtractedText(nativeHandle, request != null, request != null ? request.token : 0, flags);
    }

    @Override
    public boolean performContextMenuAction(int id) {
        return nativePerformContextMenuAction(nativeHandle, id);
    }

    @Override
    public boolean clearMetaKeyStates(int states) {
        return false;
    }

    @Override
    public void closeConnection() {
        nativeCloseConnection(nativeHandle);
    }

    @Override
    public boolean commitCompletion(CompletionInfo text) {
        return false;
    }

    @Override
    public boolean commitContent(InputContentInfo inputContentInfo, int flags, Bundle opts) {
        return false;
    }

    @Override
    public boolean commitCorrection(CorrectionInfo correctionInfo) {
        return false;
    }

    @Override
    public boolean deleteSurroundingTextInCodePoints(int beforeLength, int afterLength) {
        return nativeDeleteSurroundingTextInCodePoints(nativeHandle, beforeLength, afterLength);
    }

    @Override
    public boolean finishComposingText() {
        return nativeFinishComposingText(nativeHandle);
    }

    @Override
    public int getCursorCapsMode(int reqModes) {
        return nativeGetCursorCapsMode(nativeHandle, reqModes);
    }

    @Override
    public CharSequence getSelectedText(int flags) {
        return nativeGetSelectedText(nativeHandle, flags);
    }

    @Override
    public CharSequence getTextAfterCursor(int n, int flags) {
        return nativeGetTextAfterCursor(nativeHandle, n, flags);
    }

    @Override
    public CharSequence getTextBeforeCursor(int n, int flags) {
        return nativeGetTextBeforeCursor(nativeHandle, n, flags);
    }

    @Override
    public boolean performPrivateCommand(String action, Bundle data) {
        return false;
    }

    @Override
    public boolean reportFullscreenMode(boolean enabled) {
        return false;
    }

    @Override
    public boolean requestCursorUpdates(int cursorUpdateMode) {
        return false;
    }

    @Override
    public boolean sendKeyEvent(KeyEvent event) {
        return nativeSendKeyEvent(nativeHandle, event);
    }

    @Override
    public Handler getHandler() {
        return null;
    }

    private static native boolean nativeSetComposingRegion(long handle, int start, int end);

    private static native boolean nativeSetComposingText(long handle, String text, int newCursorPosition);

    private static native boolean nativeSetSelection(long handle, int start, int end);

    private static native boolean nativeBeginBatchEdit(long handle);

    private static native boolean nativeEndBatchEdit(long handle);

    private static native boolean nativeCommitText(long handle, String text, int newCursorPosition);

    private static native boolean nativeDeleteSurroundingText(long handle, int beforeLength, int afterLength);

    private static native boolean nativePerformEditorAction(long handle, int actionCode);

    private static native ExtractedText nativeGetExtractedText(long handle, boolean hasRequest, int token, int flags);

    private static native boolean nativePerformContextMenuAction(long handle, int id);

    private static native void nativeCloseConnection(long handle);

    private static native boolean nativeDeleteSurroundingTextInCodePoints(long handle, int beforeLength, int afterLength);

    private static native boolean nativeFinishComposingText(long handle);

    private static native int nativeGetCursorCapsMode(long handle, int reqModes);

    private static native String nativeGetSelectedText(long handle, int flags);

    private static native String nativeGetTextAfterCursor(long handle, int n, int flags);

    private static native String nativeGetTextBeforeCursor(long handle, int n, int flags);

    private static native boolean nativeSendKeyEvent(long handle, KeyEvent event);
}
