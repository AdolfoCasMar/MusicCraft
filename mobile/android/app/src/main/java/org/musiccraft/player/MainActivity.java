package org.musiccraft.player;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.graphics.Bitmap;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.View;
import android.webkit.JavascriptInterface;
import android.webkit.ValueCallback;
import android.webkit.WebChromeClient;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.webkit.WebResourceRequest;
import android.webkit.WebResourceResponse;
import android.webkit.ServiceWorkerClient;
import android.webkit.ServiceWorkerController;
import android.widget.Button;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import java.nio.charset.StandardCharsets;
import org.json.JSONArray;
import org.json.JSONObject;
import org.json.JSONTokener;

public class MainActivity extends Activity {
    private static final int PICK_AUDIO = 42;
    private WebView view;
    private WebView player;
    private WebView searchView;
    private LinearLayout youtubePanel;
    private LinearLayout searchPanel;
    private FrameLayout root;
    private Button youtubeBack;
    private Button searchBack;
    private boolean showingYoutube;
    private volatile boolean youtubeSelected;
    private boolean searchingYoutube;
    private boolean importingPlaylist;
    private boolean consentIsSearch;
    private boolean autoStartAllowed;
    private boolean activityInBackground;
    private double youtubeVolume = 0.7;
    private int navigationGeneration;
    private int searchGeneration;
    private String youtubeProbe;
    private String lastResults = "";
    private boolean destroyed;
    private final Handler handler = new Handler(Looper.getMainLooper());
    private ValueCallback<Uri[]> selection;
    private volatile String topUrl = "";
    private volatile String searchUrl = "";

    void mediaAction(String action) {
        if (!("play".equals(action)||"pause".equals(action)||"stop".equals(action)||"next".equals(action)||"previous".equals(action))) return;
        handler.post(() -> { if (!destroyed) player.evaluateJavascript("window.musiccraftMediaAction&&window.musiccraftMediaAction(" + JSONObject.quote(action) + ")", null); });
    }

    void mediaSeek(long milliseconds) {
        final double seconds=Math.max(0,milliseconds)/1000.0;
        handler.post(() -> { if (!destroyed) player.evaluateJavascript("window.musiccraftMediaSeek&&window.musiccraftMediaSeek(" + seconds + ")", null); });
    }

    private void showYoutube(boolean show) {
        showingYoutube = show;
        view.setImportantForAccessibility(show && !consentIsSearch ? View.IMPORTANT_FOR_ACCESSIBILITY_AUTO : View.IMPORTANT_FOR_ACCESSIBILITY_NO_HIDE_DESCENDANTS);
        searchView.setImportantForAccessibility(show && consentIsSearch ? View.IMPORTANT_FOR_ACCESSIBILITY_AUTO : View.IMPORTANT_FOR_ACCESSIBILITY_NO_HIDE_DESCENDANTS);
        if (show && consentIsSearch) { searchPanel.bringToFront(); searchBack.setVisibility(View.VISIBLE); }
        else if (show) { youtubePanel.bringToFront(); youtubeBack.setVisibility(View.VISIBLE); }
        else { player.bringToFront(); youtubeBack.setVisibility(View.GONE); searchBack.setVisibility(View.GONE); }
    }

    private final class PlayerBridge {
        @JavascriptInterface public void beginPlayback() {
            handler.post(() -> {
                if (!PlaybackService.running()) {
                    try { startForegroundService(new Intent(MainActivity.this, PlaybackService.class)); }
                    catch (RuntimeException ignored) { /* Playback may continue while this screen is visible. */ }
                }
            });
        }
        @JavascriptInterface public void reportMedia(String name, boolean playing, double position, double duration) {
            if(!Double.isFinite(position)||!Double.isFinite(duration))return;
            String safeName=name==null?"MusicCraft":name.substring(0,Math.min(250,name.length()));
            handler.post(() -> PlaybackService.publish(safeName,playing,position,duration));
        }
        @JavascriptInterface public void stopPlaybackService() { handler.post(() -> stopService(new Intent(MainActivity.this,PlaybackService.class))); }
        @JavascriptInterface public void search(String text, String kind) {
            if (text == null || text.trim().isEmpty() || text.length() > 300) return;
            String filter = "playlists".equals(kind) ? "EgIQAw==" : "EgIQAQ==";
            String url = new Uri.Builder().scheme("https").authority("m.youtube.com").path("/results").appendQueryParameter("search_query", text.trim()).appendQueryParameter("sp",filter).build().toString();
            handler.post(() -> { searchGeneration++; searchingYoutube = true; importingPlaylist = false; lastResults = ""; MainActivity.this.showYoutube(false); searchView.loadUrl(url); });
        }
        @JavascriptInterface public void importPlaylist(String id) {
            if(id == null || !id.matches("[A-Za-z0-9_-]{1,200}")) return;
            String url = new Uri.Builder().scheme("https").authority("m.youtube.com").path("/playlist").appendQueryParameter("list",id).build().toString();
            handler.post(() -> { searchGeneration++; searchingYoutube = true; importingPlaylist = true; lastResults = ""; searchView.loadUrl(url); });
        }
        @JavascriptInterface public void playVideo(String id) {
            if (id == null || !id.matches("[A-Za-z0-9_-]{11}")) return;
            handler.post(() -> {
                navigationGeneration++; searchingYoutube = false; youtubeSelected = true; autoStartAllowed = true;
                player.evaluateJavascript("window.musiccraftUseYouTube&&window.musiccraftUseYouTube()", null);
                MainActivity.this.showYoutube(false); view.loadUrl("https://m.youtube.com/watch?v=" + id);
            });
        }
        @JavascriptInterface public void showYoutube() { handler.post(() -> MainActivity.this.showYoutube(true)); }
        @JavascriptInterface public void chooseConsent(String choice, String source) {
            String selector;
            if ("reject".equals(choice)) selector = ".eom-reject button";
            else if ("accept".equals(choice)) selector = ".eom-accept button";
            else return;
            final String wanted = "reject".equals(choice) ? "reject" : "accept";
            final String code = "(()=>{let b=document.querySelector('" + selector + "');if((!b||!b.getClientRects().length)&&['consent.youtube.com','consent.google.com'].includes(location.hostname)){const names=" +
                ("reject".equals(wanted) ? "['reject all','rechazar todo']" : "['accept all','aceptar todo']") +
                ";b=[...document.querySelectorAll('button')].find(x=>x.getClientRects().length&&names.includes((x.textContent||'').trim().toLowerCase()))}if(!b||!b.getClientRects().length)return false;b.click();return true})()";
            final boolean fromSearch = "search".equals(source);
            handler.post(() -> { consentIsSearch=fromSearch; (fromSearch ? searchView : view).evaluateJavascript(code, result -> {
                if (!"true".equals(result)) player.evaluateJavascript("window.musiccraftConsentNeedsReview&&window.musiccraftConsentNeedsReview()", null);
            }); });
        }
        @JavascriptInterface public void useLocal() { youtubeSelected = false; command("pause"); }
        @JavascriptInterface public void volume(double value) {
            if (!Double.isFinite(value)) return;
            double v = Math.max(0, Math.min(1, value));
            handler.post(() -> { youtubeVolume = v; view.evaluateJavascript("(()=>{const v=document.querySelector('video'),p=document.getElementById('movie_player');if(v){v.volume=" + v + ";if(" + v + ">0){p?.unMute?.();v.muted=false;}}})()", null); });
        }
        @JavascriptInterface public void seek(double value) {
            if (!Double.isFinite(value)) return;
            double v = Math.max(0, value);
            handler.post(() -> view.evaluateJavascript("(()=>{const v=document.querySelector('video');if(v)v.currentTime=" + v + ";})()", null));
        }
        @JavascriptInterface public void equalizer(boolean enabled, String raw, double preamp) {
            try {
                if (!Double.isFinite(preamp)) return;
                double pre = Math.max(-12,Math.min(12,preamp));
                JSONArray input = new JSONArray(raw), bands = new JSONArray();
                if (input.length() != 9) return;
                for (int i=0; i<9; i++) { double n=input.getDouble(i); if(!Double.isFinite(n))return; bands.put(Math.max(-12, Math.min(12, n))); }
                String script = "(()=>{try{const v=document.querySelector('video');if(!v)return 'esperando';let g=window.__musiccraftEq;" +
                    "if(!g){window.__musiccraftEq=g={context:new AudioContext(),video:null};}" +
                    "if(g.video!==v){const src=v.currentSrc||'';if(!src.startsWith('blob:'+location.origin+'/'))return src?'incompatible':'esperando';" +
                    "if(g.output)try{g.output.disconnect()}catch(_){};const source=g.context.createMediaElementSource(v);g.preamp=g.context.createGain();source.connect(g.preamp);let node=g.preamp;g.filters=[];" +
                    "[60,170,310,600,1000,3000,6000,12000,16000].forEach((hz,i)=>{let f=g.context.createBiquadFilter();f.type=i===0?'lowshelf':i===8?'highshelf':'peaking';f.frequency.value=hz;f.Q.value=1;node.connect(f);node=f;g.filters.push(f)});" +
                    "g.analyser=g.context.createAnalyser();g.analyser.fftSize=1024;g.analyser.smoothingTimeConstant=.7;node.connect(g.analyser);g.analyser.connect(g.context.destination);g.output=g.analyser;g.video=v;}" +
                    "g.preamp.gain.setTargetAtTime(" + enabled + "?Math.pow(10," + pre + "/20):1,g.context.currentTime,.02);const gains=" + bands + ";g.filters.forEach((f,i)=>f.gain.setTargetAtTime(" + enabled + "?gains[i]:0,g.context.currentTime,.02));g.context.resume().catch(()=>{});return " + enabled + "?(g.context.state==='running'?'activo':'esperando'):'desactivado';}catch(_){return 'error'}})()";
                handler.post(() -> view.evaluateJavascript(script, result -> {
                    try { Object status = new JSONTokener(result).nextValue(); if(status instanceof String) player.evaluateJavascript("window.musiccraftEqState&&window.musiccraftEqState(" + JSONObject.quote((String)status) + ")",null); } catch(Exception ignored) {}
                }));
            } catch (Exception ignored) { }
        }
        @JavascriptInterface public void command(String value) {
            final String action;
            switch (value) {
                case "play": action = "if(window.__musiccraftStart){window.__musiccraftStart.cancelled=false;window.__musiccraftStart.started=false;window.__musiccraftStart.until=Date.now()+25000;}const v=document.querySelector('video'),p=document.getElementById('movie_player');if(v){p?.unMute?.();v.muted=false;v.play().catch(()=>{});}"; break;
                case "pause": action = "if(window.__musiccraftStart)window.__musiccraftStart.cancelled=true;const v=document.querySelector('video');if(v)v.pause();"; break;
                case "stop": action = "if(window.__musiccraftStart)window.__musiccraftStart.cancelled=true;const v=document.querySelector('video');if(v){v.pause();v.currentTime=0;}"; break;
                default: return;
            }
            handler.post(() -> { autoStartAllowed = value.equals("play"); view.evaluateJavascript("(()=>{" + action + "})()", null); });
        }
    }

    private void pollSpectrum() {
        if (destroyed) return;
        if (youtubeSelected && !activityInBackground) {
            int generation=navigationGeneration;
            String script="(()=>{const g=window.__musiccraftEq,v=document.querySelector('video');if(!g?.analyser||!v||v.paused||v.ended)return null;const a=g.analyser,d=new Uint8Array(a.frequencyBinCount);a.getByteFrequencyData(d);const values=[];for(let i=0;i<15;i++){let low=Math.floor(50*Math.pow(320,i/15)*a.fftSize/g.context.sampleRate),high=Math.ceil(50*Math.pow(320,(i+1)/15)*a.fftSize/g.context.sampleRate);let max=0;for(let j=Math.max(0,low);j<Math.min(d.length,Math.max(low+1,high));j++)max=Math.max(max,d[j]);values.push(max)}return JSON.stringify(values)})()";
            view.evaluateJavascript(script, value -> {
                if (generation!=navigationGeneration||!youtubeSelected||value==null||"null".equals(value)) return;
                try { Object raw=new JSONTokener(value).nextValue(); if(raw instanceof String) { JSONArray bands=new JSONArray((String)raw); if(bands.length()==15)player.evaluateJavascript("window.musiccraftSpectrum&&window.musiccraftSpectrum("+bands+")",null); } } catch(Exception ignored) {}
            });
        }
        handler.postDelayed(this::pollSpectrum, 150);
    }

    private void pollYoutube() {
        if (destroyed) return;
        if (youtubeSelected || (showingYoutube && !consentIsSearch)) {
            applyFilters(view, topUrl);
            int generation = navigationGeneration;
            view.evaluateJavascript("window.__musiccraftAutoplayAllowed=" + autoStartAllowed + ";window.__musiccraftKeepPlaying=" + activityInBackground + ";window.__musiccraftVolume=" + youtubeVolume + ";" + youtubeProbe, value -> {
                if (generation != navigationGeneration) return;
                if (value == null || value.equals("null")) return;
                try {
                    Object raw = new JSONTokener(value).nextValue();
                    if (!(raw instanceof String)) return;
                    JSONObject data = new JSONObject((String) raw);
                    String type = data.optString("type");
                    if ("playback".equals(type) && youtubeSelected) {
                        player.evaluateJavascript("window.musiccraftYouTubeState&&window.musiccraftYouTubeState(" + JSONObject.quote((String) raw) + ")", null);
                    } else if ("consent".equals(type)) {
                        consentIsSearch = false;
                        player.evaluateJavascript("window.musiccraftYouTubeConsent&&window.musiccraftYouTubeConsent('playback')", null);
                    }
                } catch (Exception ignored) { }
            });
        }
        if (searchingYoutube) {
            applyFilters(searchView, searchUrl);
            int generation = searchGeneration;
            searchView.evaluateJavascript("window.__musiccraftAutoplayAllowed=false;" + youtubeProbe, value -> {
                if (generation != searchGeneration || value == null || value.equals("null")) return;
                try {
                    Object raw = new JSONTokener(value).nextValue(); if (!(raw instanceof String)) return;
                    JSONObject data = new JSONObject((String) raw); String type = data.optString("type");
                    if ("consent".equals(type)) { consentIsSearch = true; lastResults=""; player.evaluateJavascript("window.musiccraftYouTubeConsent&&window.musiccraftYouTubeConsent('search')", null); }
                    else if ("results".equals(type) && !importingPlaylist) {
                        JSONArray items=data.optJSONArray("results"); if(items==null)return; String json=items.toString();
                        if (!json.equals(lastResults)) { lastResults=json; player.evaluateJavascript("window.musiccraftYouTubeResults&&window.musiccraftYouTubeResults(" + JSONObject.quote((String) raw) + ")", null); }
                    } else if ("playlist".equals(type) && importingPlaylist) {
                        JSONArray items=data.optJSONArray("results"); if(items!=null && items.length()>0) {
                            searchingYoutube=false; importingPlaylist=false;
                            player.evaluateJavascript("window.musiccraftPlaylistImported&&window.musiccraftPlaylistImported(" + JSONObject.quote((String) raw) + ")", null);
                        }
                    }
                } catch (Exception ignored) { }
            });
        }
        handler.postDelayed(this::pollYoutube, 1000);
    }

    private void applyFilters(WebView webView, String url) {
        Uri uri = Uri.parse(url);
        String host = uri.getHost();
        if (host == null || !(host.equalsIgnoreCase("youtube.com") || host.equalsIgnoreCase("www.youtube.com") || host.equalsIgnoreCase("m.youtube.com"))) return;
        String script = BraveFilter.cosmeticScript();
        if (script != null) webView.evaluateJavascript(script, null);
    }

    private boolean routeNavigation(WebView webView, WebResourceRequest request) {
        Uri target=request.getUrl(); String scheme=target.getScheme();
        if(scheme==null)return true;
        if(!request.isForMainFrame() && (scheme.equalsIgnoreCase("blob") || scheme.equalsIgnoreCase("data")))return false;
        if(!scheme.equalsIgnoreCase("https") && !scheme.equalsIgnoreCase("http"))return true;
        if(!request.isForMainFrame())return false;
        String host=target.getHost();
        if(host!=null && host.equalsIgnoreCase("music.youtube.com")) {
            Uri.Builder mobile=target.buildUpon().scheme("https").authority("m.youtube.com");
            if("/search".equals(target.getPath())) { String query=target.getQueryParameter("q"); mobile.path("/results").clearQuery(); if(query!=null)mobile.appendQueryParameter("search_query",query); }
            webView.loadUrl(mobile.build().toString()); return true;
        }
        return false;
    }

    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        WebView.setWebContentsDebuggingEnabled(false);
        PlaybackService.attach(this);
        getWindow().setStatusBarColor(0xff121321);
        getWindow().setNavigationBarColor(0xff121321);
        try (java.io.InputStream input = getAssets().open("youtube_probe.js")) {
            java.io.ByteArrayOutputStream output = new java.io.ByteArrayOutputStream(); byte[] buffer = new byte[4096]; int count;
            while ((count=input.read(buffer))!=-1) output.write(buffer,0,count);
            youtubeProbe = new String(output.toByteArray(), StandardCharsets.UTF_8);
        } catch (Exception e) { throw new IllegalStateException("No se pudo cargar el controlador de YouTube", e); }
        view = new PlaybackWebView(this);
        player = new PlaybackWebView(this);
        searchView = new WebView(this);
        WebSettings settings = view.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        settings.setMediaPlaybackRequiresUserGesture(false);
        settings.setAllowFileAccess(true);
        settings.setAllowFileAccessFromFileURLs(false);
        settings.setAllowUniversalAccessFromFileURLs(false);
        WebSettings searchSettings=searchView.getSettings();
        searchSettings.setJavaScriptEnabled(true); searchSettings.setDomStorageEnabled(true);
        searchSettings.setMediaPlaybackRequiresUserGesture(true); searchSettings.setAllowFileAccess(false);
        WebSettings playerSettings = player.getSettings();
        playerSettings.setJavaScriptEnabled(true);
        playerSettings.setDomStorageEnabled(true);
        playerSettings.setMediaPlaybackRequiresUserGesture(true);
        playerSettings.setAllowFileAccess(true);
        playerSettings.setAllowFileAccessFromFileURLs(false);
        playerSettings.setAllowUniversalAccessFromFileURLs(false);
        player.addJavascriptInterface(new PlayerBridge(), "MusicCraftAndroid");
        // This WebView has a native bridge, so it must never navigate to remote pages.
        player.setWebViewClient(new WebViewClient() {
            @Override public boolean shouldOverrideUrlLoading(WebView webView, WebResourceRequest request) { return true; }
        });
        BraveFilter.warmUp();
        ServiceWorkerController.getInstance().setServiceWorkerClient(new ServiceWorkerClient() {
            @Override public WebResourceResponse shouldInterceptRequest(WebResourceRequest request) {
                return BraveFilter.intercept(request, topUrl.isEmpty() ? searchUrl : topUrl);
            }
        });
        view.setWebViewClient(new WebViewClient() {
            @Override public void onPageStarted(WebView webView, String url, Bitmap icon) {
                topUrl = url; applyFilters(webView, url);
                Uri uri = Uri.parse(url); String host = uri.getHost();
                if (host != null && (host.equalsIgnoreCase("consent.youtube.com") || host.equalsIgnoreCase("consent.google.com"))) { consentIsSearch=false; player.evaluateJavascript("window.musiccraftYouTubeConsent&&window.musiccraftYouTubeConsent('playback')", null); }
            }
            @Override public void onPageFinished(WebView webView, String url) { applyFilters(webView, url); }
            @Override public WebResourceResponse shouldInterceptRequest(WebView webView, WebResourceRequest request) {
                return BraveFilter.intercept(request, topUrl);
            }
            @Override public boolean shouldOverrideUrlLoading(WebView webView, WebResourceRequest request) {
                return routeNavigation(webView,request);
            }
        });
        searchView.setWebViewClient(new WebViewClient() {
            @Override public void onPageStarted(WebView webView, String url, Bitmap icon) {
                searchUrl=url; applyFilters(webView,url); String host=Uri.parse(url).getHost();
                if(host!=null && (host.equalsIgnoreCase("consent.youtube.com") || host.equalsIgnoreCase("consent.google.com"))) { consentIsSearch=true; lastResults=""; player.evaluateJavascript("window.musiccraftYouTubeConsent&&window.musiccraftYouTubeConsent('search')",null); }
            }
            @Override public void onPageFinished(WebView webView,String url) { applyFilters(webView,url); }
            @Override public WebResourceResponse shouldInterceptRequest(WebView webView,WebResourceRequest request) { return BraveFilter.intercept(request,searchUrl); }
            @Override public boolean shouldOverrideUrlLoading(WebView webView,WebResourceRequest request) { return routeNavigation(webView,request); }
        });
        player.setWebChromeClient(new WebChromeClient() {
            @Override public boolean onShowFileChooser(WebView webView, ValueCallback<Uri[]> callback, FileChooserParams params) {
                if (selection != null) selection.onReceiveValue(null);
                selection = callback;
                Intent picker = new Intent(Intent.ACTION_OPEN_DOCUMENT);
                picker.addCategory(Intent.CATEGORY_OPENABLE);
                picker.setType("audio/*");
                picker.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true);
                try { startActivityForResult(picker, PICK_AUDIO); return true; }
                catch (Exception e) { selection = null; callback.onReceiveValue(null); return false; }
            }
        });
        root = new FrameLayout(this);
        youtubePanel = new LinearLayout(this); youtubePanel.setOrientation(LinearLayout.VERTICAL);
        youtubeBack = new Button(this); youtubeBack.setText("VOLVER A MUSICCRAFT"); youtubeBack.setOnClickListener(v -> showYoutube(false));
        youtubePanel.addView(youtubeBack, new LinearLayout.LayoutParams(-1, -2));
        youtubePanel.addView(view, new LinearLayout.LayoutParams(-1, 0, 1));
        root.addView(youtubePanel, new FrameLayout.LayoutParams(-1, -1));
        searchPanel = new LinearLayout(this); searchPanel.setOrientation(LinearLayout.VERTICAL);
        searchBack = new Button(this); searchBack.setText("VOLVER A MUSICCRAFT"); searchBack.setOnClickListener(v -> showYoutube(false));
        searchPanel.addView(searchBack,new LinearLayout.LayoutParams(-1,-2));
        searchPanel.addView(searchView,new LinearLayout.LayoutParams(-1,0,1));
        root.addView(searchPanel,new FrameLayout.LayoutParams(-1,-1));
        root.addView(player, new FrameLayout.LayoutParams(-1, -1));
        showYoutube(false); setContentView(root);
        player.loadUrl("file:///android_asset/index.html");
        pollYoutube();
        pollSpectrum();
    }

    @Override protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        if (request != PICK_AUDIO || selection == null) return;
        Uri[] files = null;
        if (result == RESULT_OK && data != null) {
            if (data.getClipData() != null) {
                int count = data.getClipData().getItemCount(); files = new Uri[count];
                for (int i=0; i<count; i++) files[i] = data.getClipData().getItemAt(i).getUri();
            } else if (data.getData() != null) files = new Uri[]{data.getData()};
        }
        selection.onReceiveValue(files); selection = null;
    }

    @Override protected void onPause() { activityInBackground=true; super.onPause(); }
    @Override protected void onResume() { super.onResume(); activityInBackground=false; }

    @Override public void onBackPressed() { if (showingYoutube) showYoutube(false); else super.onBackPressed(); }
    @Override protected void onDestroy() {
        if (selection != null) { selection.onReceiveValue(null); selection = null; }
        destroyed = true; handler.removeCallbacksAndMessages(null);
        PlaybackService.detach(this); stopService(new Intent(this,PlaybackService.class));
        player.removeJavascriptInterface("MusicCraftAndroid"); player.destroy(); view.destroy(); searchView.destroy(); super.onDestroy();
    }
}
