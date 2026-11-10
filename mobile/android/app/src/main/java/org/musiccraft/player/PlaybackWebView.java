package org.musiccraft.player;

import android.content.Context;
import android.view.View;
import android.webkit.WebView;

/** Preserve active audio when Android hides the activity during playback. */
public final class PlaybackWebView extends WebView {
    public PlaybackWebView(Context context) { super(context); }
    @Override protected void onWindowVisibilityChanged(int visibility) {
        super.onWindowVisibilityChanged(visibility != View.VISIBLE && PlaybackService.running() ? View.VISIBLE : visibility);
    }
}
