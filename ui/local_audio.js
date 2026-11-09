/* Local file playback and audio graph. */
window.initializeBraveAmpLibrary = (panel,$,send,state) => {
  if(panel!=='library')return;
  let mediaFiles=[];
    const audio=new Audio();let audioContext,filters=[],preamp,currentUrl;
    function ensureAudio(){
      if(audioContext)return;
      audioContext=new AudioContext();let source=audioContext.createMediaElementSource(audio);preamp=audioContext.createGain();source.connect(preamp);let previous=preamp;
      [60,170,310,600,1000,3000,6000,12000,16000].forEach((frequency,i)=>{let f=audioContext.createBiquadFilter();f.type=i===0?'lowshelf':i===8?'highshelf':'peaking';f.frequency.value=frequency;f.Q.value=1;previous.connect(f);previous=f;filters.push(f)});previous.connect(audioContext.destination);
    }
    function apply(eq){ensureAudio();const now=audioContext.currentTime;preamp.gain.setTargetAtTime(eq.enabled?10**((Number(eq.preamp)||0)/20):1,now,.02);filters.forEach((f,i)=>f.gain.setTargetAtTime(eq.enabled?(Number(eq.bands[i])||0):0,now,.02));}
    function report(){send('local_state',{time:audio.currentTime||0,duration:Number.isFinite(audio.duration)?audio.duration:0,playing:!audio.paused,ended:audio.ended});}
    for(const e of ['play','pause','timeupdate','ended','loadedmetadata'])audio.addEventListener(e,report);
    window.localAudio={
      play:async index=>{const file=mediaFiles[index];if(!file)return;ensureAudio();await audioContext.resume();if(currentUrl)URL.revokeObjectURL(currentUrl);currentUrl=URL.createObjectURL(file);audio.src=currentUrl;audio.volume=Math.max(0,Math.min(1,Number(state.volume)/100));apply(state.eq);await audio.play();},
      resume:()=>{if(audioContext)audioContext.resume();return audio.play()},pause:()=>audio.pause(),stop:()=>{audio.pause();audio.currentTime=0},seek:value=>{audio.currentTime=Math.max(0,Number(value)||0)},volume:value=>{audio.volume=Math.max(0,Math.min(1,Number(value)||0))},apply,
    };
    function renderFiles(){const list=$('local-list');list.replaceChildren();mediaFiles.forEach((f,i)=>{const li=document.createElement('li'),label=document.createElement('span'),play=document.createElement('button'),add=document.createElement('button');label.textContent=f.name;label.title=f.name;play.textContent='▶';add.textContent='+';play.onclick=()=>send('local_add',{index:i,title:f.name,play:true});add.onclick=()=>send('local_add',{index:i,title:f.name,play:false});li.append(label,play,add);list.append(li);});$('lib-status').textContent=`${mediaFiles.length} archivos disponibles`;}
    $('local-files').onchange=e=>{mediaFiles.push(...Array.from(e.target.files));e.target.value='';renderFiles();};
    $('lib-add-all').onclick=()=>mediaFiles.forEach((f,i)=>send('local_add',{index:i,title:f.name,play:false}));
    $('save-list').onclick=()=>send('save_playlist',{name:$('playlist-name').value.trim()});
    $('load-list').onclick=()=>send('load_playlist',{name:$('saved-lists').value});
    $('delete-list').onclick=()=>send('delete_playlist',{name:$('saved-lists').value});
};
