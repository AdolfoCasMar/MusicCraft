package org.musiccraft.player;

import android.net.Uri;
import android.webkit.WebResourceRequest;
import android.webkit.WebResourceResponse;
import java.io.ByteArrayInputStream;
import java.util.Collections;
import java.util.Locale;
import java.util.Map;

/** Experimental bridge to adblock-rust, not the complete Brave browser. */
public final class BraveFilter {
    private static final boolean AVAILABLE;
    private static volatile String script;
    static {
        boolean loaded;
        try { System.loadLibrary("musiccraft_android_filter"); loaded = true; }
        catch (UnsatisfiedLinkError e) { loaded = false; }
        AVAILABLE = loaded;
    }
    private static native boolean nativeShouldBlock(String url, String source, String kind, String method);
    private static native String nativeCosmeticScript();
    public static boolean available() { return AVAILABLE; }
    public static void warmUp() {
        if (AVAILABLE) new Thread(() -> { try { script = nativeCosmeticScript(); } catch (Throwable ignored) {} }, "MusicCraftFilter").start();
    }
    public static String cosmeticScript() { return script; }
    private static String kind(WebResourceRequest request) {
        Map<String,String> headers = request.getRequestHeaders();
        String dest = "", accept = "";
        for (Map.Entry<String,String> entry : headers.entrySet()) {
            if ("sec-fetch-dest".equalsIgnoreCase(entry.getKey())) dest = entry.getValue().toLowerCase(Locale.ROOT);
            if ("accept".equalsIgnoreCase(entry.getKey())) accept = entry.getValue().toLowerCase(Locale.ROOT);
        }
        if (dest.equals("script")) return "script";
        if (dest.equals("image") || accept.startsWith("image/")) return "image";
        if (dest.equals("style")) return "stylesheet";
        if (dest.equals("video") || dest.equals("audio")) return "media";
        String path = request.getUrl().getPath();
        path = path == null ? "" : path.toLowerCase(Locale.ROOT);
        if (path.endsWith(".js")) return "script";
        if (path.endsWith(".css")) return "stylesheet";
        return "xmlhttprequest";
    }
    public static WebResourceResponse intercept(WebResourceRequest request, String source) {
        if (!AVAILABLE || request.isForMainFrame() || source == null) return null;
        Uri sourceUri = Uri.parse(source);
        String host = sourceUri.getHost();
        if (host == null || !(host.equalsIgnoreCase("youtube.com") || host.equalsIgnoreCase("www.youtube.com") || host.equalsIgnoreCase("m.youtube.com"))) return null;
        Uri url = request.getUrl();
        if (!("https".equalsIgnoreCase(url.getScheme()) || "http".equalsIgnoreCase(url.getScheme()))) return null;
        try {
            if (nativeShouldBlock(url.toString(), source, kind(request), request.getMethod())) {
                return new WebResourceResponse("text/plain", "UTF-8", 403, "Blocked", Collections.singletonMap("Cache-Control", "no-store"), new ByteArrayInputStream(new byte[0]));
            }
        } catch (Throwable ignored) { /* Fail open if filtering fails. */ }
        return null;
    }
}
