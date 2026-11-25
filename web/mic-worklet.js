'use strict';
// Runs on the audio thread: mixes the microphone to mono and posts 20 ms
// frames of signed 16-bit PCM to the page.
class GlideMic extends AudioWorkletProcessor {
    constructor() {
        super();
        this.size=Math.round(sampleRate/50); this.frame=new Int16Array(this.size); this.used=0;
    }
    process(inputs) {
        const input=inputs[0];
        if(!input.length)return true;
        for(let i=0;i<input[0].length;i++) {
            let sample=0;
            for(const channel of input)sample+=channel[i];
            this.frame[this.used++]=Math.max(-1,Math.min(1,sample/input.length))*32767;
            if(this.used===this.size) {
                this.port.postMessage(this.frame.buffer,[this.frame.buffer]);
                this.frame=new Int16Array(this.size); this.used=0;
            }
        }
        return true;
    }
}
registerProcessor('glide-mic',GlideMic);
