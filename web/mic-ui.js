'use strict';
(()=>{
    const idle='Use this device as a microphone for your desktop.';
    const supported=window.isSecureContext&&navigator.mediaDevices?.getUserMedia&&window.AudioWorkletNode;
    let socket=null,context=null,stream=null,node=null,opening=false,live=false,retry=0,problem='';
    // Wait for the desktop to confirm the setting before sending any sound.
    const wanted=()=>ready&&prefs.microphone&&micSaved&&!document.hidden;
    const status=text=>{$('#microphone-status').textContent=text};
    function hangUp(){clearTimeout(retry);live=false;stats();if(socket){socket.onclose=null;socket.close();socket=null}}
    // Releasing the tracks turns off the browser's recording indicator.
    function release(){
        hangUp();
        stream?.getTracks().forEach(track=>track.stop());stream=null;
        node?.disconnect();node=null;
        context?.close();context=null;
    }
    // The permission prompt and audio start both need a user gesture.
    async function capture(){
        if(!supported||opening||!ready||!prefs.microphone)return;
        if(stream){if(context.state!=='running')context.resume().then(sync,()=>{});return}
        opening=true;
        try{
            const media=await navigator.mediaDevices.getUserMedia({audio:{echoCancellation:true,noiseSuppression:true,autoGainControl:true}});
            let audio;
            try{audio=new AudioContext({sampleRate:48000})}catch{audio=new AudioContext()}
            await audio.audioWorklet.addModule('/mic-worklet.js');
            const worklet=new AudioWorkletNode(audio,'glide-mic');
            audio.createMediaStreamSource(media).connect(worklet);
            // The worklet outputs silence; the connection only keeps it running.
            worklet.connect(audio.destination);
            worklet.port.onmessage=e=>{if(live&&socket?.readyState===1&&socket.bufferedAmount<16384)socket.send(e.data)};
            audio.onstatechange=sync;
            stream=media;context=audio;node=worklet;problem='';
        }catch(e){
            problem=e.name==='NotAllowedError'?'Microphone permission was denied. Allow it for this site in your browser.':'Could not open this device’s microphone.';
        }finally{opening=false}
        sync();
    }
    function connect(){
        const mine=socket=new WebSocket(wsUrl('/mic'));
        mine.onopen=()=>mine.send(JSON.stringify({token,rate:context.sampleRate}));
        mine.onmessage=e=>{
            const m=JSON.parse(e.data);
            if(m.mic){problem='';live=true;stats();status('Microphone is live. Choose “Glide phone microphone” on your desktop.')}
            if(m.error){problem=m.error;status(m.error)}
        };
        mine.onclose=()=>{
            if(socket!==mine)return;
            socket=null;live=false;stats();
            if(wanted())retry=setTimeout(sync,problem?5000:1500);
        };
    }
    function stats(){
        const text=live&&context?`Live · ${Math.round(context.sampleRate/1000)} kHz mono`:'';
        $('#microphone-stats').textContent=text;padStatus('mic',live?'Microphone live':'');
    }
    function sync(){stats();
        $('#microphone').checked=prefs.microphone;$('#microphone').disabled=!ready||!supported;
        if(!supported){status(window.isSecureContext?'This browser cannot share its microphone.':'Turn on “Secure connection (HTTPS)” in Glide on your desktop to use the microphone.');return}
        if(!ready||!prefs.microphone||document.hidden){release();problem='';status(idle);return}
        if(problem&&!stream){status(problem);return}
        if(!stream||context.state!=='running'){hangUp();status('Tap anywhere to start the microphone.');return}
        if(!wanted()){hangUp();return}
        if(!socket){clearTimeout(retry);if(!problem)status('Starting the microphone…');connect()}
    }
    $('#microphone').onchange=e=>{prefs.microphone=e.target.checked;problem='';saveSettings();if(prefs.microphone)capture();else sync()};
    for(const type of ['pointerup','touchend','click','keydown'])document.addEventListener(type,()=>{if(!problem)capture()},true);
    refreshMic=sync;
    sync();
})();
