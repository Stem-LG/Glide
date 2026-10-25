const {test}=require('node:test');
const assert=require('node:assert/strict');
const {TapFeedback,tapPulse}=require('../web/taps.js');
const {FourFingerMedia}=require('../web/media.js');
const event=(id,time,x=100,y=100)=>({pointerId:id,pointerType:'touch',timeStamp:time,clientX:x,clientY:y});

test('one, two and three finger taps give exactly one feedback pulse',()=>{
    for(const n of [1,2,3]) {
        const t=new TapFeedback();
        for(let i=0;i<n;i++)t.down(event(i,i*15,100+i*30));
        for(let i=0;i<n;i++)assert.equal(t.up(event(i,70+i*15,100+i*30)),i===n-1?n:0);
        assert.equal(t.up(event(n-1,150)),0); // lost capture after pointerup
    }
});
test('movement, scroll, swipe, hold and four finger contacts do not pulse',()=>{
    for(const n of [1,2,3]) {
        const t=new TapFeedback();
        for(let i=0;i<n;i++)t.down(event(i,0));
        t.move(event(0,30,120));t.move(event(0,40,100));
        for(let i=0;i<n;i++)assert.equal(t.up(event(i,80)),0);
    }
    const t=new TapFeedback();t.down(event(1,0));assert.equal(t.up(event(1,181)),0);
    for(let i=0;i<4;i++)t.down(event(i,200));
    for(let i=0;i<4;i++)assert.equal(t.up(event(i,260)),0);
});
test('release motion and coalesced excursions invalidate a tap',()=>{
    const t=new TapFeedback();t.down(event(1,0));assert.equal(t.up(event(1,50,130)),0);
    t.down(event(1,100));t.move({...event(1,120),getCoalescedEvents:()=>[event(1,110,130)]});
    assert.equal(t.up(event(1,150)),0);
});
test('cancellation, reset and new contacts during release do not pulse',()=>{
    const t=new TapFeedback();t.down(event(1,0));t.down(event(2,10));
    assert.equal(t.up(event(1,50),true),0);assert.equal(t.up(event(2,60)),0);
    t.down(event(1,100));t.reset();assert.equal(t.up(event(1,130)),0);
    t.down(event(1,200));t.down(event(2,210));t.up(event(1,220));t.down(event(3,230));
    assert.equal(t.up(event(2,240)),0);assert.equal(t.up(event(3,250)),0);
    t.down({...event(1,300),pointerType:'mouse'});assert.equal(t.up(event(1,310)),0);
});
test('vibration is brief and gracefully handles unsupported or denied APIs',()=>{
    const pulses=[];assert.equal(tapPulse({vibrate:n=>{pulses.push(n);return true;}}),true);
    assert.deepEqual(pulses,[25]);assert.equal(tapPulse({}),false);
    assert.equal(tapPulse({vibrate:()=>false}),false);
    assert.equal(tapPulse({vibrate:()=>{throw Error('blocked');}}),false);
});

test('web input pulses only when enabled and connected; cancellation stays silent',()=>{
    const vm=require('node:vm'),fs=require('node:fs');
    const elements=new Map(),pulses=[],frames=[];
    const element=()=>({value:'',checked:false,hidden:false,disabled:false,textContent:'',dataset:{},
        classList:{toggle(){}},setAttribute(){},replaceChildren(){},append(){},
        setPointerCapture(){},getBoundingClientRect:()=>({left:0,top:0,width:390,height:844}),
        showModal(){},close(){}});
    const context=vm.createContext({Map,Set,JSON,Math,Number,Boolean,String,TapFeedback,tapPulse,FourFingerMedia,
        navigator:{vibrate:n=>{pulses.push(n);return true;}},
        document:{querySelector:s=>{if(!elements.has(s))elements.set(s,element());return elements.get(s);},
            documentElement:element(),addEventListener(){},createElement:element,hidden:false},
        window:{addEventListener(){}},localStorage:{getItem:()=>'',setItem(){},removeItem(){}},
        setInterval(){},setTimeout(){},clearTimeout(){},requestAnimationFrame(){return 1;},cancelAnimationFrame(){},
        performance:{now:()=>0}});
    vm.runInContext(fs.readFileSync(require.resolve('../web/app.js'),'utf8'),context);
    context.record=frame=>frames.push(JSON.parse(frame));
    vm.runInContext('ready=true;ws={readyState:1,bufferedAmount:0,send:record}',context);
    const pad=elements.get('#pad');
    const e=(id,t,type)=>({...event(id,t),type,preventDefault(){}});
    pad.onpointerdown(e(1,0,'pointerdown'));pad.onpointerup(e(1,50,'pointerup'));
    pad.onlostpointercapture(e(1,51,'lostpointercapture'));assert.deepEqual(pulses,[25]);
    assert.deepEqual(frames.map(f=>f.points.length),[1,0]); // no synthetic/duplicate button
    assert.ok(frames.every(f=>f.button===0));
    vm.runInContext('prefs.haptics=false',context);
    pad.onpointerdown(e(1,100,'pointerdown'));pad.onpointerup(e(1,150,'pointerup'));
    vm.runInContext('prefs.haptics=true',context);
    pad.onpointerdown(e(1,200,'pointerdown'));pad.onpointercancel(e(1,250,'pointercancel'));
    pad.onpointerdown(e(1,300,'pointerdown'));vm.runInContext('clear()',context);pad.onpointerup(e(1,350,'pointerup'));
    vm.runInContext('ready=false',context);pad.onpointerdown(e(1,400,'pointerdown'));pad.onpointerup(e(1,450,'pointerup'));
    assert.deepEqual(pulses,[25]);
    vm.runInContext('ready=true',context);
    elements.get('#haptic-duration').oninput({target:{value:'42'}});
    assert.equal(elements.get('#haptic-duration-value').textContent,'42 ms');
    elements.get('#haptic-duration').onchange();
    assert.equal(frames.at(-1).settings.haptic_duration_ms,42);
    assert.deepEqual(pulses,[25,42]);
    pad.onpointerdown(e(1,500,'pointerdown'));pad.onpointerup(e(1,550,'pointerup'));
    assert.deepEqual(pulses,[25,42,42]);
    elements.get('#test-haptics').onclick();
    assert.deepEqual(pulses,[25,42,42,42]);
    assert.match(elements.get('#haptics-result').textContent,/accepted/);
    context.navigator.vibrate=()=>false;elements.get('#test-haptics').onclick();
    assert.match(elements.get('#haptics-result').textContent,/rejected/);
    delete context.navigator.vibrate;elements.get('#test-haptics').onclick();
    assert.match(elements.get('#haptics-result').textContent,/does not support/);
    elements.get('#media-controls').onchange({target:{checked:true}});
    assert.equal(frames.at(-1).settings.media_controls,true);
    frames.length=0;
    for(let i=0;i<4;i++)pad.onpointerdown(e(i,600+i*10,'pointerdown'));
    for(let i=0;i<4;i++)pad.onpointerup(e(i,680+i*10,'pointerup'));
    assert.equal(frames.length,1);
    assert.equal(frames[0].media,'play_pause');
    assert.deepEqual(frames[0].points,[]);
    frames.length=0;
    for(let i=0;i<4;i++)pad.onpointerdown(e(i,800+i*10,'pointerdown'));
    pad.onpointercancel(e(0,850,'pointercancel'));
    for(let i=1;i<4;i++)pad.onpointerup(e(i,860+i*10,'pointerup'));
    assert.ok(frames.every(f=>f.media===null && !f.points.length));

});
