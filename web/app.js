'use strict';
const $=s=>document.querySelector(s),points=new Map();
let ws,ready=false,button=0,seq=0,raf=0,manual=false,retryTimer,latency=0,awaiting=new Map();
let token=localStorage.getItem('glide-token')||'';
const taps=new TapFeedback();
let prefs={haptics:true,hapticDuration:25,dots:false,theme:'system',cursorSpeed:1,scrollSpeed:1};
for(const key of ['cursorSpeed','scrollSpeed']) { const value=Number(prefs[key]); prefs[key]=Number.isFinite(value)?Math.max(.25,Math.min(2,value)):1; }
function speeds(){for(const [id,key] of [['cursor-speed','cursorSpeed'],['scroll-speed','scrollSpeed']]){$('#'+id).value=prefs[key];$('#'+id+'-value').textContent=Number(prefs[key].toFixed(2))+'×';$('#'+id).setAttribute('aria-valuetext',Number(prefs[key].toFixed(2))+' times');}}
function appearance(){speeds();hapticDurationLabel();$('#haptics').checked=prefs.haptics;$('#haptics-support').textContent=typeof navigator.vibrate==='function'?'Brief vibration on taps. Your phone or browser may suppress it.':'Vibration is not supported by this browser.';document.documentElement.dataset.theme=prefs.theme;$('#show-points').checked=prefs.dots;$('#theme').value=prefs.theme;draw()}
function saveSettings(){if(!ready||ws?.readyState!==1)return;$('#settings-status').textContent='Saving to desktop…';ws.send(JSON.stringify({settings:{cursor_speed:prefs.cursorSpeed,scroll_speed:prefs.scrollSpeed,show_points:prefs.dots,haptics:prefs.haptics,haptic_duration_ms:prefs.hapticDuration,theme:prefs.theme}}))}
function draw(){ $('#dots').replaceChildren();$('#pad').classList.toggle('active',points.size>0);if(prefs.dots)for(const p of points.values()){const n=document.createElement('i');n.className='dot';n.style.left=p.x*100+'%';n.style.top=p.y*100+'%';$('#dots').append(n)}}
function state(text,online=false){$('#connection').textContent=text;$('#connection').classList.toggle('online',online);$('#reconnect').hidden=online||!token;$('#disconnect').hidden=!online;for(const id of ['cursor-speed','scroll-speed','reset-speeds','show-points','haptics','haptic-duration','theme'])$('#'+id).disabled=!online;if(!online)$('#settings-status').textContent='Connect to edit device settings.'}
function send(){cancelAnimationFrame(raf);raf=0;if(!ready||ws?.readyState!==1)return;if(ws.bufferedAmount>32768){disconnect();state('Network too slow');return}const id=++seq;awaiting.set(id,performance.now());if(awaiting.size>128)awaiting.delete(awaiting.keys().next().value);const bounds=pad.getBoundingClientRect();ws.send(JSON.stringify({points:[...points.values()],button,seq:id,surface:{width:Math.max(1,bounds.width),height:Math.max(1,bounds.height)}}))}
function clear(){taps.reset();points.clear();button=0;draw();send()}
function disconnect(){manual=true;clear();ready=false;clearTimeout(retryTimer);ws?.close();state('Disconnected');$('#feedback').textContent='Input released'}
function connect(){if(!token||document.hidden||ws&&ws.readyState<2)return;manual=false;state('Connecting…');$('#pairing').hidden=true;$('#controller').hidden=false;ws=new WebSocket(`ws://${location.host}/ws`);ws.onopen=()=>ws.send(JSON.stringify({token}));ws.onmessage=e=>{const m=JSON.parse(e.data);if(m.settings){prefs.cursorSpeed=m.settings.cursor_speed;prefs.scrollSpeed=m.settings.scroll_speed;prefs.dots=m.settings.show_points;prefs.haptics=m.settings.haptics??true;prefs.hapticDuration=m.settings.haptic_duration_ms??25;prefs.theme=m.settings.theme;appearance();$('#settings-status').textContent='Saved on your desktop'}if(m.settings_error){$('#settings-status').textContent=m.settings_error}if(m.ready){ready=true;state('Connected',true);$('#feedback').textContent='Ready to glide'}if(m.error){manual=true;$('#feedback').textContent=m.error}if(m.ack&&awaiting.has(m.ack)){latency=Math.round(performance.now()-awaiting.get(m.ack));awaiting.delete(m.ack);$('#feedback').textContent=`${latency} ms · ${m.contacts?m.contacts+' fingers':'Connected to your desktop'}`}};ws.onclose=()=>{ready=false;taps.reset();points.clear();button=0;draw();state('Disconnected');if(!manual&&!document.hidden)retryTimer=setTimeout(connect,1800)};ws.onerror=()=>state('Desktop unavailable')}
$('#pair-form').onsubmit=async e=>{e.preventDefault();const b=e.target.querySelector('button');b.disabled=true;$('#pair-error').textContent='';try{const r=await fetch('/api/pair',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({name:$('#name').value,code:$('#code').value,...($('#profile-choice').hidden?{}:{profile_id:$('#profile').value})})});if(!r.ok)throw Error(await r.text());const data=await r.json();if(data.profiles){const select=$('#profile');select.replaceChildren(new Option('Set up a new device',''));for(const profile of data.profiles)select.add(new Option(profile.name,profile.id));$('#profile-choice').hidden=false;b.textContent='Connect with this profile';return;}token=data.token;localStorage.setItem('glide-token',token);connect()}catch(e){$('#pair-error').textContent=e.message}finally{b.disabled=false}};
const pad=$('#pad');function update(e){const r=pad.getBoundingClientRect();points.set(e.pointerId,{id:e.pointerId,x:Math.max(0,Math.min(1,(e.clientX-r.left)/r.width)),y:Math.max(0,Math.min(1,(e.clientY-r.top)/r.height))});draw()}
pad.onpointerdown=e=>{e.preventDefault();if(!ready||points.size>=5)return;pad.setPointerCapture(e.pointerId);taps.down(e);update(e);send()};pad.onpointermove=e=>{if(!points.has(e.pointerId))return;e.preventDefault();taps.move(e);update(e);if(!raf)raf=requestAnimationFrame(send)};function up(e){if(!points.has(e.pointerId))return;const tapped=taps.up(e,e.type!=='pointerup');points.delete(e.pointerId);draw();send();if(tapped&&ready&&prefs.haptics)tapPulse(navigator,prefs.hapticDuration)}pad.onpointerup=pad.onpointercancel=pad.onlostpointercapture=up;pad.oncontextmenu=e=>e.preventDefault();
$('#menu-button').onclick=()=>{clear();$('#controls').showModal()};$('#close-menu').onclick=()=>$('#controls').close();$('#controls').onclick=e=>{if(e.target===$('#controls')){const r=e.target.getBoundingClientRect();if(e.clientX<r.left||e.clientX>r.right||e.clientY<r.top||e.clientY>r.bottom)e.target.close()}};$('#disconnect').onclick=()=>{disconnect();$('#controls').close()};$('#reconnect').onclick=()=>{connect();$('#controls').close()};$('#settings-button').onclick=()=>{$('#controls').close();clear();$('#settings').showModal()};$('#close-settings').onclick=()=>$('#settings').close();$('#show-points').onchange=e=>{prefs.dots=e.target.checked;appearance();saveSettings()};$('#haptics').onchange=e=>{prefs.haptics=e.target.checked;saveSettings()};$('#theme').onchange=e=>{prefs.theme=e.target.value;appearance();saveSettings()};$('#forget').onclick=()=>{disconnect();token='';localStorage.removeItem('glide-token');$('#controller').hidden=true;$('#pairing').hidden=false;$('#settings').close();$('#code').value='';$('#profile-choice').hidden=true;$('#pair-form button').textContent='Connect to desktop';state('Not connected')};
document.addEventListener('visibilitychange',()=>{if(document.hidden){clear();ready=false;ws?.close()}else if(!manual)connect()});window.addEventListener('pagehide',disconnect);window.addEventListener('blur',clear);window.addEventListener('resize',clear);setInterval(()=>{if(ready)send()},400);
for(const [id,key] of [['cursor-speed','cursorSpeed'],['scroll-speed','scrollSpeed']]) {$('#'+id).oninput=e=>{prefs[key]=Number(e.target.value);appearance()};$('#'+id).onchange=saveSettings;}
$('#reset-speeds').onclick=()=>{prefs.cursorSpeed=prefs.scrollSpeed=1;appearance();saveSettings()};
appearance();state('Not connected');if(token)connect();

// Fullscreen must be requested directly from a user gesture. Keep the button in
// the fullscreen element so there is always an explicit way back out.
let noticeTimer;
function notice(text) { $('#notice').textContent=text;$('#notice').hidden=false;clearTimeout(noticeTimer);noticeTimer=setTimeout(()=>$('#notice').hidden=true,6000); }
function fullscreenActive(){return Boolean(document.fullscreenElement||document.webkitFullscreenElement)}
function syncFullscreen(){
    const active=fullscreenActive(),label=active?'Exit fullscreen':'Enter fullscreen';
    $('#fullscreen-label').textContent=label;$('#fullscreen').setAttribute('aria-label',label);$('#fullscreen').title=label;
    $('#fullscreen').setAttribute('aria-pressed',String(active));
    $('#fullscreen-path').setAttribute('d',active?'M3 8h5V3m8 0v5h5M8 21v-5H3m18 0h-5v5':'M8 3H3v5m13-5h5v5M3 16v5h5m13-5v5h-5');
    clear();
}
$('#fullscreen').onclick=async()=>{
    $('#controls').close();clear();
    try {
        if(fullscreenActive()){
            const exit=document.exitFullscreen||document.webkitExitFullscreen;
            if(exit)await exit.call(document);
        }else{
            const root=document.documentElement;
            if(root.requestFullscreen)await root.requestFullscreen({navigationUI:'hide'});
            else if(root.webkitRequestFullscreen)await root.webkitRequestFullscreen();
            else notice('This browser does not support page fullscreen. Try opening Glide in Chrome or Brave.');
        }
    }catch {notice('Fullscreen could not open. Try again, or use your browser’s fullscreen option.');}
    syncFullscreen();
};
document.addEventListener('fullscreenchange',syncFullscreen);
document.addEventListener('webkitfullscreenchange',syncFullscreen);

// An explicit button provides user activation and separates phone/browser
// support from touchpad tap recognition. API success is not hardware proof.
function testHaptics(){
    let message;
    if(typeof navigator.vibrate!=='function') {
        message='This browser does not support vibration.';
    } else if(tapPulse(navigator,prefs.hapticDuration)) {
        message=`The browser accepted a ${prefs.hapticDuration} ms pulse. If you felt nothing, try a longer pulse or check your phone’s vibration settings.`;
    } else {
        message='The browser rejected vibration. Check this site’s sound/vibration permissions or try another browser.';
    }
    $('#haptics-result').textContent=message;
};

function hapticDurationLabel(){
    $('#haptic-duration').value=prefs.hapticDuration;
    $('#haptic-duration-value').textContent=`${prefs.hapticDuration} ms`;
    $('#haptic-duration').setAttribute('aria-valuetext',`${prefs.hapticDuration} milliseconds`);
}
$('#haptic-duration').oninput=e=>{
    prefs.hapticDuration=Math.max(5,Math.min(100,Math.round(Number(e.target.value)||25)));
    hapticDurationLabel();
};
$('#haptic-duration').onchange=()=>{
    saveSettings();
    if(prefs.haptics)testHaptics();
};
$('#test-haptics').onclick=testHaptics;
