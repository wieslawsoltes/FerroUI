package org.ferroui.android;

import android.content.Context;
import android.graphics.Rect;
import android.os.Bundle;
import android.view.View;
import android.view.ViewParent;
import android.view.accessibility.AccessibilityEvent;
import android.view.accessibility.AccessibilityManager;
import android.view.accessibility.AccessibilityNodeInfo;
import android.view.accessibility.AccessibilityNodeProvider;

/**
 * The accessibility delegate of a {@link FerroView}: the automation peers of the framework as
 * virtual views of the accessibility tree, through the node provider of the platform.
 *
 * The native side says what a node has, performs the actions and keeps which virtual view has the
 * accessibility focus, the keyboard focus and the finger over it. This class does what is a call
 * into the view system: the node of the host view, the package, source and visible bounds of a
 * node, and the accessibility events of a virtual view.
 */
// obtain() of nodes and events, the bounds in the parent and the boolean checked state are
// deprecated on the newest systems and are what the systems from API 26 have.
@SuppressWarnings("deprecation")
final class FerroAccessHelper extends View.AccessibilityDelegate {
    /** The virtual view that stands for the host view itself. */
    static final int HOST_ID = View.NO_ID;
    /** No virtual view. */
    static final int INVALID_ID = Integer.MIN_VALUE;

    private final FerroView host;
    private final long nativeHandle;
    private final AccessibilityManager manager;

    private final AccessibilityNodeProvider provider = new AccessibilityNodeProvider() {
        @Override
        public AccessibilityNodeInfo createAccessibilityNodeInfo(int virtualViewId) {
            return virtualViewId == HOST_ID ? createNodeForHost() : createNodeForChild(virtualViewId);
        }

        @Override
        public boolean performAction(int virtualViewId, int action, Bundle arguments) {
            if (virtualViewId == HOST_ID) {
                return host.performAccessibilityAction(action, arguments);
            }
            CharSequence text = arguments != null
                    ? arguments.getCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE)
                    : null;
            return nativePerformAction(
                    nativeHandle, virtualViewId, action, arguments != null, text != null ? text.toString() : null);
        }

        @Override
        public AccessibilityNodeInfo findFocus(int focusType) {
            int focusedId = nativeFindFocus(nativeHandle, focusType);
            return focusedId == INVALID_ID ? null : createAccessibilityNodeInfo(focusedId);
        }
    };

    FerroAccessHelper(FerroView host, long nativeHandle) {
        this.host = host;
        this.nativeHandle = nativeHandle;
        this.manager = (AccessibilityManager) host.getContext().getSystemService(Context.ACCESSIBILITY_SERVICE);
    }

    @Override
    public AccessibilityNodeProvider getAccessibilityNodeProvider(View host) {
        return provider;
    }

    private AccessibilityNodeInfo createNodeForHost() {
        AccessibilityNodeInfo info = AccessibilityNodeInfo.obtain(host);
        host.onInitializeAccessibilityNodeInfo(info);
        // The children of the host view are the virtual views of the children of the root peer.
        nativePopulateHost(nativeHandle, info, host);
        return info;
    }

    private AccessibilityNodeInfo createNodeForChild(int virtualViewId) {
        AccessibilityNodeInfo node = AccessibilityNodeInfo.obtain();
        node.setParent(host);

        nativePopulateNode(nativeHandle, virtualViewId, node, host);

        node.setPackageName(host.getContext().getPackageName());
        node.setSource(host, virtualViewId);

        // The bounds in the parent from the bounds on the screen, and the part of the node that is
        // visible: a node that is not visible to the user is passed over by assistive technology.
        int[] location = new int[2];
        host.getLocationOnScreen(location);
        Rect screen = new Rect();
        node.getBoundsInScreen(screen);
        Rect parent = new Rect(screen);
        parent.offset(host.getScrollX() - location[0], host.getScrollY() - location[1]);
        node.setBoundsInParent(parent);

        Rect visible = new Rect();
        if (host.getLocalVisibleRect(visible)) {
            visible.offset(location[0] - host.getScrollX(), location[1] - host.getScrollY());
            if (screen.intersect(visible)) {
                node.setBoundsInScreen(screen);
                if (isVisibleToUser(screen)) {
                    node.setVisibleToUser(true);
                }
            }
        }

        return node;
    }

    /** Whether a rectangle of the host view is visible: the window and every ancestor are. */
    private boolean isVisibleToUser(Rect localRect) {
        if (localRect == null || localRect.isEmpty()) {
            return false;
        }
        if (host.getWindowVisibility() != View.VISIBLE) {
            return false;
        }
        ViewParent viewParent = host.getParent();
        while (viewParent instanceof View) {
            View view = (View) viewParent;
            if (view.getAlpha() <= 0 || view.getVisibility() != View.VISIBLE) {
                return false;
            }
            viewParent = view.getParent();
        }
        return viewParent != null;
    }

    // ---- what the native side asks -------------------------------------------------------------

    boolean isAccessibilityEnabled() {
        return manager != null && manager.isEnabled();
    }

    boolean isTouchExplorationEnabled() {
        return manager != null && manager.isTouchExplorationEnabled();
    }

    void invalidateHost() {
        host.invalidate();
    }

    boolean isHostFocusedOrRequestFocus() {
        return host.isFocused() || host.requestFocus();
    }

    /** Sends an accessibility event for a virtual view through the parent of the host view. */
    boolean sendEventForVirtualView(int virtualViewId, int eventType) {
        if (virtualViewId == INVALID_ID || !isAccessibilityEnabled()) {
            return false;
        }
        ViewParent parent = host.getParent();
        if (parent == null) {
            return false;
        }
        return parent.requestSendAccessibilityEvent(host, createEvent(virtualViewId, eventType));
    }

    /** Sends the event that the content of a virtual view changed. */
    void sendContentChanged(int virtualViewId, int changeTypes) {
        if (virtualViewId == INVALID_ID || !isAccessibilityEnabled()) {
            return;
        }
        ViewParent parent = host.getParent();
        if (parent == null) {
            return;
        }
        AccessibilityEvent event = createEvent(virtualViewId, AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED);
        event.setContentChangeTypes(changeTypes);
        parent.requestSendAccessibilityEvent(host, event);
    }

    private AccessibilityEvent createEvent(int virtualViewId, int eventType) {
        AccessibilityEvent event = AccessibilityEvent.obtain(eventType);
        if (virtualViewId == HOST_ID) {
            host.onInitializeAccessibilityEvent(event);
            return event;
        }

        AccessibilityNodeInfo node = provider.createAccessibilityNodeInfo(virtualViewId);
        if (node.getText() != null) {
            event.getText().add(node.getText());
        }
        event.setContentDescription(node.getContentDescription());
        event.setScrollable(node.isScrollable());
        event.setPassword(node.isPassword());
        event.setEnabled(node.isEnabled());
        event.setChecked(node.isChecked());
        event.setClassName(node.getClassName());
        event.setSource(host, virtualViewId);
        event.setPackageName(host.getContext().getPackageName());
        return event;
    }

    private static native void nativePopulateHost(long handle, AccessibilityNodeInfo info, View host);

    private static native void nativePopulateNode(
            long handle, int virtualViewId, AccessibilityNodeInfo info, View host);

    private static native boolean nativePerformAction(
            long handle, int virtualViewId, int action, boolean hasArguments, String setText);

    private static native int nativeFindFocus(long handle, int focusType);

    static native boolean nativeDispatchHoverEvent(long handle, int action, float x, float y);

    static native void nativeFocusChanged(long handle, boolean gainFocus);
}
