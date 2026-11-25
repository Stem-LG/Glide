'use strict';

// Plain HTTP pages cannot use AudioWorklet, so each PCM frame is scheduled
// back to back on the context clock. The queue is kept close to a small
// target: it grows after an underrun, relaxes while playback is clean, and a
// backlog is played slightly fast (or dropped if large) instead of lingering.
class PcmPlayer {
    constructor(context, rate=48000, channels=2, target=.05, floor=.03, ceiling=.2) {
        this.context=context; this.rate=rate; this.channels=channels;
        this.target=target; this.floor=floor; this.ceiling=ceiling;
        this.next=0; this.underruns=0; this.dropped=0;
    }
    // Seconds between a frame arriving here and it being heard.
    get delay() {
        const ctx=this.context;
        return Math.max(0,this.next-ctx.currentTime)+(ctx.outputLatency||0)+(ctx.baseLatency||0);
    }
    // `data` is an ArrayBuffer of interleaved signed 16-bit little-endian samples.
    push(data) {
        const ctx=this.context, frames=Math.floor(data.byteLength/2/this.channels);
        if(!frames||ctx.state!=='running')return 'idle';
        const now=ctx.currentTime; let lead=this.next-now;
        if(lead<.005) {
            if(this.next) { this.underruns++; this.target=Math.min(this.ceiling,this.target+.02); }
            this.next=now+this.target; lead=this.target;
        } else if(lead>this.target+.25) { this.dropped++; return 'dropped'; }
        const view=new DataView(data), buffer=ctx.createBuffer(this.channels,frames,this.rate);
        for(let c=0;c<this.channels;c++) {
            const out=buffer.getChannelData(c);
            for(let i=0;i<frames;i++)out[i]=view.getInt16((i*this.channels+c)*2,true)/32768;
        }
        const node=ctx.createBufferSource(), speed=lead>this.target+.03?1.04:1;
        node.buffer=buffer; node.playbackRate.value=speed;
        node.connect(ctx.destination); node.start(this.next);
        this.next+=frames/this.rate/speed;
        this.target=Math.max(this.floor,this.target-frames/this.rate*2e-4);
        return speed>1?'catching-up':'queued';
    }
}
if(typeof module!=='undefined')module.exports={PcmPlayer};
