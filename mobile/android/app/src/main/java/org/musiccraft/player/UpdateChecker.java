package org.musiccraft.player;

import org.json.JSONArray;
import org.json.JSONObject;
import java.net.HttpURLConnection;
import java.net.URL;
import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/** Checks only public MusicCraft releases. No installation or download is performed. */
final class UpdateChecker {
    static final String RELEASES_PAGE = "https://github.com/AdolfoCasMar/MusicCraft/releases";
    private static final Pattern TAG = Pattern.compile("^android-v(\\d+)\\.(\\d+)(?:\\.(\\d+))?$");
    static String newerRelease(String json, int[] current) throws Exception {
        JSONArray releases = new JSONArray(json);
        int[] best = null; boolean found = false;
        for (int i=0;i<releases.length();i++) {
            JSONObject release=releases.getJSONObject(i);
            if(release.optBoolean("draft"))continue;
            Matcher m=TAG.matcher(release.optString("tag_name"));
            if(!m.matches())continue;
            int[] version={Integer.parseInt(m.group(1)),Integer.parseInt(m.group(2)),m.group(3)==null?0:Integer.parseInt(m.group(3))};
            JSONArray assets=release.optJSONArray("assets"); boolean apk=false;
            if(assets!=null)for(int j=0;j<assets.length();j++)if(assets.optJSONObject(j)!=null&&assets.optJSONObject(j).optString("name").toLowerCase(java.util.Locale.ROOT).endsWith(".apk"))apk=true;
            if(apk){found=true;if(compare(version,current)>0&&(best==null||compare(version,best)>0))best=version;}
        }
        if(!found)return null;
        if(best==null)return "";
        return best[0]+"."+best[1]+(best[2]==0?"":"."+best[2]);
    }
    static int compare(int[] a,int[] b){for(int i=0;i<3;i++){int n=Integer.compare(a[i],b[i]);if(n!=0)return n;}return 0;}
    static String fetch() throws Exception {
        HttpURLConnection c=(HttpURLConnection)new URL("https://api.github.com/repos/AdolfoCasMar/MusicCraft/releases?per_page=100").openConnection();
        c.setConnectTimeout(10000);c.setReadTimeout(10000);c.setRequestProperty("Accept","application/vnd.github+json");c.setRequestProperty("User-Agent","MusicCraft-Android");
        try {
            if(c.getResponseCode()!=200)throw new java.io.IOException("GitHub no está disponible");
            try(InputStream in=c.getInputStream();ByteArrayOutputStream out=new ByteArrayOutputStream()){
                byte[] buf=new byte[8192];int n;while((n=in.read(buf))!=-1){out.write(buf,0,n);if(out.size()>2_000_000)throw new java.io.IOException("Respuesta demasiado grande");}return out.toString("UTF-8");
            }
        } finally { c.disconnect(); }
    }
}
