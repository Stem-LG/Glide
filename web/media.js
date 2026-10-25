'use strict';

// Reserve a short settling window only while media mode is enabled. Once
// native input starts, never steal that contact sequence from the compositor.
class FourFingerMedia {
    constructor(native, action, schedule=(fn,ms)=>setTimeout(fn,ms), unschedule=id=>clearTimeout(id)) {
        this.native=native; this.action=action;
        this.schedule=schedule; this.unschedule=unschedule;
        this.enabled=false; this.reset();
    }
    reset() {
        this.unschedule(this.timer); this.timer=null;
        this.mode='idle'; this.points=[]; this.origins=new Map();
        this.trackSent=false; this.axis=null; this.moved=false; this.cancelled=false; this.released=false;
    }
    cancel() { this.reset(); this.native([]); }
    commit() {
        this.unschedule(this.timer); this.timer=null;
        this.mode='native'; this.native(this.points,true);
    }
    center(points, surface) {
        return {x:points.reduce((s,p)=>s+p.x*surface.width,0)/points.length,
            y:points.reduce((s,p)=>s+p.y*surface.height,0)/points.length};
    }
    update(points, surface, now, cancelled=false) {
        if(cancelled) {
            this.unschedule(this.timer); this.timer=null;
            this.mode='blocked'; this.cancelled=true; this.native([]);
        }
        if(this.mode==='idle' && points.length) {
            this.started=now;
            this.mode=this.enabled?'pending':'native';
            if(this.mode==='pending') this.timer=this.schedule(()=>{
                if(this.mode==='pending')this.commit();
            },90);
        }
        if(this.mode==='pending') {
            // On release, forward the last full set before the release itself,
            // preserving native one/two/three-finger taps without media leakage.
            const removed=this.points.some(p=>!points.some(q=>q.id===p.id));
            if(removed) this.commit();
            else {
                this.points=points.map(p=>({...p}));
                for(const p of points)if(!this.origins.has(p.id))this.origins.set(p.id,{...p});
                if(points.length===4) {
                    this.unschedule(this.timer); this.timer=null; this.mode='media';
                    this.anchor=this.center(points,surface);this.stepY=this.anchor.y;
                    this.lastVolume=-Infinity;
                } else if(points.length>4) {
                    this.unschedule(this.timer);this.timer=null;this.mode='blocked';
                } else if(points.some(p=>{
                    const o=this.origins.get(p.id);
                    return Math.hypot((p.x-o.x)*surface.width,(p.y-o.y)*surface.height)>6;
                })) {
                    // Start from the initial positions so the first movement
                    // is not swallowed when releasing the pending sequence.
                    this.native([...this.origins.values()],true);this.commit();
                }
            }
        }
        if(this.mode==='native') this.native(points);
        if(this.mode==='media') {
            if(points.length!==4) {
                if(points.length>4)this.cancelled=true;
                this.released=true;
            } else if(this.released) this.cancelled=true;
            else {
                const c=this.center(points,surface),dx=c.x-this.anchor.x,dy=c.y-this.anchor.y;
                if(points.some(p=>{
                    const o=this.origins.get(p.id);
                    return !o || Math.hypot((p.x-o.x)*surface.width,(p.y-o.y)*surface.height)>10;
                }))this.moved=true;
                if(!this.axis && Math.max(Math.abs(dx),Math.abs(dy))>=28) {
                    if(Math.abs(dx)>Math.abs(dy)*1.3)this.axis='horizontal';
                    else if(Math.abs(dy)>Math.abs(dx)*1.3)this.axis='vertical';
                }
                if(this.axis==='horizontal' && !this.trackSent && Math.abs(dx)>=48) {
                    this.trackSent=true;this.action(dx<0?'previous':'next');
                } else if(this.axis==='vertical' && Math.abs(c.y-this.stepY)>=28 && now-this.lastVolume>=90) {
                    this.action(c.y<this.stepY?'volume_up':'volume_down');
                    this.stepY=c.y;this.lastVolume=now;
                }
            }
            if(!points.length && !this.cancelled && !this.moved && !this.axis && now-this.started<=300)
                this.action('play_pause');
        }
        this.points=points.map(p=>({...p}));
        if(!points.length) { this.reset(); this.trackSent=false; }
    }
}
if(typeof module!=='undefined')module.exports={FourFingerMedia};
