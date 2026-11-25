'use strict';
(()=>{
    const idle='Hear your desktop’s sound on this device while it is connected.';
    let socket=null,context=null,player=null,retry=0,problem='';
    // Wait for the desktop to confirm the setting before opening the stream.
    const wanted=()=>ready&&prefs.audio&&audioSaved&&!document.hidden;
    const status=text=>{$('#audio-status').textContent=text};
    function stop(){
        clearTimeout(retry);player=null;
        if(socket){socket.onclose=null;socket.close();socket=null}
    }
    // Browsers only start audio from a user gesture, so every tap tries this.
    function unlock(){
        if(!prefs.audio)return;
        if(!context){
            const Context=window.AudioContext||window.webkitAudioContext;
            if(!Context){status('This browser cannot play desktop audio.');return}
            // Matching the stream rate avoids per-frame resampling clicks.
            try{context=new Context({sampleRate:48000,latencyHint:'interactive'})}catch{context=new Context()}
            context.onstatechange=sync;
        }
        if(context.state!=='running')context.resume().then(sync,()=>{});
    }
    function start(){
        const mine=socket=new WebSocket(wsUrl('/audio'));
        mine.binaryType='arraybuffer';
        mine.onopen=()=>mine.send(JSON.stringify({token}));
        mine.onmessage=e=>{
            if(typeof e.data!=='string'){player?.push(e.data);return}
            const m=JSON.parse(e.data);
            if(m.audio){problem='';player=new PcmPlayer(context,m.audio.rate,m.audio.channels);status('Playing desktop audio. Use your phone’s volume buttons.')}
            if(m.error){problem=m.error;status(m.error)}
        };
        mine.onclose=()=>{
            if(socket!==mine)return;
            socket=null;player=null;
            if(wanted())retry=setTimeout(sync,problem?5000:1500);
        };
    }
    function sync(){
        $('#audio').checked=prefs.audio;$('#audio').disabled=!ready;
        if(!wanted()){stop();problem='';status(idle)}
        else if(!context||context.state!=='running'){stop();status('Tap anywhere to start desktop audio.')}
        else if(!socket){clearTimeout(retry);if(!problem)status('Starting desktop audio…');start()}
        stats();
    }
    // The delay covers this device's queue and speaker path plus a fixed
    // allowance for desktop capture and framing; Wi-Fi transit is not measured.
    function stats(){
        const playing=player&&context?.state==='running'&&player.next>0;
        if(!playing){$('#audio-stats').textContent='';padStatus('audio','');return}
        const delay=Math.round((player.delay+.02)*1000),queue=Math.round(Math.max(0,player.next-context.currentTime)*1000);
        $('#audio-stats').textContent=`About ${delay} ms delay · ${queue} ms buffered · ${player.underruns} gaps`;
        padStatus('audio',`Desktop audio · ${delay} ms`);
    }
    $('#audio').onchange=e=>{prefs.audio=e.target.checked;unlock();saveSettings();sync()};
    for(const type of ['pointerup','touchend','click','keydown'])document.addEventListener(type,unlock,true);
    setInterval(stats,500);
    refreshAudio=sync;
    sync();
})();
