'use strict';

// Feedback only: libinput still owns clicks, tap-to-drag, and gestures.
// Use a conservative local tap estimate; the browser cannot observe the
// compositor's eventual click or its tap-to-click setting.
class TapFeedback {
    constructor() { this.reset(); }
    reset() {
        this.contacts = new Map();
        this.started = 0;
        this.peak = 0;
        this.valid = true;
        this.released = false;
    }
    down(e) {
        if (e.pointerType !== 'touch') return;
        if (!this.contacts.size) {
            this.reset();
            this.started = e.timeStamp;
        }
        if (this.released) this.valid = false;
        this.contacts.set(e.pointerId, {x: e.clientX, y: e.clientY});
        this.peak = Math.max(this.peak, this.contacts.size);
        if (this.peak > 3) this.valid = false;
    }
    move(e) {
        const start = this.contacts.get(e.pointerId);
        if (!start) return;
        const samples = e.getCoalescedEvents?.() || [];
        for (const p of [...samples, e]) {
            if (Math.hypot(p.clientX - start.x, p.clientY - start.y) > 8) this.valid = false;
        }
    }
    up(e, cancelled = false) {
        if (!this.contacts.has(e.pointerId)) return 0;
        this.move(e);
        this.contacts.delete(e.pointerId);
        this.released = true;
        if (cancelled || e.timeStamp - this.started > 180) this.valid = false;
        if (this.contacts.size) return 0;
        const fingers = this.valid ? this.peak : 0;
        this.reset();
        return fingers;
    }
}
function tapPulse(device, duration = 25) {
    // Unsupported/blocked vibration must never interrupt touch input.
    try { return typeof device.vibrate === 'function' && device.vibrate(duration); }
    catch { return false; }
}
if (typeof module !== 'undefined') module.exports = {TapFeedback, tapPulse};
