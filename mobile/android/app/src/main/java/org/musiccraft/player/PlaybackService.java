package org.musiccraft.player;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.media.MediaMetadata;
import android.media.session.MediaSession;
import android.media.session.PlaybackState;
import android.os.IBinder;
import android.os.PowerManager;
import java.lang.ref.WeakReference;

/** Keeps an active playback session visible to Android while the activity is backgrounded. */
public final class PlaybackService extends Service {
    private static final String CHANNEL = "musiccraft_playback";
    private static final int NOTIFICATION = 1001;
    private static WeakReference<MainActivity> activity = new WeakReference<>(null);
    private static PlaybackService instance;
    private MediaSession session;
    private PowerManager.WakeLock wakeLock;
    private boolean playing;
    private String title = "MusicCraft";
    private long position, duration;
    private String notificationKey = "";

    public static void attach(MainActivity target) { activity = new WeakReference<>(target); }
    public static void detach(MainActivity target) { if (activity.get()==target) activity.clear(); }
    public static boolean running() { return instance != null; }
    public static void publish(String name, boolean isPlaying, double seconds, double total) {
        PlaybackService current=instance;
        if(current!=null)current.update(name,isPlaying,seconds,total);
    }
    private static void send(String command) {
        MainActivity current=activity.get();
        if(current!=null)current.mediaAction(command);
    }

    @Override public void onCreate() {
        super.onCreate(); instance=this;
        NotificationManager notifications=(NotificationManager)getSystemService(Context.NOTIFICATION_SERVICE);
        notifications.createNotificationChannel(new NotificationChannel(CHANNEL,"Reproducción de música",NotificationManager.IMPORTANCE_LOW));
        PowerManager power=(PowerManager)getSystemService(Context.POWER_SERVICE);
        wakeLock=power.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK,"MusicCraft:Playback");
        wakeLock.setReferenceCounted(false);
        session=new MediaSession(this,"MusicCraft");
        session.setCallback(new MediaSession.Callback() {
            @Override public void onPlay() { send("play"); }
            @Override public void onPause() { send("pause"); }
            @Override public void onSkipToNext() { send("next"); }
            @Override public void onSkipToPrevious() { send("previous"); }
            @Override public void onSeekTo(long pos) {
                long target=Math.max(0,duration>0?Math.min(pos,duration):pos);
                MainActivity current=activity.get();
                if(current!=null) { position=target; updateSession(); current.mediaSeek(target); }
            }
            @Override public void onStop() { send("stop"); stopSelf(); }
        });
        session.setActive(true);
        updateSession();
        startForeground(NOTIFICATION,notification());
    }

    private PendingIntent action(String command, int request) {
        Intent intent=new Intent(this,PlaybackService.class).setAction(command);
        return PendingIntent.getService(this,request,intent,PendingIntent.FLAG_UPDATE_CURRENT|PendingIntent.FLAG_IMMUTABLE);
    }
    private Notification notification() {
        Intent open=new Intent(this,MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP|Intent.FLAG_ACTIVITY_SINGLE_TOP);
        PendingIntent openApp=PendingIntent.getActivity(this,0,open,PendingIntent.FLAG_UPDATE_CURRENT|PendingIntent.FLAG_IMMUTABLE);
        Notification.Action previous=new Notification.Action.Builder(android.R.drawable.ic_media_previous,"Anterior",action("previous",1)).build();
        Notification.Action toggle=new Notification.Action.Builder(playing?android.R.drawable.ic_media_pause:android.R.drawable.ic_media_play,playing?"Pausar":"Reproducir",action(playing?"pause":"play",2)).build();
        Notification.Action next=new Notification.Action.Builder(android.R.drawable.ic_media_next,"Siguiente",action("next",3)).build();
        return new Notification.Builder(this,CHANNEL).setSmallIcon(android.R.drawable.ic_media_play).setContentTitle(title).setContentText(playing?"Reproduciendo":"En pausa")
            .setContentIntent(openApp).setOnlyAlertOnce(true).setVisibility(Notification.VISIBILITY_PUBLIC).setOngoing(playing)
            .addAction(previous).addAction(toggle).addAction(next)
            .setStyle(new Notification.MediaStyle().setMediaSession(session.getSessionToken()).setShowActionsInCompactView(0,1,2)).build();
    }
    private void updateSession() {
        session.setMetadata(new MediaMetadata.Builder().putString(MediaMetadata.METADATA_KEY_TITLE,title).putLong(MediaMetadata.METADATA_KEY_DURATION,duration).build());
        long actions=PlaybackState.ACTION_PLAY|PlaybackState.ACTION_PAUSE|PlaybackState.ACTION_PLAY_PAUSE|PlaybackState.ACTION_SKIP_TO_NEXT|PlaybackState.ACTION_SKIP_TO_PREVIOUS|PlaybackState.ACTION_STOP|PlaybackState.ACTION_SEEK_TO;
        session.setPlaybackState(new PlaybackState.Builder().setActions(actions).setState(playing?PlaybackState.STATE_PLAYING:PlaybackState.STATE_PAUSED,position,playing?1f:0f).build());
    }
    private void update(String name, boolean isPlaying, double seconds, double total) {
        title=name==null||name.trim().isEmpty()?"MusicCraft":name.substring(0,Math.min(250,name.length()));
        playing=isPlaying; position=(long)(Math.max(0,seconds)*1000); duration=(long)(Math.max(0,total)*1000);
        if(playing){
            if(playing && !wakeLock.isHeld())wakeLock.acquire(6*60*60*1000L);
        } else { if(wakeLock.isHeld())wakeLock.release(); }
        updateSession();
        String key=title+"|"+playing;
        if(!key.equals(notificationKey)){notificationKey=key;((NotificationManager)getSystemService(Context.NOTIFICATION_SERVICE)).notify(NOTIFICATION,notification());}
    }
    @Override public int onStartCommand(Intent intent,int flags,int id) {
        String action=intent==null?null:intent.getAction();
        if(action!=null){ if("stop".equals(action)){send("stop");stopSelf();} else if(action.equals("play")||action.equals("pause")||action.equals("next")||action.equals("previous"))send(action); }
        return START_NOT_STICKY;
    }
    @Override public void onTaskRemoved(Intent intent) { send("stop");stopSelf();super.onTaskRemoved(intent); }
    @Override public void onDestroy() {
        if(wakeLock!=null&&wakeLock.isHeld())wakeLock.release();
        if(session!=null){session.setActive(false);session.release();}
        if(instance==this)instance=null;
        stopForeground(STOP_FOREGROUND_REMOVE);super.onDestroy();
    }
    @Override public IBinder onBind(Intent intent) { return null; }
}
