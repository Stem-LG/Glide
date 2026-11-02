'use strict';
(()=>{
    const tools=new GlideTools(prefs.pinnedTools);
    const icons={
        touchpad:'<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="4" width="18" height="16" rx="3"/><path d="M3 15h18m-9 0v5"/></svg>',
        numpad:'<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="4" y="2" width="16" height="20" rx="3"/><path d="M8 7h8m-8 5h1m3 0h1m3 0h.1M8 16h1m3 0h1m3 0h.1"/></svg>',
    };
    function select(id,toggle=false){
        if(!(toggle?tools.toggle(id):tools.select(id)))return;
        clear();$('#tool-workspace').dataset.tool=tools.active;$('#numpad-panel').hidden=tools.active!=='numpad';
        renderRail();
        if(tools.active==='numpad')sendKey('num_lock');
    }
    function shortcut(tool){
        const b=document.createElement('button');b.type='button';b.className='tool-shortcut';
        b.title=tool.name;b.setAttribute('aria-label',tool.name);b.setAttribute('aria-pressed',String(tools.active===tool.id));
        b.innerHTML=icons[tool.id];b.onclick=()=>select(tool.id,true);return b;
    }
    function renderRail(){
        $('#rail-home').replaceChildren(shortcut(GLIDE_TOOLS[0]));
        $('#rail-pins').replaceChildren(...tools.pins.map(id=>shortcut(GLIDE_TOOLS.find(t=>t.id===id))));
    }
    function renderPicker(){
        $('#tool-list').replaceChildren();
        for(const tool of GLIDE_TOOLS.filter(t=>t.pinnable)){
            const row=document.createElement('div');row.className='tool-library-row';
            const open=document.createElement('button');open.type='button';open.className='tool-library-open';
            open.innerHTML=icons[tool.id];const text=document.createElement('span');
            const title=document.createElement('strong');title.textContent=tool.name;
            const description=document.createElement('small');description.textContent=tool.description;
            text.append(title,description);open.append(text);open.onclick=()=>{$('#tools-dialog').close();select(tool.id);};
            const pin=document.createElement('button');pin.type='button';pin.className='tool-pin';
            const pinned=tools.pins.includes(tool.id);pin.textContent=pinned?'Unpin':'Pin';pin.disabled=!ready;
            pin.setAttribute('aria-label',(pinned?'Unpin ':'Pin ')+tool.name);pin.setAttribute('aria-pressed',String(pinned));
            pin.onclick=()=>{
                if(!ready)return;tools.pin(tool.id,!pinned);prefs.pinnedTools=[...tools.pins];renderRail();renderPicker();saveSettings();
            };
            row.append(open,pin);$('#tool-list').append(row);
        }
    }
    function sendKey(key){
        if(!ready)return;
        if(raf)send();
        send(null,key);
        if(prefs.haptics)tapPulse(navigator,prefs.hapticDuration);
    }
    for(const key of NUMPAD_KEYS){
        const b=document.createElement('button');b.type='button';b.className='numpad-key '+(key.kind||'');
        b.setAttribute('aria-label',key.label);b.dataset.key=key.key;
        if(key.key==='backspace')b.innerHTML='<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 5h12v14H9L2 12zm3 4 6 6m0-6-6 6"/></svg>';
        else b.textContent=key.symbol||key.label;
        b.onclick=()=>sendKey(key.key);$('#numpad-keys').append(b);
    }
    function picker(){clear();renderPicker();$('#tools-dialog').showModal();}
    $('#all-tools').onclick=picker;$('#tools-button').onclick=()=>{$('#controls').close();picker();};
    $('#close-tools').onclick=()=>$('#tools-dialog').close();
    refreshTools=()=>{
        tools.setPins(prefs.pinnedTools);renderRail();
        for(const b of $('#numpad-keys').querySelectorAll('button'))b.disabled=!ready;
        if(!ready)$('#numpad-status').textContent='Connect to use the number pad.';
        else if($('#numpad-status').textContent==='Connect to use the number pad.')$('#numpad-status').textContent='Keys go to the focused app on your desktop.';
        if($('#tools-dialog').open)renderPicker();
    };
    refreshTools();
})();
