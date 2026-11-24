import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import test from 'node:test';
import ts from 'typescript';

const html = readFileSync(new URL('../../integrations/phonebox/tokenball-widget.html', import.meta.url), 'utf8');
const script = html.match(/<script>([\s\S]*?)<\/script>/)[1];
const initialTime = Date.parse('2026-11-23T10:00:00Z');

class Element {
  listeners = {}; attributes = {}; clientWidth = 500; scrollLeft = 0;
  addEventListener(name, callback) { this.listeners[name]=callback; }
  setAttribute(name, value) { this.attributes[name]=value; }
  scrollTo({left}) { this.scrollLeft=left; }
  children = []; style = { setProperty(name,value) { this[name]=value; } }; className = ''; ownText = '';
  appendChild(node) { this.children.push(node); return node; }
  removeChild(node) { this.children.splice(this.children.indexOf(node), 1); }
  get firstChild() { return this.children[0]; }
  set textContent(value) { this.ownText = String(value); this.children = []; }
  get textContent() { return this.ownText + this.children.map(n => n.textContent).join(' '); }
}
function harness() {
  let now = initialTime, subscriber, tick;
  const nodes = Object.fromEntries(['connection','metrics','notice','accounts','pagination','counter','previous','next'].map(id => [id,new Element()]));
  class Clock extends Date { static now() { return now; } }
  runInNewContext(script, { document: { createElement: () => new Element(), getElementById: id => nodes[id] },
    Date: Clock, window: { addEventListener:()=>{},phonebox: { on: (kind, callback) => { assert.equal(kind,'push'); subscriber = callback; } } },
    setInterval: callback => { tick=callback; } });
  return { nodes, send: items => subscriber(items), advance: ms => {now+=ms;tick();}, text: () => Object.values(nodes).map(n=>n.textContent).join(' ') };
}
function payload() {
  return { schema:'tokenball.phonebox.v1',pushedAt:new Date(initialTime).toISOString(),
    summary:{accountCount:1,availableAccountCount:1,averageRemainingPercent:80,equivalentAccounts:.8,stale:false},
    includedAccountCount:1,omittedAccountCount:0,accounts:[{id:'opaque',alias:'账号 12345678',provider:'volcengine',instanceAlias:'实例 98765432',planName:'Coding Plan',
      status:'available',connectionStatus:'healthy',collectedAt:new Date(initialTime-1000).toISOString(),staleAt:new Date(initialTime+2000).toISOString(),stale:false,
      subscriptionUntil:null,periods:{fiveHour:{remainingPercent:80,resetAt:new Date(initialTime+1000).toISOString()},weekly:{remainingPercent:null,resetAt:null},monthly:null}}] };
}
test('page JSON matches source, grid and documented download', () => {
  const page = JSON.parse(readFileSync(new URL('../../integrations/phonebox/tokenball-page.json',import.meta.url),'utf8'));
  const download = JSON.parse(readFileSync(new URL('../../docs/public/integrations/phonebox/tokenball-page.json',import.meta.url),'utf8'));
  assert.deepEqual(page, download); assert.equal(page.phonebox,'pages');assert.equal(page.version,1);
  assert.equal(page.pages[0].widgets.length,1);
  const [custom] = page.pages[0].widgets;
  assert.equal(custom.type,'html');
  assert.equal(custom.options.html,html);assert.deepEqual([custom.x,custom.y,custom.w,custom.h],[0,0,12,6]);
  assert.ok(custom.options.html.includes('items.tokenball'));
});
test('missing data, partial periods and inert text', () => {
  const h=harness(); assert.match(h.text(),/未收到数据/);
  const data=payload();data.accounts[0].planName='<img src=x onerror=alert(1)>';
  h.send({tokenball:{data,at:initialTime}});
  assert.match(h.text(),/80%/);assert.match(h.text(),/--/);assert.match(h.text(),/<img src=x onerror=alert\(1\)>/);
  assert.doesNotMatch(html,/innerHTML|fetch\(|XMLHttpRequest|phonebox\.(run|goto)|<script\s+src|https?:\/\//);
});
test('local clock ages each account and never invents a reset', () => {
  const h=harness();h.send({tokenball:{data:payload(),at:initialTime}});
  assert.doesNotMatch(h.text(),/缓存 \/ 异常/);h.advance(3000);
  assert.match(h.text(),/缓存 \/ 异常/);assert.doesNotMatch(h.text(),/分钟后恢复|等待额度刷新/);assert.match(h.text(),/80%/);
  h.advance(180000);assert.match(h.text(),/推送中断/);
});
test('new transport does not clear stale provider data', () => {
  const h=harness(),data=payload();data.accounts[0].stale=true;
  h.send({tokenball:{data,at:initialTime}});assert.match(h.text(),/推送在线/);assert.match(h.text(),/缓存 \/ 异常/);
  data.omittedAccountCount=5;h.send({tokenball:{data}});assert.match(h.text(),/省略 5 个账号/);
});
test('TTL disappearance and unsupported schema', () => {
  const h=harness();h.send({tokenball:{data:payload(),expiresAt:initialTime+1000}});h.advance(1001);assert.match(h.text(),/未收到数据/);
  h.send({tokenball:{data:{schema:'v99'}}});assert.match(h.text(),/版本不兼容/);
  h.send({});assert.match(h.text(),/等待推送/);
});

test('large overview and instance-style quota bars preserve known, low and unknown quota', () => {
  const h=harness(),data=payload();
  data.accounts[0].periods.weekly.remainingPercent=8;
  h.send({tokenball:{data}});
  const hero=h.nodes.metrics.children[0];
  assert.match(hero.className,/hero-ring/);
  assert.equal(hero.style['--sweep'],'288deg');
  assert.match(hero.textContent,/80%.*总体余量/);
  const grid=h.nodes.accounts.children[0].children[1];
  assert.equal(grid.className,'quota-bars');
  assert.equal(grid.children[0].children[1].children[0].style.width,'80%');
  assert.match(grid.children[1].children[1].className,/critical/);
  assert.match(grid.children[2].children[1].className,/unknown/);
  assert.equal(grid.children[2].children[1].children[0].style.width,'0%');
  assert.equal(grid.children[2].children[3].textContent,'--');
  data.summary.averageRemainingPercent=null;
  h.send({tokenball:{data}});
  assert.match(h.nodes.metrics.children[0].className,/unknown/);
  assert.match(h.nodes.metrics.children[0].textContent,/--/);
});

test('overview ring and quota bar colors match tray thresholds including boundary rounding', () => {
  const h=harness(),data=payload();
  for(const [value,expected] of [[0,'critical'],[29,'critical'],[30,'warning'],[59,'warning'],[60,'healthy'],[100,'healthy'],[null,'unknown'],[101,'unknown']]) {
    data.summary.averageRemainingPercent=value;
    data.accounts[0].periods.fiveHour.remainingPercent=value;
    h.send({tokenball:{data}});
    assert.ok(h.nodes.metrics.children[0].className.endsWith(expected));
    assert.ok(h.nodes.accounts.children[0].children[1].children[0].children[1].className.endsWith(expected));
  }
  data.summary.averageRemainingPercent=29.8;
  data.accounts[0].periods.fiveHour.remainingPercent=29.8;
  h.send({tokenball:{data}});
  assert.match(h.nodes.metrics.children[0].className,/critical/);
  assert.match(h.nodes.accounts.children[0].children[1].children[0].children[1].className,/warning/);
});

test('instance-style rows show usage, remaining percentage and independent reset times', () => {
  const h=harness(),data=payload(),account=data.accounts[0];
  account.criticalRemainingPercent=79.6;
  account.periods.fiveHour={remainingPercent:80,used:20,total:100,unit:'percent',resetAt:new Date(initialTime+1000).toISOString()};
  h.send({tokenball:{data}});
  const card=h.nodes.accounts.children[0],row=card.children[1].children[0];
  assert.equal(card.children[0].children[2].textContent,'80%');
  assert.equal(row.children[2].textContent,'20% / 100%');
  assert.equal(row.children[3].textContent,'80%');
  const time=new Date(account.periods.fiveHour.resetAt),pad=n=>String(n).padStart(2,'0');
  assert.equal(row.children[4].textContent,pad(time.getMonth()+1)+'/'+pad(time.getDate())+' '+pad(time.getHours())+':'+pad(time.getMinutes()));
  assert.equal(row.children.length,5,'one reset date per row, no countdown');
  assert.equal(card.children.length,3,'no decorative cover');
  data.summary.averageRemainingPercent=99.9;h.send({tokenball:{data}});
  assert.match(h.nodes.metrics.children[0].children[0].className,/long-value/);
  assert.equal(h.nodes.metrics.children[0].children[0].textContent,'99.9%');
});

test('each reset and subscription time uses tray period-specific thresholds', () => {
  const h=harness(),data=payload(),account=data.accounts[0];
  for(const [period,critical,warning] of [['fiveHour',1,3],['weekly',24,72],['monthly',72,168]]) {
    const index=['fiveHour','weekly','monthly'].indexOf(period);
    for(const [hours,color] of [[-1,'critical'],[critical,'critical'],[critical+.01,'warning'],[warning,'warning'],[warning+.01,'healthy'],[null,'unknown']]) {
      const value=hours===null?null:new Date(initialTime+hours*3600000).toISOString();
      account.periods[period]={remainingPercent:80,resetAt:value};
      account.subscriptionUntil=value;
      h.send({tokenball:{data}});
      const cell=h.nodes.accounts.children[0].children[1].children[index];
      assert.ok(cell.children[4].className.endsWith(color),period+' '+hours);
      if(period==='monthly')assert.ok(h.nodes.accounts.children[0].children[2].children[1].children[1].className.endsWith(color));
    }
  }
});

test('timer recolors reset, collection, expiry and transport without fabricating quota', () => {
  const h=harness(),data=payload(),account=data.accounts[0];
  account.periods.fiveHour.resetAt=new Date(initialTime+3601000).toISOString();
  account.subscriptionUntil=new Date(initialTime+72*3600000+1000).toISOString();
  h.send({tokenball:{data}});
  assert.match(h.nodes.accounts.children[0].children[1].children[0].children[4].className,/warning/);
  assert.match(h.nodes.accounts.children[0].children[2].children[1].children[1].className,/warning/);
  h.advance(3000);
  const row=h.nodes.accounts.children[0];
  assert.match(row.children[1].children[0].children[4].className,/critical/);
  assert.match(row.children[2].children[0].children[1].className,/critical/);
  assert.match(row.children[2].children[1].children[1].className,/critical/);
  assert.match(h.text(),/80%/);
  h.advance(180000);assert.match(h.nodes.connection.className,/warning/);
  h.send({});assert.match(h.nodes.connection.className,/unknown/);
  h.send({tokenball:{data:{schema:'v99'}}});assert.match(h.nodes.connection.className,/critical/);
});

test('quota update shows actual collection time and turns red when old or stale', () => {
  const h=harness(),data=payload(),account=data.accounts[0];
  account.collectedAt=new Date(initialTime-3600000).toISOString();
  account.staleAt=new Date(initialTime+7200000).toISOString();
  h.send({tokenball:{data}});
  const update=()=>h.nodes.accounts.children[0].children[2].children[0];
  assert.equal(update().children[0].textContent,'额度更新');
  assert.match(update().children[1].textContent,/^\d{2}\/\d{2} \d{2}:\d{2}$/);
  assert.equal(update().children[1].className,'healthy');
  const actual=update().children[1].textContent;
  h.advance(1000);assert.equal(update().children[1].className,'critical');
  data.pushedAt=new Date(initialTime+1000).toISOString();h.send({tokenball:{data}});
  assert.equal(update().children[1].textContent,actual,'heartbeat cannot replace collection time');
  assert.equal(update().children[1].className,'critical');
  account.collectedAt=new Date(initialTime).toISOString();account.stale=true;h.send({tokenball:{data}});
  assert.equal(update().children[1].className,'critical');
  account.collectedAt=null;h.send({tokenball:{data}});
  assert.equal(update().children[1].className,'unknown');
  assert.equal(update().children[1].textContent,'--');
});

test('account badges distinguish healthy, warning, errors and unknown', () => {
  const h=harness(),data=payload(),account=data.accounts[0];
  for(const [status,color] of [['available','healthy'],['warning','warning'],['cooldown','warning'],['exhausted','critical'],['authExpired','critical'],['offline','critical'],['error','critical'],['unknown','unknown'],['disabled','unknown']]) {
    account.status=status;h.send({tokenball:{data}});
    assert.ok(h.nodes.accounts.children[0].children[0].children[1].className.endsWith(color),status);
  }
  account.status='authExpired';account.stale=true;h.send({tokenball:{data}});
  assert.match(h.nodes.accounts.children[0].children[0].children[1].textContent,/登录失效.*缓存/);
});

test('carousel supports buttons, keyboard, dots, wraparound and stable selection across heartbeat', () => {
  const h=harness(),data=payload(),original=data.accounts[0];
  data.accounts=[original,{...original,id:'second',alias:'账号二'},{...original,id:'third',alias:'账号三'}];
  h.send({tokenball:{data}});
  assert.equal(h.nodes.counter.textContent,'1 / 3');
  assert.equal(h.nodes.next.disabled,false);
  h.nodes.next.listeners.click();assert.equal(h.nodes.counter.textContent,'2 / 3');
  assert.equal(h.nodes.accounts.scrollLeft,512);
  const card=h.nodes.accounts.children[1];
  h.advance(3000);assert.equal(h.nodes.accounts.children[1],card);
  data.pushedAt=new Date(initialTime+3000).toISOString();h.send({tokenball:{data}});
  assert.equal(h.nodes.accounts.children[1],card);assert.equal(h.nodes.counter.textContent,'2 / 3');
  h.nodes.accounts.listeners.keydown({key:'ArrowRight',preventDefault(){}});
  assert.equal(h.nodes.counter.textContent,'3 / 3');
  h.nodes.next.listeners.click();assert.equal(h.nodes.counter.textContent,'1 / 3');
  h.nodes.previous.listeners.click();assert.equal(h.nodes.counter.textContent,'3 / 3');
  h.nodes.pagination.children[1].listeners.click();assert.equal(h.nodes.counter.textContent,'2 / 3');
  data.accounts=[data.accounts[1],data.accounts[0]];h.send({tokenball:{data}});
  assert.equal(h.nodes.counter.textContent,'2 / 2');
  data.accounts=[original];h.send({tokenball:{data}});
  assert.equal(h.nodes.next.disabled,true);assert.equal(h.nodes.previous.disabled,true);
  h.send({});assert.equal(h.nodes.counter.textContent,'0 / 0');
});

test('account display names and ordering match the actual tray ranking', () => {
  const source=readFileSync(new URL('../../src-ui/src/views/HoverPanelView.vue',import.meta.url),'utf8').split('<script setup lang="ts">')[1].split('</script>')[0];
  const ast=ts.createSourceFile('tray.ts',source,ts.ScriptTarget.Latest,true);
  const names=['primaryExpiryWindow','findExpiryWindow','highestQuotaWindow','windowPriority','balanceResetTime','accountExpiryTime'];
  const functions=names.map(name=>ast.statements.find(n=>ts.isFunctionDeclaration(n)&&n.name?.text===name).getText(ast)).join('\n');
  const ranking=ast.statements.filter(ts.isVariableStatement).flatMap(n=>Array.from(n.declarationList.declarations)).find(n=>n.name.getText(ast)==='balanceExpiryRanking').initializer.arguments[0].getText(ast);
  const compiled=ts.transpileModule(functions+'\nconst rank='+ranking+';return rank();',{compilerOptions:{target:ts.ScriptTarget.ES2020}}).outputText;
  const fixture=[
    {id:'later',displayName:'晚到期',subscriptionUntil:'2026-12-02T00:00:00Z',nextResetAt:null,windows:[]},
    {id:'month',displayName:'月额度',subscriptionUntil:'2026-12-01T00:00:00Z',windows:[{id:'month',periodType:'monthly',resetAt:'2026-11-30T00:00:00Z'}]},
    {id:'early',displayName:'早恢复',subscriptionUntil:'2026-12-01T00:00:00Z',windows:[{id:'5h',periodType:'rolling',periodSeconds:18000,resetAt:'2026-11-24T00:00:00Z'}]},
    {id:'zhong',displayName:'中账号',windows:[],nextResetAt:null},
    {id:'a',displayName:'阿账号',windows:[],nextResetAt:null},
    {id:'fallback',displayName:'恢复回退',windows:[],nextResetAt:'2026-11-25T00:00:00Z'}
  ];
  const key=w=>w.periodType==='monthly'?'monthly':w.periodSeconds===18000?'fiveHour':'other';
  const expected=new Function('store','windowPeriodKey',compiled)({enabledAccounts:fixture},key);
  const hints=Object.fromEntries(expected.map(row=>[row.account.id,row.window?.resetAt??row.account.nextResetAt??null]));
  const h=harness(),data=payload();
  data.accounts=fixture.map(account=>({...data.accounts[0],...account,alias:'账号 摘要',priorityResetAt:hints[account.id]}));
  h.send({tokenball:{data}});
  const displayed=h.nodes.accounts.children.map(row=>row.children[0].children[0].children[0].textContent);
  assert.deepEqual(displayed,expected.map(row=>row.account.displayName));
  assert.deepEqual(data.accounts.map(a=>a.id),fixture.map(a=>a.id),'input payload not reordered');
  data.accounts[0].displayName='<img src=x onerror=alert(1)>研发';
  h.send({tokenball:{data}});assert.match(h.text(),/<img src=x onerror=alert\(1\)>研发/);
  data.accounts=[{...data.accounts[0],displayName:'',alias:'旧版账号别名'}];
  h.send({tokenball:{data}});assert.match(h.text(),/旧版账号别名/);
});

test('refresh and connection mutations only enqueue delivery after storage work', () => {
  const source = name => readFileSync(new URL('../../src-tauri/src/'+name,import.meta.url),'utf8');
  for(const [file,names] of [
    ['commands/quota.rs',['quota_refresh_connection','refresh_all_internal']],
    ['commands/connection.rs',['connection_save','connection_import_config','connection_import_config_from_file','connection_delete','connection_set_enabled']]
  ]) {
    const text=source(file);
    for(const name of names) {
      const body=text.slice(text.indexOf('fn '+name+'(')).split('\n#[tauri::command]')[0].split('\nasync fn ')[0];
      assert.ok(body.includes('state.phonebox.notify();'),name);
      assert.ok(body.indexOf('state.phonebox.notify();') > body.indexOf('repository::'),name);
      assert.doesNotMatch(body,/phonebox.*\.await/);
    }
  }
  for(const file of ['integrations/phonebox/delivery.rs','integrations/phonebox/payload.rs']) {
    assert.doesNotMatch(source(file),/providers::|sync_connection|refresh_all_internal|AppHandle|WebviewWindow/);
  }
});
