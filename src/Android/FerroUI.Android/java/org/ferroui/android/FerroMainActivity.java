package org.ferroui.android;

/**
 * The main activity of an application: its view shows the main view of the application lifetime.
 */
public class FerroMainActivity extends FerroActivity {
    @Override
    protected boolean isMainActivity() {
        return true;
    }
}
