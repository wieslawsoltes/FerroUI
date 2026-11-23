package org.ferroui.webview;

import android.annotation.SuppressLint;
import android.content.Context;
import android.graphics.Bitmap;
import android.net.http.SslError;
import android.os.Handler;
import android.os.Looper;
import android.view.View;
import android.webkit.JavascriptInterface;
import android.webkit.SslErrorHandler;
import android.webkit.WebBackForwardList;
import android.webkit.WebChromeClient;
import android.webkit.WebResourceError;
import android.webkit.WebResourceRequest;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;

/** The system web view of one web view control; every method runs on the main thread. */
public final class WebViewEngine {
    /** {@code WebViewClient.ERROR_FAILED_SSL_HANDSHAKE}, for a certificate the page failed on. */
    private static final int ERROR_CERTIFICATE = WebViewClient.ERROR_FAILED_SSL_HANDSHAKE;

    private final WebView view;
    private final Handler main = new Handler(Looper.getMainLooper());
    private final String documentScript;
    private long owner;
    private long nextNavigation = 1;
    private long navigation;
    private boolean isCompleted = true;

    @SuppressLint({"SetJavaScriptEnabled", "AddJavascriptInterface"})
    public WebViewEngine(Context context, long owner, String documentScript) {
        this.owner = owner;
        this.documentScript = documentScript;
        view = new WebView(context);
        WebSettings settings = view.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        settings.setAllowFileAccess(false);
        settings.setAllowContentAccess(false);
        view.addJavascriptInterface(new Host(), "ferrouiWebViewHost");
        view.setWebViewClient(new Client());
        view.setWebChromeClient(new ChromeClient());
    }

    public View view() {
        return view;
    }

    /** Announces a navigation the application asked for; false when a handler canceled it. */
    private boolean announce(String uri) {
        long id = nextNavigation++;
        if (owner == 0 || !nativeNavigationStarting(owner, id, uri, false, false)) {
            return false;
        }
        // A handler can close the engine while it is asked.
        if (owner == 0) {
            return false;
        }
        navigation = id;
        isCompleted = false;
        return true;
    }

    public void navigate(String uri) {
        if (announce(uri)) {
            view.loadUrl(uri);
        }
    }

    public void navigateToString(String html) {
        if (announce("about:blank")) {
            view.loadDataWithBaseURL(null, html, "text/html", "UTF-8", null);
        }
    }

    public void reload() {
        String uri = view.getUrl();
        if (announce(uri == null ? "about:blank" : uri)) {
            view.reload();
        }
    }

    public void stop() {
        view.stopLoading();
    }

    private String historyEntry(int offset) {
        WebBackForwardList list = view.copyBackForwardList();
        int index = list.getCurrentIndex() + offset;
        if (index < 0 || index >= list.getSize()) {
            return null;
        }
        return list.getItemAtIndex(index).getUrl();
    }

    public void goBack() {
        String uri = historyEntry(-1);
        if (uri != null && announce(uri)) {
            view.goBack();
        }
    }

    public void goForward() {
        String uri = historyEntry(1);
        if (uri != null && announce(uri)) {
            view.goForward();
        }
    }

    public void executeScript(final long id, String script) {
        view.evaluateJavascript(script, result -> {
            if (owner != 0) {
                nativeScriptCompleted(owner, id, result);
            }
        });
    }

    public void evaluate(String script) {
        view.evaluateJavascript(script, null);
    }

    /** After this no callback reaches the owner. */
    public void close() {
        owner = 0;
        main.removeCallbacksAndMessages(null);
        view.stopLoading();
        view.setWebViewClient(new WebViewClient());
        view.setWebChromeClient(null);
        view.removeJavascriptInterface("ferrouiWebViewHost");
        view.destroy();
    }

    private void complete(boolean isSuccess, int error) {
        if (isCompleted || owner == 0) {
            return;
        }
        isCompleted = true;
        nativeNavigationCompleted(owner, navigation, isSuccess, error);
    }

    private void reportHistory() {
        if (owner != 0) {
            nativeHistoryChanged(owner, view.canGoBack(), view.canGoForward());
        }
    }

    private final class Host {
        @JavascriptInterface
        public void postMessage(final String json) {
            // Called on a thread of the web view.
            main.post(() -> {
                if (owner != 0) {
                    String source = view.getUrl();
                    nativeWebMessage(owner, source == null ? "" : source, json);
                }
            });
        }
    }

    private final class Client extends WebViewClient {
        @Override
        public boolean shouldOverrideUrlLoading(WebView webView, WebResourceRequest request) {
            if (owner == 0 || !request.isForMainFrame()) {
                return false;
            }
            boolean isRedirected = request.isRedirect();
            long id = isRedirected ? navigation : nextNavigation++;
            boolean allow = nativeNavigationStarting(
                    owner, id, request.getUrl().toString(), request.hasGesture(), isRedirected);
            if (allow && owner != 0) {
                navigation = id;
                isCompleted = false;
            }
            return !allow;
        }

        @Override
        public void onPageStarted(WebView webView, String url, Bitmap favicon) {
            webView.evaluateJavascript(documentScript, null);
            if (owner != 0 && url != null) {
                nativeSourceChanged(owner, url);
            }
        }

        @Override
        public void doUpdateVisitedHistory(WebView webView, String url, boolean isReload) {
            if (owner != 0 && url != null) {
                nativeSourceChanged(owner, url);
            }
            reportHistory();
        }

        @Override
        public void onPageFinished(WebView webView, String url) {
            reportHistory();
            complete(true, 0);
        }

        @Override
        public void onReceivedError(WebView webView, WebResourceRequest request, WebResourceError error) {
            if (request.isForMainFrame()) {
                complete(false, error.getErrorCode());
            }
        }

        @Override
        public void onReceivedSslError(WebView webView, SslErrorHandler handler, SslError error) {
            handler.cancel();
            complete(false, ERROR_CERTIFICATE);
        }
    }

    private final class ChromeClient extends WebChromeClient {
        @Override
        public void onReceivedTitle(WebView webView, String title) {
            if (owner != 0) {
                nativeTitleChanged(owner, title == null ? "" : title);
            }
        }
    }

    private static native boolean nativeNavigationStarting(
            long owner, long id, String uri, boolean isUserInitiated, boolean isRedirected);

    private static native void nativeNavigationCompleted(long owner, long id, boolean isSuccess, int error);

    private static native void nativeSourceChanged(long owner, String uri);

    private static native void nativeHistoryChanged(long owner, boolean canGoBack, boolean canGoForward);

    private static native void nativeTitleChanged(long owner, String title);

    private static native void nativeWebMessage(long owner, String source, String json);

    private static native void nativeScriptCompleted(long owner, long id, String result);
}
