'use strict';

// Add future tools here; the rail and picker use this shared registry.
const GLIDE_TOOLS=Object.freeze([
    Object.freeze({id:'touchpad',name:'Touchpad',description:'Move, scroll and navigate',pinnable:false}),
    Object.freeze({id:'numpad',name:'Number pad',description:'Digits and arithmetic keys for your desktop',pinnable:true}),
]);
const NUMPAD_KEYS=Object.freeze([
    {key:'backspace',label:'Backspace',symbol:'⌫'}, {key:'delete',label:'Delete',symbol:'Del'},
    {key:'divide',label:'Divide',symbol:'÷',kind:'operator'}, {key:'multiply',label:'Multiply',symbol:'×',kind:'operator'},
    {key:'7',label:'7'}, {key:'8',label:'8'}, {key:'9',label:'9'}, {key:'subtract',label:'Subtract',symbol:'−',kind:'operator'},
    {key:'4',label:'4'}, {key:'5',label:'5'}, {key:'6',label:'6'}, {key:'add',label:'Add',symbol:'+',kind:'operator'},
    {key:'1',label:'1'}, {key:'2',label:'2'}, {key:'3',label:'3'}, {key:'enter',label:'Enter',kind:'enter'},
    {key:'0',label:'0',kind:'zero'}, {key:'decimal',label:'Decimal separator',symbol:'.'},
]);
class GlideTools {
    constructor(pins=['numpad']){this.active='touchpad';this.setPins(pins);}
    setPins(pins){this.pins=[...new Set(Array.isArray(pins)?pins:[])].filter(id=>GLIDE_TOOLS.some(t=>t.id===id&&t.pinnable));}
    select(id){if(!GLIDE_TOOLS.some(t=>t.id===id))return false;this.active=id;return true;}
    toggle(id){return this.select(this.active===id?'touchpad':id);}
    pin(id,enabled){
        if(!GLIDE_TOOLS.some(t=>t.id===id&&t.pinnable))return false;
        this.setPins(enabled?[...this.pins,id]:this.pins.filter(p=>p!==id));return true;
    }
}
if(typeof module!=='undefined')module.exports={GlideTools,GLIDE_TOOLS,NUMPAD_KEYS};
