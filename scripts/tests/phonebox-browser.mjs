import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync, mkdirSync } from 'node:fs';
import { createServer } from 'vite';

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = new URL('../../', import.meta.url);
const template = readFileSync(new URL('integrations/phonebox/tokenball-widget.html',root),'utf8');
// Optional read-only compatibility check against an unmodified PhoneBox checkout.
const stockWidget = process.env.PHONEBOX_WIDGET_SOURCE
  ? readFileSync(process.env.PHONEBOX_WIDGET_SOURCE,'utf8').split('// Battery levels')[0].split('\n').filter(line=>!line.startsWith('import ')).join('\n').replaceAll('export const ','const ')
  : null;
const artifacts = new URL('.diagnostics/phonebox/',root);
mkdirSync(artifacts,{recursive:true});
const server=await createServer({server:{host:'127.0.0.1',port:1423,strictPort:true}});
await server.listen();
const browser=await chromium.launch({headless:true, executablePath:process.env.PLAYWRIGHT_EXECUTABLE});
try {
  const page=await browser.newPage({viewport:{width:1120,height:900}});
  await page.addInitScript(() => {
    let settings={enabled:false,baseUrl:'http://127.0.0.1:8765'}, count=0;
    window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'main'}},transformCallback:()=>++count,unregisterCallback:()=>{},
      invoke:async(command,args)=>{
        if(command==='connection_list'||command==='plugin_list')return [];
        if(command==='quota_get_latest')return {accounts:[],totalAccounts:0,availableAccounts:0,status:'unknown',stale:false};
        if(command==='settings_get_display')return {};
        if(command==='orb_get_visible')return true;
        if(command==='plugin:event|listen')return ++count;
        if(command==='phonebox_get_settings'){if(window.phoneboxFailLoad)throw Error('PhoneBox 设置无效，请重新保存');return settings;}
        if(command==='phonebox_save_settings'){settings={...args.settings};return settings;}
        if(command==='phonebox_get_status')return {sending:false,lastAttemptAt:null,lastSuccessAt:null,error:null};
        if(command==='phonebox_push_now'){window.phoneboxPushCount=(window.phoneboxPushCount||0)+1;return;}
        return null;
      }};
    window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:()=>{}};
  });
  await page.goto('http://127.0.0.1:1423/?view=main');
  await page.getByRole('button',{name:/设置/}).first().click();
  await page.getByRole('button',{name:/常规/}).first().click();
  const push=page.getByRole('button',{name:'立即推送',exact:true});
  await push.waitFor(); assert.equal(await push.isDisabled(),true);
  await page.getByLabel('启用额度推送').check();
  assert.equal(await push.isDisabled(),true);
  await page.getByRole('button',{name:'保存 PhoneBox 设置'}).click();
  await page.waitForFunction(()=>!Array.from(document.querySelectorAll('button')).find(b=>b.textContent==='立即推送').disabled);
  await push.click(); assert.equal(await page.evaluate(()=>window.phoneboxPushCount),1);
  await page.getByLabel('PhoneBox Agent 地址').fill('http://127.0.0.1:18765');
  assert.equal(await push.isDisabled(),true);
  await page.screenshot({path:new URL('settings.png',artifacts).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
  await page.addInitScript(()=>{window.phoneboxFailLoad=true;});
  await page.reload();
  await page.getByRole('button',{name:/设置/}).first().click();
  await page.getByRole('button',{name:/常规/}).first().click();
  const repair=page.getByRole('button',{name:'使用默认值修复'});
  await repair.click();
  assert.equal(await page.getByLabel('启用额度推送').isChecked(),false);
  assert.equal(await page.getByRole('button',{name:'保存 PhoneBox 设置'}).isEnabled(),true);
  assert.equal(await push.isDisabled(),true);

  const widget=await browser.newPage({viewport:{width:420,height:780}});
  const errors=[], requests=[];widget.on('pageerror',e=>errors.push(e.message));
  widget.on('request',r=>requests.push(r.url()));
  await widget.setContent('<div id="host" style="height:96vh"></div>');
  if(stockWidget) {
    await widget.evaluate(({source,html})=>{
      const W={},D={};
      const h=(tag,cls)=>{const n=document.createElement(tag);n.className=cls;return n;};
      const create=new Function('W','D','h','cfg','api',source+'; return W.html;');
      window.stockData=D;
      window.stockController=create(W,D,h,{shortcuts:[]},()=>{throw Error('unexpected shortcut')}).create(document.getElementById('host'),{o:{html}});
      const frame=document.querySelector('iframe');frame.style.cssText='width:100%;height:100%;border:0';
    },{source:stockWidget,html:template});
  } else {
    await widget.evaluate(html=>{
      const frame=document.createElement('iframe');frame.setAttribute('sandbox','allow-scripts');frame.style.cssText='width:100%;height:100%;border:0';
      frame.srcdoc='<script>window.phonebox={on:function(k,fn){addEventListener("message",function(e){if(e.data.phonebox==="data"&&e.data.kind===k)fn(e.data.value)})}};<'+ '/script>'+html;
      document.getElementById('host').append(frame);
    },template);
  }
  const frame=widget.frameLocator('iframe');await frame.locator('#accounts').waitFor();
  const now=Date.now();
  const account={id:'opaque',alias:'账号 7ab93c12',provider:'volcengine',instanceAlias:'实例 1a4e3a66',planName:'Coding Plan',status:'available',connectionStatus:'healthy',collectedAt:new Date(now-60000).toISOString(),staleAt:new Date(now+3600000).toISOString(),stale:false,subscriptionUntil:new Date(now+10*86400000).toISOString(),periods:{fiveHour:{remainingPercent:82,resetAt:new Date(now+3500000).toISOString()},weekly:{remainingPercent:48,resetAt:new Date(now+172800000).toISOString()},monthly:null}};
  const data={schema:'tokenball.phonebox.v1',pushedAt:new Date(now).toISOString(),summary:{accountCount:3,availableAccountCount:2,averageRemainingPercent:65,equivalentAccounts:1.3},accounts:[{...account,displayName:'研发主账号',priorityResetAt:new Date(now+3600000).toISOString()},{...account,id:'second',displayName:'运营账号',priorityResetAt:new Date(now+7200000).toISOString(),alias:'账号 4cc2d908',provider:'qianwen',planName:'<img src=x onerror=alert(1)>',stale:true},{...account,id:'third',displayName:'备用账号',priorityResetAt:new Date(now+10800000).toISOString(),alias:'账号 1897ead1',provider:'cliProxyApi'}],omittedAccountCount:0};
  await widget.locator('iframe').evaluate((f,data)=>f.contentWindow.postMessage({phonebox:'data',kind:'push',value:{tokenball:{data}}},'*'),data);
  await frame.getByText('82%',{exact:true}).first().waitFor();
  assert.equal(await frame.locator('img').count(),0);
  assert.equal(await frame.locator('.account').count(),3);
  assert.equal(await frame.locator('#connection').textContent(),'推送在线');
  assert.equal(await frame.locator('.hero-ring').count(),1);
  assert.equal(await frame.locator('.period .ring').count(),9);
  assert.equal(await frame.locator('.period .ring.unknown').count(),3);
  assert.ok(await frame.locator('.hero-ring .percent').evaluate(n=>parseFloat(getComputedStyle(n).fontSize)>=43));
  assert.ok((await frame.locator('.account').nth(1).textContent()).includes('<img src=x onerror=alert(1)>'));
  data.accounts[1].planName='Coding Plan';
  data.accounts[1].periods={...account.periods,fiveHour:{remainingPercent:18,resetAt:new Date(now+7200000).toISOString()}};
  await widget.locator('iframe').evaluate((f,data)=>f.contentWindow.postMessage({phonebox:'data',kind:'push',value:{tokenball:{data}}},'*'),data);
  await frame.getByText('<img src=x onerror=alert(1)>',{exact:false}).waitFor({state:'hidden'});
  await frame.getByText('18%',{exact:true}).waitFor();
  const first=frame.locator('.account').first();
  assert.equal(await first.locator('.ring.healthy .percent').evaluate(n=>getComputedStyle(n).color),'rgb(25, 195, 125)');
  assert.equal(await first.locator('.reset.critical').first().evaluate(n=>getComputedStyle(n).color),'rgb(255, 90, 84)');
  assert.equal(await first.locator('.reset.warning').first().evaluate(n=>getComputedStyle(n).color),'rgb(233, 185, 73)');
  assert.equal(await frame.locator('.account').nth(1).locator('.ring.critical .percent').evaluate(n=>getComputedStyle(n).color),'rgb(255, 90, 84)');
  assert.equal(await first.locator('.foot time.healthy').count(),2);
  await widget.screenshot({path:new URL('portrait.png',artifacts).pathname.replace(/^\/([A-Za-z]:)/,'$1')});
  await widget.setViewportSize({width:900,height:430});
  await frame.locator('#counter').filter({hasText:'1 / 3'}).waitFor();
  await frame.getByRole('button',{name:'下一个账号',exact:true}).click();
  await widget.waitForTimeout(500);
  assert.equal(await frame.locator('#counter').textContent(),'2 / 3');
  assert.ok(await frame.locator('.account').nth(1).evaluate(n=>Math.abs(n.getBoundingClientRect().left-document.getElementById('accounts').getBoundingClientRect().left)<2));
  await widget.waitForTimeout(1200);
  assert.equal(await frame.locator('#counter').textContent(),'2 / 3');
  await frame.getByRole('button',{name:'上一个账号',exact:true}).click();
  await widget.waitForTimeout(500);
  await widget.screenshot({path:new URL('landscape.png',artifacts).pathname.replace(/^\/([A-Za-z]:)/,'$1')});
  for(const size of [{width:320,height:568},{width:900,height:320},{width:1280,height:720},{width:680,height:430}]) {
    await widget.setViewportSize(size);
    assert.ok(await frame.locator('body').evaluate(n=>n.scrollWidth<=innerWidth),'no horizontal overflow');
    assert.ok(await frame.locator('#accounts').evaluate(n=>n.clientHeight>=100&&n.scrollWidth>n.clientWidth),'carousel remains swipeable '+size.width+'x'+size.height+' '+await frame.locator('#accounts').evaluate(n=>[n.clientHeight,n.clientWidth,n.scrollWidth].join('/')));
    assert.ok(await frame.locator('#metrics').evaluate(n=>n.getBoundingClientRect().bottom<=innerHeight),'overview remains visible '+size.width+'x'+size.height);
    if(size.width>=680)assert.ok(await frame.locator('.overview').evaluate(n=>Math.abs(n.getBoundingClientRect().bottom-(innerHeight-parseFloat(getComputedStyle(document.querySelector('main')).paddingBottom)))<2),'overview fills height');
    assert.ok(await first.evaluate(n=>{
      const box=s=>n.querySelector(s).getBoundingClientRect();
      return box('.account-cover').bottom<=box('.account-head').top&&box('.account-head').bottom<=box('.periods').top&&box('.periods').bottom<=box('.foot').top;
    }),'cover, identity, rings and footer never overlap');
    assert.ok(await first.locator('.period').evaluateAll(nodes=>nodes.every(n=>{
      const r=n.querySelector('.ring').getBoundingClientRect(),t=n.querySelector('.reset').getBoundingClientRect();return r.bottom<=t.top&&r.width<=n.clientWidth;
    })),'period text stays outside circles');
    await frame.getByRole('button',{name:'查看备用账号',exact:true}).click();
    await widget.waitForTimeout(500);
    assert.equal(await frame.locator('#counter').textContent(),'3 / 3');
    await frame.locator('#accounts').focus();
    await widget.keyboard.press('ArrowLeft');
    await widget.waitForTimeout(500);
    assert.equal(await frame.locator('#counter').textContent(),'2 / 3');
    await frame.getByRole('button',{name:'查看研发主账号',exact:true}).click();
    await widget.waitForTimeout(500);
    if(size.width===320)await widget.screenshot({path:new URL('narrow.png',artifacts).pathname.replace(/^\/([A-Za-z]:)/,'$1')});
  }
  const reordered={...data,accounts:[data.accounts[2],data.accounts[0],data.accounts[1]]};
  await widget.locator('iframe').evaluate((f,data)=>f.contentWindow.postMessage({phonebox:'data',kind:'push',value:{tokenball:{data}}},'*'),reordered);
  assert.deepEqual(await frame.locator('.account .name').allTextContents(),['研发主账号','运营账号','备用账号']);
  const changed={...data,accounts:data.accounts.map((a,index)=>({...a,subscriptionUntil:new Date(now+(3-index)*86400000).toISOString()}))};
  await widget.locator('iframe').evaluate((f,data)=>f.contentWindow.postMessage({phonebox:'data',kind:'push',value:{tokenball:{data}}},'*'),changed);
  await frame.getByRole('button',{name:'查看研发主账号',exact:true}).waitFor();
  await widget.waitForTimeout(100);
  assert.deepEqual(await frame.locator('.account .name').allTextContents(),['备用账号','运营账号','研发主账号']);
  assert.equal(await frame.locator('#counter').textContent(),'3 / 3','selected account survives resort');
  assert.deepEqual(errors,[]);
  assert.deepEqual(requests,[]);
  console.log('PhoneBox 设置交互、'+(stockWidget?'原版 HTML 组件':'模拟沙箱')+'、安全文本、零网络请求、横竖屏检查通过');
} finally {await browser.close();await server.close();}
