(()=>{
 if(window.__baBridge) return; window.__baBridge=true;
 const send=(type,data)=>{try{window.ipc.postMessage(JSON.stringify({type,...data}))}catch(_){}};
 let was=null, lastList='', attached=null, consentVisible=false;
 // YouTube can pause its video while the first page is still initializing.
 // Retry only during startup; transport controls can cancel this at any time.
 let startupDeadline=0, lastStartAttempt=0, startPending=false, startupCancelled=false;
 window.braveampCancelStartup=()=>{startupCancelled=true;};
 // WebAudio source is built once per video element. YouTube currently uses a
 // same-origin MediaSource blob; do not attach to foreign media URLs, which
 // browsers are required to render as silence in a WebAudio graph.
 let eqContext=null, eqGraph=null, eqVideo=null, eqSettings=null;
 function applyEqualizer(settings){
   eqSettings=settings;const v=document.querySelector('video');
   if(!v) return {ok:false,reason:'Espera a que cargue el vídeo.'};
   if(eqVideo!==v){
     const src=v.currentSrc||'';
     if(!src.startsWith('blob:'+location.origin+'/'))return {ok:false,reason:'La fuente de audio no es compatible con el ecualizador.'};
     if(!eqContext) eqContext=new AudioContext();
     if(eqGraph){try{eqGraph.output.disconnect()}catch(_){}}
     const input=eqContext.createMediaElementSource(v), preamp=eqContext.createGain(), bands=[];
     input.connect(preamp);let node=preamp;
     [60,170,310,600,1000,3000,6000,12000,16000].forEach((frequency,i)=>{
       const filter=eqContext.createBiquadFilter();filter.type=i===0?'lowshelf':i===8?'highshelf':'peaking';filter.frequency.value=frequency;filter.Q.value=1;node.connect(filter);node=filter;bands.push(filter);
     });node.connect(eqContext.destination);eqVideo=v;eqGraph={input,preamp,bands,output:node};
   }
   const now=eqContext.currentTime,enabled=!!settings.enabled,clamp=x=>Math.max(-12,Math.min(12,Number(x)||0));
   eqGraph.preamp.gain.setTargetAtTime(enabled?10**(clamp(settings.preamp)/20):1,now,.02);
   eqGraph.bands.forEach((f,i)=>f.gain.setTargetAtTime(enabled?clamp(settings.bands?.[i]):0,now,.02));
   eqContext.resume().catch(e=>send('eq_error',{message:e.message}));return {ok:true};
 }
 window.braveampEq={configure(settings){try{const result=applyEqualizer(settings);if(!result.ok)send('eq_error',{message:result.reason});else send('eq_ready',{});return result}catch(e){send('eq_error',{message:e.message});return {ok:false,reason:e.message}}},status(){return {active:!!eqGraph,enabled:!!eqSettings?.enabled,gains:eqGraph?.bands.map(f=>f.gain.value)||[],state:eqContext?.state}}};

 function tick(){
   // Do not make a cookie choice: only signal a visible first-party dialog.
   const dialog=document.querySelector('ytd-consent-bump-v2-lightbox, ytm-consent-bump-v2-lightbox, ytd-consent-bump-lightbox');
   const surface=dialog?.querySelector('[role=dialog], #dialog') || dialog;
   const rect=surface?.getBoundingClientRect();
   const visible=!!(surface && surface.isConnected && !surface.hidden && rect && rect.width>0 && rect.height>0 && getComputedStyle(surface).display !== 'none' && getComputedStyle(surface).visibility !== 'hidden');
   if(visible && !document.getElementById('braveamp-consent-layout')){
     const style=document.createElement('style');style.id='braveamp-consent-layout';
     style.textContent='ytd-consent-bump-v2-lightbox #dialog .eom-buttons>.eom-button-row:first-child{position:fixed!important;bottom:0!important;left:0!important;right:0!important;z-index:2147483640!important;display:flex!important;justify-content:center!important;gap:8px!important;padding:8px 4px!important;background:#fff!important;box-shadow:0 -2px 8px #0003!important}';
     (document.head || document.documentElement).appendChild(style);
   }
   if(visible !== consentVisible){consentVisible=visible;send(visible?'consent_required':'consent_resolved',{});}
   const v=document.querySelector('video');
   if(v && location.pathname.includes('/watch')){
     if(v!==attached){attached=v;if(!startupDeadline)startupDeadline=Date.now()+30000;}
     if(visible && !startupCancelled)startupDeadline=Date.now()+30000;
     if(!startupCancelled && v.currentTime>1 && !v.paused)startupCancelled=true;
     if(!startupCancelled && !visible && !v.ended && v.paused && v.readyState>=2 && Date.now()<startupDeadline && Date.now()-lastStartAttempt>=1000 && !startPending){
       lastStartAttempt=Date.now();startPending=true;
       Promise.resolve(v.play()).catch(()=>{}).finally(()=>{startPending=false;});
     }
     if(eqSettings?.enabled && eqVideo!==v && v.currentSrc.startsWith('blob:'+location.origin+'/'))window.braveampEq.configure(eqSettings);
     const title=document.querySelector('h1.ytd-watch-metadata, .slim-video-metadata-title, h1')?.textContent?.trim() || document.title.replace(/ - YouTube$/,'');
     const state={title:title.slice(0,300),time:v.currentTime||0,duration:Number.isFinite(v.duration)?v.duration:0,playing:!v.paused,volume:v.volume,ended:v.ended,url:location.href};
     const key=JSON.stringify(state); if(key!==was){was=key;send('playback',{state})}
   }
   if(location.pathname.includes('/results')||location.pathname.includes('/search')||location.pathname.includes('/playlist')){
     const isPlaylist=location.pathname.includes('/playlist');
     const list=[],seen=new Set();
     for(const a of document.querySelectorAll('a[href]')){
       let u;try{u=new URL(a.href,location.href)}catch(_){continue}
       if(!['www.youtube.com','m.youtube.com','youtube.com'].includes(u.hostname))continue;
       const id=u.searchParams.get('v'),pl=u.searchParams.get('list');
       let kind='',key=''; if(id && /^[A-Za-z0-9_-]{11}$/.test(id)){kind='songs';key=id;}else if(pl && /^[A-Za-z0-9_-]+$/.test(pl)){kind='playlists';key=pl;} else continue;
       const row=a.closest('ytd-video-renderer,ytd-playlist-renderer,ytd-playlist-video-renderer,yt-lockup-view-model,ytm-video-with-context-renderer,ytm-playlist-renderer'); if(!row) continue;
       const label=row.querySelector('a#video-title')?.title || row.querySelector('a#video-title')?.textContent?.trim() || row.querySelector('h3, .yt-lockup-metadata-view-model__title, .media-item-headline')?.textContent?.trim() || a.title || a.textContent.trim();
       if(!label || label.length<4 || seen.has(key) || (isPlaylist && kind!=='songs'))continue;seen.add(key);
       list.push({id:key,kind,title:label.slice(0,250),url:kind==='songs'?'https://m.youtube.com/watch?v='+key:'https://www.youtube.com/playlist?list='+key});
       if(list.length>=35)break;
     }
     let str=JSON.stringify(list);if(str!==lastList){lastList=str;send(isPlaylist?'playlist':'results',{results:list})}
   }
 }
 setInterval(tick,500);document.addEventListener('yt-navigate-finish',tick);
})();
