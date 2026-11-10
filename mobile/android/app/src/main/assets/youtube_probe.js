(()=>{
 const dialog=document.querySelector('ytd-consent-bump-v2-lightbox,ytm-consent-bump-v2-lightbox,ytm-consent-bump-v2-renderer,ytd-consent-bump-lightbox');
 const box=dialog?.querySelector('[role=dialog],#dialog,.eom-dialog')||dialog, rect=box?.getBoundingClientRect();
 if(box&&rect&&rect.width>0&&rect.height>0&&getComputedStyle(box).display!=='none'&&getComputedStyle(box).visibility!=='hidden')return JSON.stringify({type:'consent'});
 if(location.pathname.includes('/results')||location.pathname.includes('/search')||location.pathname.includes('/playlist')){
  const isPlaylist=location.pathname.includes('/playlist');
  const results=[],seen=new Set();
  for(const a of document.querySelectorAll('a[href]')){
   let u;try{u=new URL(a.href,location.href)}catch(_){continue}
   if(!['youtube.com','m.youtube.com','www.youtube.com'].includes(u.hostname))continue;
   const playlistRow=a.closest('ytm-playlist-renderer,ytm-compact-playlist-renderer,ytd-playlist-renderer');
   const videoId=u.searchParams.get('v'),listId=u.searchParams.get('list');
   const isList=!isPlaylist&&!!playlistRow&&!!listId;
   const id=isList?listId:videoId;
   if(!id||!(isList?/^[A-Za-z0-9_-]{1,200}$/.test(id):/^[A-Za-z0-9_-]{11}$/.test(id))||seen.has(id))continue;
   const row=playlistRow||a.closest('ytm-video-with-context-renderer,ytm-compact-video-renderer,ytm-playlist-video-renderer,ytm-playlist-panel-video-renderer,ytm-rich-item-renderer,ytd-video-renderer,ytd-playlist-video-renderer,yt-lockup-view-model');if(!row)continue;
   const name=(row.querySelector('h3,.media-item-headline,.yt-lockup-metadata-view-model__title')?.textContent||a.title||a.textContent||'').trim();
   if(name.length<3)continue;seen.add(id);results.push({id,title:name.slice(0,220),kind:isList?'playlist':'youtube'});if(results.length>=50)break;
  }
  return JSON.stringify({type:isPlaylist?'playlist':'results',results});
 }
 if(location.pathname.includes('/watch')){
  const v=document.querySelector('video');if(!v)return null;
  const id=new URL(location.href).searchParams.get('v');
  if(!window.__musiccraftStart||window.__musiccraftStart.id!==id)window.__musiccraftStart={id,until:Date.now()+25000,attempt:0,started:false,cancelled:false};
  const start=window.__musiccraftStart;
  if(window.__musiccraftAutoplayAllowed){
   try{const p=document.getElementById('movie_player');if(v.muted||p?.isMuted?.()){p?.unMute?.();v.muted=false;}if(Number.isFinite(window.__musiccraftVolume)&&Math.abs(v.volume-window.__musiccraftVolume)>.01)v.volume=window.__musiccraftVolume;}catch(_){}
  }
  if(!v.paused&&v.currentTime>1)start.started=true;
  const needsStart=!start.started&&Date.now()<start.until;
  const needsBackgroundResume=!!window.__musiccraftKeepPlaying&&start.started;
  if(window.__musiccraftAutoplayAllowed&&(needsStart||needsBackgroundResume)&&!start.cancelled&&!v.ended&&v.paused&&Date.now()-start.attempt>1200){
   start.attempt=Date.now();
   if(v.readyState>=2)v.play().catch(()=>{});
   else {const player=document.getElementById('movie_player');if(typeof player?.playVideo==='function')player.playVideo();else document.querySelector('.ytp-large-play-button')?.click();}
  }
  return JSON.stringify({type:'playback',title:(document.querySelector('h1,.slim-video-metadata-title')?.textContent||document.title||'YouTube').trim().slice(0,250),time:v.currentTime||0,duration:Number.isFinite(v.duration)?v.duration:0,playing:!v.paused,ended:v.ended,muted:!!v.muted});
 }
 return null;
})()
