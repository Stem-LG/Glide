const {test}=require('node:test'),assert=require('node:assert/strict');
const {GlideTools,GLIDE_TOOLS,NUMPAD_KEYS}=require('../web/tools.js');
test('number pad can be opened without pinning, and pinning does not change active tool',()=>{
 const t=new GlideTools();assert.equal(t.active,'touchpad');assert.deepEqual(t.pins,['numpad']);
 t.toggle('numpad');assert.equal(t.active,'numpad');t.pin('numpad',false);
 assert.equal(t.active,'numpad');assert.deepEqual(t.pins,[]);t.select('touchpad');t.select('numpad');assert.equal(t.active,'numpad');
 t.toggle('numpad');assert.equal(t.active,'touchpad');
});
test('unknown tools, duplicates and non-pinnable entries are not accepted',()=>{
 const t=new GlideTools(['numpad','numpad','touchpad','shell']);assert.deepEqual(t.pins,['numpad']);
 assert.equal(t.select('shell'),false);assert.equal(t.pin('touchpad',true),false);
 assert.equal(t.active,'touchpad');t.setPins([]);assert.deepEqual(t.pins,[]);
 assert.equal(GLIDE_TOOLS.filter(t=>t.pinnable).length,1);
});
test('numeric layout has one copy of every digit and required editing/arithmetic keys',()=>{
 const keys=NUMPAD_KEYS.map(k=>k.key);assert.equal(new Set(keys).size,keys.length);
 for(const key of ['0','1','2','3','4','5','6','7','8','9','decimal','add','subtract','divide','multiply','enter','backspace','delete'])assert.ok(keys.includes(key),key);
});
