(() => {
  const $ = id => document.getElementById(id);
  const cleanTitle = value => String(value || '').replace(/\bYouTube(?:\s+Music)?\b/gi,'').replace(/\s*[-–|·]\s*$/,'').trim() || 'MÚSICA';
  const panel = ['player','equalizer','playlist','library'].includes(window.BRAVEAMP_PANEL) ? window.BRAVEAMP_PANEL : new URLSearchParams(location.search).get('panel') || 'player';
  (document.getElementById(panel) || $('player')).hidden = false;
  const state = {title:'MUSICCRAFT',time:0,duration:0,playing:false,volume:70,queue:[],results:[],currentIndex:-1,shuffle:false,repeat:false,capabilities:{playback:false,search:false,playlist:false,equalizer:false},eq:{enabled:false,preamp:0,bands:Array(9).fill(0)}};
  const frequencies = ['PRE','60','170','310','600','1K','3K','6K','12K','16K'];
  const send = (command,args={}) => {
    if (window.ipc?.postMessage) window.ipc.postMessage(JSON.stringify({command,...args}));
    else console.info('[MusicCraft]',command,args);
  };
  // The title bar and equalizer graph both provide room to move the window.
  // Sliders, buttons and search fields keep their normal interaction.
  document.querySelectorAll('header, .eqgraph').forEach(handle=>{
    handle.title='Arrastra para mover esta ventana';
    handle.addEventListener('pointerdown',event=>{
      if (event.isPrimary && event.button === 0 && !event.target.closest('button,select,input')) {
        event.preventDefault(); send('drag_window');
      }
    });
  });
  for (const edge of ['n','s','e','w','nw','ne','sw','se']) {
    const grip=document.createElement('div');grip.className=`resize-grip resize-${edge}`;grip.title='Arrastrar para cambiar el tamaño';
    grip.addEventListener('pointerdown',event=>{if(event.button===0){event.preventDefault();event.stopPropagation();send('resize_window',{edge});}});
    document.body.appendChild(grip);
  }
  document.querySelectorAll('.collapse').forEach(button=>button.onclick=()=>send('collapse_window'));
  document.querySelectorAll('.min').forEach(button=>button.onclick=()=>send('window_minimize'));
  document.querySelectorAll('.close').forEach(button=>button.onclick=()=>send('window_close'));
  for (const [id,command] of Object.entries({prev:'previous',play:'play',pause:'pause',stop:'stop',next:'next',clear:'clear'})) $(id).onclick=()=>send(command);
  $('volume').oninput = event=>send('volume',{value:Number(event.target.value)});
  $('seek').onchange = event=>send('seek',{value:Number(event.target.value)});
  for (const id of ['shuffle','repeat']) $(id).onclick=()=>send(id,{value:!state[id]});
  $('toggle-eq').onclick=()=>send('toggle_window',{panel:'equalizer'});
  $('toggle-list').onclick=()=>send('toggle_window',{panel:'playlist'});
  $('toggle-library').onclick=()=>send('toggle_window',{panel:'library'});
  $('browser-toggle').onclick=()=>send('browser_toggle');
  $('browser-toggle-player').onclick=()=>send('browser_toggle');
  $('search-form').onsubmit = event=>{ event.preventDefault(); const query=$('query').value.trim(); if(query) send('search',{query,kind:document.querySelector('input[name="kind"]:checked').value}); };
  $('add-form').onsubmit = event=>{ event.preventDefault(); const url=$('url').value.trim(); if(url) send('add_url',{url}); };
  const eqSliders = $('eq-sliders');
  frequencies.forEach((frequency,index)=>{
    const label=document.createElement('label'), input=document.createElement('input'), output=document.createElement('output'), caption=document.createElement('small');
    input.type='range'; input.min='-12'; input.max='12'; input.step='0.5'; input.value='0'; input.setAttribute('aria-label',`${frequency} decibelios`); input.disabled=true;
    input.addEventListener('input',()=>{output.textContent=input.value;send('eq_change',{index,value:Number(input.value)});});
    output.textContent='0'; caption.textContent=frequency; label.append(input,output,caption); eqSliders.append(label);
  });
  $('eq-on').onclick=()=>send('eq_enabled',{value:!state.eq.enabled});
  $('eq-reset').onclick=()=>send('eq_preset',{value:'flat'});
  $('preset').onchange=event=>send('eq_preset',{value:event.target.value});
  window.initializeBraveAmpLibrary(panel,$,send,state);
  let titleValue='', updateNote='', updateAvailable=false;
  function title(text){
    text=cleanTitle(text || 'MUSICCRAFT'); if(text===titleValue) return; titleValue=text;
    $('title-text').textContent=text; $('title').title=text; $('title').setAttribute('aria-label',text); requestAnimationFrame(measureTitle);
  }
  function measureTitle(){
    const box=$('title'), track=$('track'), text=$('title-text');
    const overflow=text.getBoundingClientRect().width > box.clientWidth-6;
    track.classList.remove('scrolling'); $('title-copy').textContent=overflow?titleValue:'';
    track.style.setProperty('--travel',`-${text.getBoundingClientRect().width+28}px`);
    track.style.setProperty('--duration',`${Math.max(8,(text.getBoundingClientRect().width+28)/28)}s`);
    if(overflow){void track.offsetWidth;track.classList.add('scrolling');}
  }
  if (typeof ResizeObserver !== 'undefined') new ResizeObserver(measureTitle).observe($('title'));
  const clock=seconds=>{seconds=Math.max(0,Math.floor(Number(seconds)||0));return `${String(Math.floor(seconds/60)).padStart(2,'0')}:${String(seconds%60).padStart(2,'0')}`;};
  function renderQueue(){
    const queue=$('queue');queue.replaceChildren();
    state.queue.forEach((item,index)=>{
      const li=document.createElement('li'), play=document.createElement('button'), remove=document.createElement('button');
      const name=cleanTitle(typeof item==='string'?item:item.title || 'Sin título');
      li.classList.toggle('active',index===state.currentIndex);play.textContent=`${String(index+1).padStart(2,'0')}. ${name}`;play.title=name;
      play.disabled=!state.capabilities.playlist;play.onclick=()=>send('play_queue',{index});
      remove.textContent='×';remove.title=`Quitar ${name}`;remove.disabled=!state.capabilities.playlist;remove.onclick=()=>send('remove_queue',{index});li.append(play,remove);queue.append(li);
    });
    $('empty').hidden=state.queue.length>0;$('count').textContent=`${state.queue.length} PISTAS`;
  }
  function renderResults(){
    const results=$('results');results.replaceChildren();
    state.results.forEach((item,index)=>{
      const li=document.createElement('li'),name=document.createElement('span'),play=document.createElement('button'),add=document.createElement('button');
      name.className='result-title';name.textContent=cleanTitle(item.title || 'Sin título');li.append(name);
      if(item.kind!=='playlists'){play.textContent='▶';play.title='Reproducir';play.disabled=!state.capabilities.playlist;play.onclick=()=>send('play_result',{index});li.append(play);}
      add.textContent=item.kind==='playlists'?'+ LISTA':'+ COLA';add.disabled=!state.capabilities.playlist;add.onclick=()=>send('add_result',{index});li.append(add);results.append(li);
    });
  }
  function renderEq(){
    const eq=state.eq,values=[eq.preamp,...eq.bands];
    eqSliders.querySelectorAll('label').forEach((label,index)=>{
      const input=label.querySelector('input'),output=label.querySelector('output'),value=Number(values[index])||0;
      input.disabled=!state.capabilities.equalizer;if(document.activeElement!==input)input.value=value;
      output.textContent=`${value>0?'+':''}${value}`;input.title=`${value} dB`;
    });
    $('eq-path').setAttribute('d',eq.bands.map((gain,index)=>`${index?'L':'M'}${index*37.5} ${12-(Number(gain)||0)*.85}`).join(' '));
    $('eq-on').disabled=$('eq-reset').disabled=$('preset').disabled=!state.capabilities.equalizer;
    $('eq-on').setAttribute('aria-pressed',String(!!eq.enabled));
    $('eq-status').textContent=state.eqStatus || (state.capabilities.equalizer ? 'Audio procesado localmente · ±12 dB' : 'Ecualizador pendiente de integración.');
  }
  function render(){
    title(state.title);$('time').textContent=clock(state.time);$('status').textContent=state.status || (state.playing?'REPRODUCIENDO':'EN PAUSA');
    $('notice').textContent=updateNote || state.notice || 'Preparado para reproducir música.';document.body.classList.toggle('playing',!!state.playing);
    for(const id of ['prev','play','pause','stop','next','volume','shuffle','repeat']) $(id).disabled=!state.capabilities.playback;
    $('seek').disabled=!state.capabilities.playback || !state.duration;
    if(document.activeElement!==$('seek')){$('seek').max=Math.max(1,Number(state.duration)||0);$('seek').value=Number(state.time)||0;}
    if(document.activeElement!==$('volume')) $('volume').value=state.volume;
    for(const id of ['shuffle','repeat']) $(id).setAttribute('aria-pressed',String(!!state[id]));
    $('search-button').disabled=!state.capabilities.search;$('query').disabled=!state.capabilities.search;
    $('add-form').querySelectorAll('input,button').forEach(e=>e.disabled=!state.capabilities.playlist);
    $('clear').disabled=!state.capabilities.playlist;
    $('search-status').textContent=state.searchStatus || (state.capabilities.search?'Busca música online.':'Búsqueda pendiente de integración.');
    renderQueue();renderResults();renderEq();
    if(panel==='library' && Array.isArray(state.savedPlaylists)) {const el=$('saved-lists'),old=el.value;el.replaceChildren();state.savedPlaylists.forEach(name=>{const option=document.createElement('option');option.textContent=name;option.value=name;el.append(option)});if(state.savedPlaylists.includes(old))el.value=old;}

  }
  window.braveampUpdate = update=>{
    if(!update || typeof update!=='object')return;
    if(update.capabilities) state.capabilities={...state.capabilities,...update.capabilities};
    if(update.eq)state.eq={...state.eq,...update.eq};
    for(const key of Object.keys(update)) if(key!=='capabilities' && key!=='eq')state[key]=update[key];
    if(!Array.isArray(state.queue))state.queue=[];if(!Array.isArray(state.results))state.results=[];
    if(!Array.isArray(state.eq.bands) || state.eq.bands.length!==9)state.eq.bands=Array(9).fill(0);
    render();
  };
  $('about-open').onclick=()=>{$('about-version').textContent=window.MUSICCRAFT_VERSION||'—';$('about-dialog').hidden=false;$('about-close').focus()};
  $('about-close').onclick=()=>{$('about-dialog').hidden=true;$('about-open').focus()};
  $('about-profile').onclick=()=>send('open_author');
  $('about-dialog').onclick=e=>{if(e.target===$('about-dialog'))$('about-close').click()};
  if(panel==='player'){
    const btn=$('check-updates');let running=false;
    const say=text=>{updateNote=text;render();setTimeout(()=>{if(updateNote===text){updateNote='';render()}},12000)};
    const compare=(a,b)=>{for(let i=0;i<3;i++){let d=(a[i]||0)-(b[i]||0);if(d)return d}return 0};
    async function check(manual){
      if(running)return;running=true;if(manual)say('Consultando GitHub…');
      const os=window.MUSICCRAFT_PLATFORM||'', version=String(window.MUSICCRAFT_VERSION||'').split('.').map(Number);
      const allowed={windows:['windows','exe'],linux:['linux','tar.gz'],macos:['macos','zip']}[os];
      if(!allowed||version.length<2||version.some(n=>!Number.isInteger(n))){if(manual)say('No se conoce la versión instalada.');running=false;return}
      try{
        const ctl=new AbortController(),timer=setTimeout(()=>ctl.abort(),10000);let response;
        try{response=await fetch('https://api.github.com/repos/AdolfoCasMar/MusicCraft/releases?per_page=100',{headers:{Accept:'application/vnd.github+json'},signal:ctl.signal})}finally{clearTimeout(timer)}
        if(!response.ok)throw Error('GitHub no disponible');const data=await response.json();if(!Array.isArray(data))throw Error('Respuesta no válida');let found=false,best=null;
        for(const rel of data){const match=String(rel.tag_name||'').match(new RegExp('^'+allowed[0]+'-v(\\d+)\\.(\\d+)(?:\\.(\\d+))?$','i'));if(rel.draft||!match||!Array.isArray(rel.assets)||!rel.assets.some(x=>String(x.name||'').toLowerCase().endsWith('.'+allowed[1])))continue;found=true;const n=[+match[1],+match[2],+(match[3]||0)];if(compare(n,version)>0&&(!best||compare(n,best)>0))best=n}
        updateAvailable=!!best;btn.textContent=best?'NUEVA':'↑';btn.title=best?'Abrir descarga disponible':'Buscar actualizaciones';
        if(best)say('Nueva versión '+best.join('.')+' · pulsa NUEVA');else if(manual)say(found?'Ya tienes la versión más reciente.':'No hay versiones publicadas para este sistema.');
      }catch(_){if(manual)say('No se pudo consultar GitHub.');}finally{running=false}
    }
    btn.onclick=()=>{if(updateAvailable)send('open_updates');else check(true)};
    setTimeout(()=>check(false),5000);
  }
  render();
  send('ui_ready',{panel});
})();
