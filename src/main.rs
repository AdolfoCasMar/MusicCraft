#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use serde_json::{json, Value};
use std::{collections::HashMap, path::PathBuf};
use tao::platform::windows::WindowExtWindows;
use tao::{
    dpi::{LogicalPosition, LogicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{ResizeDirection, WindowBuilder},
};
use wry::{Rect, WebContext, WebViewBuilder};
mod consent;
mod equalizer;
mod library;
mod playback;
mod taskbar;
use playback::{js_video, load, video_id};
mod window_manager;
use window_manager::{
    arrange_panel, base_size, dock_default, open_panel_fifo, set_visible_position,
    shift_stack_below, snap_window, visible_rect, Panel, VRect,
};
#[path = "../browser/shields.rs"]
mod shields;
#[derive(Debug, Clone)]
enum AppEvent {
    Ui(String, Value),
    Browser(String, Value),
    Navigation(String, consent::NavigationKind),
}
fn emit(panels: &HashMap<String, Panel>, state: &Value) {
    let js = format!("window.braveampUpdate && window.braveampUpdate({});", state);
    for p in panels.values() {
        let _ = p.webview.evaluate_script(&js);
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
    let html = include_str!("../ui/index.html")
        .replace(
            "<link rel=\"stylesheet\" href=\"style.css\">",
            &format!("<style>{}</style>", include_str!("../ui/style.css")),
        )
        .replace(
            "<script src=\"local_audio.js\"></script>",
            &format!("<script>{}</script>", include_str!("../ui/local_audio.js")),
        )
        .replace(
            "<script src=\"app.js\"></script>",
            &format!("<script>{}</script>", include_str!("../ui/app.js")),
        );
    let cfg_dir = std::env::var_os("BRAVEAMP_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join("BraveAmpRust")
        });
    let _ = std::fs::create_dir_all(&cfg_dir);
    let mut web_context = WebContext::new(Some(cfg_dir.join("WebView2")));
    let mut panels: HashMap<String, Panel> = HashMap::new();
    for (name, y, h) in [
        ("player", 80, 155),
        ("equalizer", 235, 180),
        ("playlist", 415, 470),
        ("library", 80, 470),
    ] {
        let owner = if name == "player" {
            None
        } else {
            Some(panels["player"].window.hwnd())
        };
        let builder = WindowBuilder::new()
            .with_title(format!("MusicCraft - {name}"))
            .with_decorations(false)
            .with_resizable(true)
            .with_min_inner_size(LogicalSize::new(
                if name == "library" { 231.0 } else { 200.75 },
                (h as f64) * 0.55,
            ))
            .with_inner_size(LogicalSize::new(
                if name == "library" { 420.0 } else { 365.0 },
                h as f64,
            ))
            .with_position(LogicalPosition::new(
                if name == "library" { 465.0 } else { 100.0 },
                y as f64,
            ));
        let window = taskbar::configure(builder, owner).build(&event_loop)?;
        let id = name.to_string();
        let proxy = event_loop.create_proxy();
        let view = WebViewBuilder::new_with_web_context(&mut web_context)
            .with_html(&html)
            .with_initialization_script(format!(
                "window.BRAVEAMP_PANEL={};",
                serde_json::to_string(name)?
            ))
            .with_ipc_handler(move |r| {
                if let Ok(v) = serde_json::from_str::<Value>(r.body()) {
                    let _ = proxy.send_event(AppEvent::Ui(id.clone(), v));
                }
            })
            .build(&window)?;
        panels.insert(
            name.into(),
            Panel {
                window,
                webview: view,
            },
        );
    }
    let mut browserviews = Vec::new();
    let shields_script = shields::initialization_script();
    for mode in ["playback", "search"] {
        let proxy = event_loop.create_proxy();
        let mode_s = mode.to_string();
        let nav_proxy = event_loop.create_proxy();
        let nav_mode = mode.to_string();
        let view = WebViewBuilder::new_with_web_context(&mut web_context)
            .with_url("about:blank")
            .with_visible(false)
            .with_autoplay(true)
            .with_bounds(Rect {
                position: LogicalPosition::new(0, 20).into(),
                size: LogicalSize::new(365, 450).into(),
            })
            .with_initialization_script(shields_script.clone())
            .with_initialization_script(include_str!("youtube.js"))
            .with_navigation_handler(move |url| {
                let kind = consent::classify_navigation(&url);
                if kind != consent::NavigationKind::Other {
                    let _ = nav_proxy.send_event(AppEvent::Navigation(nav_mode.clone(), kind));
                }
                true
            })
            .with_ipc_handler(move |r| {
                let origin = r.uri().to_string();
                if !(origin.starts_with("https://www.youtube.com/")
                    || origin.starts_with("https://m.youtube.com/"))
                {
                    return;
                }
                if r.body().len() < 200_000 {
                    if let Ok(v) = serde_json::from_str::<Value>(r.body()) {
                        let _ = proxy.send_event(AppEvent::Browser(mode_s.clone(), v));
                    }
                }
            })
            .build_as_child(&panels["playlist"].window)?;
        match shields::install(&view) {
            Ok(_) => {}
            Err(e) => eprintln!("Filtro no disponible: {e}"),
        }
        browserviews.push(view);
    }
    let mut state = json!({"title":"MUSICCRAFT · RUST","status":"LISTO","notice":"Busca música en YouTube.","time":0,"duration":0,"playing":false,"volume":70,"shuffle":false,"repeat":false,"currentIndex":-1,"queue":[],"results":[],"searchStatus":"Busca canciones o playlists en YouTube.","capabilities":{"playback":true,"search":true,"playlist":true,"equalizer":true},"eqStatus":"Ecualizador para YouTube y archivos locales.","eq":{"enabled":false,"preamp":0,"bands":[0,0,0,0,0,0,0,0,0]}});
    let mut queue: Vec<Value> = Vec::new();
    let mut results: Vec<Value> = Vec::new();
    let mut current: Option<usize> = None;
    let mut showing = false;
    let mut shown_idx = 0usize;
    let mut search_kind = String::from("songs");
    let mut import_pending = false;
    let mut ended_latch = false;
    let mut consent_view: Option<usize> = None;
    let mut eq_needs_apply = false;
    let mut local_source = false;
    let layout_path = cfg_dir.join("layout.json");
    let mut saved_lists = library::PlaylistStore::open(cfg_dir.join("playlists.json"));
    state["savedPlaylists"] = json!(saved_lists.names());
    if let Ok(content) = std::fs::read_to_string(&layout_path) {
        if let Ok(layout) = serde_json::from_str::<Value>(&content) {
            for (name, p) in &panels {
                if let (Some(x), Some(y)) = (layout[name]["x"].as_i64(), layout[name]["y"].as_i64())
                {
                    p.window
                        .set_outer_position(tao::dpi::PhysicalPosition::new(x as i32, y as i32));
                }
                if let (Some(w), Some(h)) = (
                    layout[name]["width"].as_u64(),
                    layout[name]["height"].as_u64(),
                ) {
                    if (250..=4000).contains(&w) && (120..=4000).contains(&h) {
                        p.window
                            .set_inner_size(tao::dpi::PhysicalSize::new(w as u32, h as u32));
                    }
                }
            }
        }
    }
    let stored_layout = std::fs::read_to_string(&layout_path)
        .ok()
        .and_then(|v| serde_json::from_str::<Value>(&v).ok());
    let mut ui_scale = stored_layout
        .as_ref()
        .and_then(|v| v["scale"].as_f64())
        .unwrap_or(1.0)
        .clamp(0.55, 2.5);
    if stored_layout
        .as_ref()
        .and_then(|v| v["schema"].as_u64())
        .unwrap_or(0)
        < 3
    {
        ui_scale = 1.0;
        for (name, p) in &panels {
            let (w, h) = base_size(name);
            p.window.set_inner_size(LogicalSize::new(w, h));
            let _ = p.webview.zoom(ui_scale);
        }
        dock_default(&panels);
    } else {
        for p in panels.values() {
            let _ = p.webview.zoom(ui_scale);
        }
    }
    let mut pending_arrange: Option<(std::time::Instant, String)> = None;
    let mut collapsed_panels: HashMap<String, tao::dpi::PhysicalSize<u32>> = HashMap::new();
    let mut active_drag: Option<String> = None;
    let mut active_resize: Option<(String, String, HashMap<String, VRect>, f64)> = None;
    let mut scaling_until = std::time::Instant::now();
    let mut primary_follow_until = std::time::Instant::now();
    let mut secondary_order = vec![
        "equalizer".to_string(),
        "playlist".to_string(),
        "library".to_string(),
    ];
    secondary_order.sort_by_key(|name| {
        panels[name]
            .window
            .outer_position()
            .map(|p| p.y)
            .unwrap_or(i32::MAX)
    });
    let mut panel_order = vec!["player".to_string()];
    panel_order.extend(secondary_order);
    let mut last_player_pos = panels["player"].window.outer_position().ok();
    event_loop.run(move |event,_,flow|{
  let _keep_web_context_alive=&web_context;
  *flow=match &pending_arrange {Some((t,_))=>ControlFlow::WaitUntil(*t),None=>ControlFlow::Wait};
  match event {
   Event::NewEvents(tao::event::StartCause::Init)=>{arrange_panel(&panels,"equalizer");arrange_panel(&panels,"playlist");},
   Event::NewEvents(tao::event::StartCause::ResumeTimeReached{..})=>{
    if pending_arrange.as_ref().is_some_and(|(t,_)|std::time::Instant::now()>=*t){
     let mouse_down=unsafe{windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(1)}<0;
     if mouse_down {if let Some((t,_))=&mut pending_arrange{*t=std::time::Instant::now()+std::time::Duration::from_millis(160);}}
     else {
      let target=active_drag.take().or_else(||pending_arrange.as_ref().map(|(_,n)|n.clone())).unwrap_or_default();pending_arrange=None;
      if let Some((name,edge,old_rects,old_scale))=active_resize.take(){
       let (bw,bh)=base_size(&name);let p=&panels[&name];let logical=p.window.inner_size().to_logical::<f64>(p.window.scale_factor());
       let factor=(if edge=="n"||edge=="s"{logical.height/bh}else{logical.width/bw}).clamp(0.55,2.5);
       let anchor=visible_rect(&panels["player"].window).unwrap_or((0,0,0,0));let old_anchor=old_rects.get("player").copied().unwrap_or(anchor);
       scaling_until=std::time::Instant::now()+std::time::Duration::from_millis(800);ui_scale=factor;
       for (name,p) in &panels {let (w,h)=base_size(name);if collapsed_panels.contains_key(name){let dpi=p.window.scale_factor();collapsed_panels.insert(name.clone(),tao::dpi::PhysicalSize::new((w*factor*dpi).round() as u32,(h*factor*dpi).round() as u32));p.window.set_inner_size(LogicalSize::new(w*factor,25.0*factor));}else{p.window.set_inner_size(LogicalSize::new(w*factor,h*factor));}let _=p.webview.zoom(factor);}
       let mut placed=Vec::<VRect>::new();
       for name in ["player","equalizer","playlist","library"] {let p=&panels[name];if name!="player" {if let Some(old)=old_rects.get(name){let x=anchor.0+((old.0-old_anchor.0) as f64*factor/old_scale).round() as i32;let y=anchor.1+((old.1-old_anchor.1) as f64*factor/old_scale).round() as i32;let size=visible_rect(&p.window).unwrap_or((x,y,0,0));let (sx,sy)=snap_window(x,y,size.2,size.3,&placed,(20.0*p.window.scale_factor()) as i32);set_visible_position(&p.window,sx,sy);}}if let Some(r)=visible_rect(&p.window){placed.push(r);}}
       last_player_pos=panels["player"].window.outer_position().ok();
      }
      if target!="player" {if let Some(panel)=panels.get(&target){if panel.window.is_visible(){
       if let Some(pos)=visible_rect(&panel.window){let margin=(12.0*panel.window.scale_factor()) as i32;
        let mut nearby:Vec<_>=panels.iter().filter(|(n,p)|*n!=&target && p.window.is_visible()).collect();nearby.sort_by_key(|(n,_)|if n.as_str()=="player"{0}else{1});
        let others:Vec<_>=nearby.into_iter().filter_map(|(_,p)|visible_rect(&p.window)).collect();
        let (x,y)=snap_window(pos.0,pos.1,pos.2,pos.3,&others,margin);
        if (x,y)!=(pos.0,pos.1){set_visible_position(&panel.window,x,y);}
       }
       arrange_panel(&panels,&target);
      }}}
      if collapsed_panels.is_empty(){let mut layout=json!({"schema":3,"scale":ui_scale});for (name,p) in &panels {if let Ok(pos)=p.window.outer_position(){let size=p.window.inner_size();layout[name]=json!({"x":pos.x,"y":pos.y,"width":size.width,"height":size.height});}}let _=std::fs::write(&layout_path,layout.to_string());}
     }
    }
   },
   Event::WindowEvent{window_id,event:WindowEvent::CloseRequested,..}=>{if panels["player"].window.id()==window_id{*flow=ControlFlow::Exit}else{for p in panels.values(){if p.window.id()==window_id{p.window.set_visible(false)}}}},
   Event::WindowEvent{window_id,event:WindowEvent::Resized(size),..}=>{
    if panels["playlist"].window.id()==window_id {let scale=panels["playlist"].window.scale_factor();let logical=size.to_logical::<f64>(scale);for web in &browserviews {let _=web.set_bounds(Rect{position:LogicalPosition::new(0.0,20.0).into(),size:LogicalSize::new(logical.width,(logical.height-20.0).max(1.0)).into()});}}
    if std::time::Instant::now()>=scaling_until {if let Some((name,_))=panels.iter().find(|(_,p)|p.window.id()==window_id){pending_arrange=Some((std::time::Instant::now()+std::time::Duration::from_millis(220),name.clone()));}}
   },
   Event::WindowEvent{window_id,event:WindowEvent::Moved(pos),..}=>{
    if std::time::Instant::now()<scaling_until {return}
    if let Some((name,panel))=panels.iter().find(|(_,p)|p.window.id()==window_id) {
     if panel.window.outer_position().ok()!=Some(pos) {return}
     if name=="player" {
      if let Some(old)=last_player_pos {let (dx,dy)=(pos.x-old.x,pos.y-old.y); if dx!=0 || dy!=0 {for (other,p) in &panels {if other!="player" {if let Ok(pt)=p.window.outer_position(){p.window.set_outer_position(tao::dpi::PhysicalPosition::new(pt.x+dx,pt.y+dy));}}}}}
      last_player_pos=Some(pos);primary_follow_until=std::time::Instant::now()+std::time::Duration::from_millis(500);
     }

    }
    let moved_name=panels.iter().find(|(_,p)|p.window.id()==window_id).map(|(n,_)|n.clone()).unwrap_or_default();
    let target=active_drag.clone().unwrap_or_else(||if moved_name!="player" && std::time::Instant::now()<primary_follow_until {"player".into()}else{moved_name});
    pending_arrange=Some((std::time::Instant::now()+std::time::Duration::from_millis(220),target));
   },
   Event::UserEvent(AppEvent::Ui(panel,msg))=>{
    let cmd=msg["command"].as_str().unwrap_or("");let index=msg["index"].as_u64().unwrap_or(u64::MAX) as usize;let val=msg["value"].as_f64().unwrap_or(0.0);
    match cmd {
     "ui_ready"=>{},
     "drag_window"=>{active_drag=Some(panel.clone());pending_arrange=Some((std::time::Instant::now()+std::time::Duration::from_millis(220),panel.clone()));if let Some(p)=panels.get(&panel){let _=p.window.drag_window();}},
     #[cfg(debug_assertions)]
     "debug_scale"=>{let factor=msg["value"].as_f64().unwrap_or(1.0).clamp(0.55,2.5);let snapshot=panels.iter().filter_map(|(n,p)|visible_rect(&p.window).map(|r|(n.clone(),r))).collect();active_resize=Some(("player".to_string(),"se".to_string(),snapshot,ui_scale));pending_arrange=Some((std::time::Instant::now()+std::time::Duration::from_millis(220),"player".to_string()));let (w,h)=base_size("player");panels["player"].window.set_inner_size(LogicalSize::new(w*factor,h*factor));},
     "resize_window"=>{let edge=msg["edge"].as_str().unwrap_or("").to_owned();let direction=match edge.as_str(){"n"=>Some(ResizeDirection::North),"s"=>Some(ResizeDirection::South),"e"=>Some(ResizeDirection::East),"w"=>Some(ResizeDirection::West),"ne"=>Some(ResizeDirection::NorthEast),"nw"=>Some(ResizeDirection::NorthWest),"se"=>Some(ResizeDirection::SouthEast),"sw"=>Some(ResizeDirection::SouthWest),_=>None};if let (Some(p),Some(d))=(panels.get(&panel),direction){let snapshot=panels.iter().filter_map(|(n,p)|visible_rect(&p.window).map(|r|(n.clone(),r))).collect();active_resize=Some((panel.clone(),edge,snapshot,ui_scale));pending_arrange=Some((std::time::Instant::now()+std::time::Duration::from_millis(220),panel.clone()));let _=p.window.drag_resize_window(d);}},
     "collapse_window"=>{if panel!="player" {if let Some(p)=panels.get(&panel){if let Some(old)=visible_rect(&p.window){
      let collapse=!collapsed_panels.contains_key(&panel);scaling_until=std::time::Instant::now()+std::time::Duration::from_millis(500);
      if collapse {let size=p.window.inner_size();collapsed_panels.insert(panel.clone(),size);let height=(25.0*ui_scale*p.window.scale_factor()).round() as u32;p.window.set_inner_size(tao::dpi::PhysicalSize::new(size.width,height));}
      else if let Some(size)=collapsed_panels.remove(&panel){p.window.set_inner_size(size);}
      let _=p.webview.evaluate_script(if collapse{"document.body.classList.add('panel-collapsed');document.querySelector('.panel:not([hidden]) .collapse').textContent='+';"}else{"document.body.classList.remove('panel-collapsed');document.querySelector('.panel:not([hidden]) .collapse').textContent='−';"});
      if let Some(new)=visible_rect(&p.window){shift_stack_below(&panels,&panel,old,(new.1+new.3)-(old.1+old.3));}
      last_player_pos=panels["player"].window.outer_position().ok();if !collapse && collapsed_panels.is_empty(){pending_arrange=Some((std::time::Instant::now()+std::time::Duration::from_millis(600),panel.clone()));}
     }}}},
     "window_close"=>{if panel=="player"{*flow=ControlFlow::Exit}else if let Some(p)=panels.get(&panel){p.window.set_visible(false);panel_order.retain(|n|n!=&panel);}},
     "window_minimize"=>{if let Some(p)=panels.get(&panel){p.window.set_minimized(true);}},
     "toggle_window"=>{if let Some(name)=msg["panel"].as_str(){if let Some(p)=panels.get(name){if p.window.is_visible(){p.window.set_visible(false);panel_order.retain(|n|n!=name);}else{open_panel_fifo(&panels,name,&mut panel_order,&mut last_player_pos);}}}},
     "browser_toggle"=>{consent_view=None;showing=!showing;if showing && !panels["playlist"].window.is_visible(){open_panel_fifo(&panels,"playlist",&mut panel_order,&mut last_player_pos);}let _=browserviews[shown_idx].set_visible(showing);state["notice"]=json!(if showing{"YouTube integrado: acepta cookies o inicia la reproducción."}else{"YouTube oculto dentro de la aplicación."});},
     "search"=>{let q=msg["query"].as_str().unwrap_or("").trim();if !q.is_empty(){search_kind=msg["kind"].as_str().unwrap_or("songs").to_string();let mut u=url::Url::parse("https://www.youtube.com/results").unwrap();u.query_pairs_mut().append_pair("search_query",q).append_pair("sp",if search_kind=="playlists"{"EgIQAw=="}else{"EgIQAQ=="});let _=browserviews[1].load_url(u.as_str());results.clear();state["results"]=json!([]);state["searchStatus"]=json!("Buscando en YouTube… Si pide cookies, pulsa VER YOUTUBE.");if showing{let _=browserviews[shown_idx].set_visible(false);}shown_idx=1;if showing{let _=browserviews[shown_idx].set_visible(true);}}},
     "local_add"=>{if panel=="library"{if let (Some(id),Some(title))=(msg["index"].as_u64(),msg["title"].as_str()){if title.len()<512 {let item=json!({"id":id,"title":title,"kind":"local"});queue.push(item);if msg["play"].as_bool().unwrap_or(false){current=Some(queue.len()-1);local_source=true;state["capabilities"]["equalizer"]=json!(true);state["eqStatus"]=json!("Ecualizador activo para archivos locales.");state["title"]=json!(title);load(queue.last().unwrap(),&browserviews[0],&panels["library"].webview);}}}}},
     "local_state"=>{if panel=="library" && local_source {for key in ["time","duration","playing"]{if !msg[key].is_null(){state[key]=msg[key].clone();}}state["status"]=json!(if msg["playing"].as_bool().unwrap_or(false){"REPRODUCIENDO"}else{"EN PAUSA"});
      let ended=msg["ended"].as_bool().unwrap_or(false);if ended && !ended_latch && !queue.is_empty(){ended_latch=true;if let Some(n)=playback::next_from_state(current,queue.len(),&state){current=Some(n);let item=&queue[n];local_source=item["kind"]=="local";state["capabilities"]["equalizer"]=json!(true);state["eqStatus"]=json!(if local_source{"Ecualizador activo para archivos locales."}else{"Ecualizador disponible para YouTube."});state["title"]=item["title"].clone();load(item,&browserviews[0],&panels["library"].webview);}}if !ended{ended_latch=false;}
     }},
     "save_playlist"=>{if let Some(n)=msg["name"].as_str(){match saved_lists.save(n,&queue){Ok(excluded)=>{state["savedPlaylists"]=json!(saved_lists.names());state["notice"]=json!(if excluded>0{"Lista guardada; archivos locales no incluidos."}else{"Playlist guardada."});},Err(e)=>state["notice"]=json!(format!("No se pudo guardar: {e}"))}}},
     "load_playlist"=>{if let Some(items)=msg["name"].as_str().and_then(|n|saved_lists.get(n)){queue.extend(items.clone());state["notice"]=json!("Playlist añadida a la cola.");}},
     "delete_playlist"=>{if let Some(n)=msg["name"].as_str(){match saved_lists.delete(n){Ok(())=>state["savedPlaylists"]=json!(saved_lists.names()),Err(e)=>state["notice"]=json!(format!("No se pudo borrar: {e}"))}}},
     "eq_change"|"eq_enabled"|"eq_preset"=>{equalizer::adjust(&mut state["eq"],cmd,index,val,&msg);equalizer::apply(&state["eq"],local_source,&browserviews[0],&panels["library"].webview);},
     "add_url"=>{if let Some(id)=msg["url"].as_str().and_then(video_id){let item=json!({"id":id,"title":format!("YouTube · {id}"),"kind":"songs"});queue.push(item);state["notice"]=json!("Añadido a la cola.");}else{state["notice"]=json!("Introduce un enlace de vídeo individual de YouTube.");}},
     "add_result"|"play_result"=>{if let Some(item)=results.get(index){if item["kind"]=="songs" {queue.push(item.clone());if cmd=="play_result"{current=Some(queue.len()-1);if showing{let _=browserviews[shown_idx].set_visible(false);}shown_idx=0;if showing{let _=browserviews[0].set_visible(true);}if load(item,&browserviews[0],&panels["library"].webview){state["status"]=json!("CARGANDO");state["notice"]=json!("Si YouTube pide confirmación, pulsa VER YOUTUBE.");}}}else if let Some(id)=item["id"].as_str(){
       if id.len()<160 && id.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'_'||c==b'-'){
        import_pending=true;let _=browserviews[1].load_url(&format!("https://www.youtube.com/playlist?list={id}"));
        state["searchStatus"]=json!("Cargando playlist… Si pide cookies, pulsa YT en el reproductor.");
       }
      }}},
     "play_queue"=>{if let Some(item)=queue.get(index){current=Some(index);if showing{let _=browserviews[shown_idx].set_visible(false);}shown_idx=0;if showing{let _=browserviews[0].set_visible(true);}load(item,&browserviews[0],&panels["library"].webview);state["status"]=json!("CARGANDO");}},
     "remove_queue"=>{if index<queue.len(){queue.remove(index);current=match current{Some(i) if i==index=>None,Some(i) if i>index=>Some(i-1),x=>x};}},
     "clear"=>{queue.clear();current=None;},
     "previous"|"next"=>{if !queue.is_empty(){let i=match (cmd,current){("previous",Some(i))=>i.saturating_sub(1),("next",Some(i))=>(i+1).min(queue.len()-1),_=>0};current=Some(i);load(&queue[i],&browserviews[0],&panels["library"].webview);}},
     "play"|"pause"|"stop"=>{if cmd=="play" && current.is_none() && !queue.is_empty(){current=Some(0);load(&queue[0],&browserviews[0],&panels["library"].webview);}else if local_source{let method=if cmd=="play"{"resume"}else{cmd};let _=panels["library"].webview.evaluate_script(&format!("window.localAudio&&window.localAudio.{method}();"));}else{js_video(&browserviews[0],cmd,0.0);}},
     "seek"=>{if local_source{let _=panels["library"].webview.evaluate_script(&format!("window.localAudio&&window.localAudio.seek({});",val.max(0.0)));}else{js_video(&browserviews[0],"seek",val);}},
     "volume"=>{state["volume"]=json!(val.clamp(0.0,100.0));let v=val.clamp(0.0,100.0)/100.0;if local_source{let _=panels["library"].webview.evaluate_script(&format!("window.localAudio&&window.localAudio.volume({v});"));}else{js_video(&browserviews[0],"volume",v);}},
     "shuffle"|"repeat"=>{state[cmd]=json!(!state[cmd].as_bool().unwrap_or(false));},
     _=>{}
    }
    if let Some(i)=current {if let Some(item)=queue.get(i){let next_local=item["kind"]=="local";if next_local{state["title"]=item["title"].clone();}if next_local!=local_source{local_source=next_local;state["capabilities"]["equalizer"]=json!(true);state["eqStatus"]=json!(if local_source{"Ecualizador activo para archivos locales."}else{"Ecualizador disponible para YouTube."});}}}
    state["queue"]=json!(queue);state["currentIndex"]=json!(current.map(|i|i as i64).unwrap_or(-1));emit(&panels,&state);
   },
   Event::UserEvent(AppEvent::Navigation(mode,kind))=>{
    let idx=if mode=="search"{1}else{0};
    if idx==0 && kind==consent::NavigationKind::Youtube {eq_needs_apply=true;}
    if kind==consent::NavigationKind::Consent {
     if showing && shown_idx!=idx {let _=browserviews[shown_idx].set_visible(false);}
     if !panels["playlist"].window.is_visible(){open_panel_fifo(&panels,"playlist",&mut panel_order,&mut last_player_pos);}
     shown_idx=idx;showing=true;consent_view=Some(idx);let _=browserviews[idx].set_visible(true);
     state["notice"]=json!("YouTube pide elegir cookies en el panel de playlist.");
     state["searchStatus"]=json!("Elige tus preferencias de cookies en YouTube para continuar.");
    } else if consent_view==Some(idx) {
     state["notice"]=json!("Cuando termines en YouTube, pulsa YT para ocultar la página.");
     state["searchStatus"]=json!("Continúa en YouTube; pulsa YT para ocultar la página.");
    }
    emit(&panels,&state);
   },
   Event::UserEvent(AppEvent::Browser(mode,msg))=>{
    let idx=if mode=="search"{1}else{0};
    if msg["type"]=="eq_error" && idx==0 {state["eqStatus"]=json!(msg["message"].as_str().unwrap_or("No se pudo activar el ecualizador para este vídeo."));}
    if msg["type"]=="eq_ready" && idx==0 {state["eqStatus"]=json!("Ecualizador conectado al audio de YouTube.");}
    if msg["type"]=="consent_required" {
     if showing && shown_idx!=idx {let _=browserviews[shown_idx].set_visible(false);}
     if !panels["playlist"].window.is_visible(){open_panel_fifo(&panels,"playlist",&mut panel_order,&mut last_player_pos);}
     shown_idx=idx;showing=true;consent_view=Some(idx);let _=browserviews[idx].set_visible(true);
     state["notice"]=json!("YouTube pide elegir cookies en el panel de playlist.");
     state["searchStatus"]=json!("Elige tus preferencias de cookies en YouTube para continuar.");
    }
    if msg["type"]=="consent_resolved" && consent_view==Some(idx) {
     let _=browserviews[idx].set_visible(false);showing=false;consent_view=None;
     state["notice"]=json!("Continuando en YouTube…");
    }
    if mode=="search" && msg["type"]=="playlist" && import_pending {
     if let Some(items)=msg["results"].as_array(){ if !items.is_empty() {let count=items.len();queue.extend(items.iter().filter(|x|x["kind"]=="songs").cloned());import_pending=false;state["queue"]=json!(queue);state["searchStatus"]=json!(format!("{count} canciones añadidas; se importan las entradas cargadas inicialmente."));}}
    }
    if mode=="search" && msg["type"]=="results" {if let Some(items)=msg["results"].as_array(){results=items.iter().filter(|x|x["kind"].as_str()==Some(search_kind.as_str())).take(35).cloned().collect();state["results"]=json!(results);state["searchStatus"]=json!(format!("{} resultados. Los vídeos pueden incluir contenido no musical.",results.len()));}}
    if mode=="playback" && msg["type"]=="playback" && !local_source {if eq_needs_apply && state["eq"]["enabled"].as_bool().unwrap_or(false){let _=browserviews[0].evaluate_script(&format!("window.braveampEq&&window.braveampEq.configure({});",state["eq"]));eq_needs_apply=false;}let s=&msg["state"];for key in ["title","time","duration","playing"]{if !s[key].is_null(){state[key]=s[key].clone();}}if let Some(v)=s["volume"].as_f64(){state["volume"]=json!((v*100.0).round());}state["status"]=json!(if s["playing"].as_bool().unwrap_or(false){"REPRODUCIENDO"}else{"EN PAUSA"});
     let ended=s["ended"].as_bool().unwrap_or(false);
     if ended && !ended_latch && !queue.is_empty() {ended_latch=true;if let Some(n)=playback::next_from_state(current,queue.len(),&state){current=Some(n);state["currentIndex"]=json!(n);local_source=queue[n]["kind"]=="local";state["capabilities"]["equalizer"]=json!(true);state["eqStatus"]=json!(if local_source{"Ecualizador activo para archivos locales."}else{"Ecualizador disponible para YouTube."});state["title"]=queue[n]["title"].clone();load(&queue[n],&browserviews[0],&panels["library"].webview);}}
     if !ended {ended_latch=false;}
    }
    emit(&panels,&state);
   },
   _=>{}
  }
 });
}
