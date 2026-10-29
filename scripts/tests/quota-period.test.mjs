import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import ts from "typescript";

// Load the actual TypeScript helper with the project's existing compiler.
const source = readFileSync(new URL("../../src-ui/src/utils/quotaPeriod.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ES2020 }
});
const { windowPeriodKey, accountPeriodWindow, periodResetLabel, accountPeriodResetLabel, resetTimeClass, accountPeriodResetClass } =
  await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);

const window = (props = {}) => ({ id: "unknown", name: "额度", periodType: "custom", ...props });
const account = (windows = [], props = {}) => ({ windows, ...props });
const localDate = (month, day, hour, minute) => new Date(2026, month - 1, day, hour, minute).toISOString();

test("recognizes current provider names, ids, types and durations", () => {
  const cases = [
    [{ id: "afp-five-hour", name: "5 小时 AFP", periodSeconds: null }, "fiveHour"],
    [{ id: "afp-five-hour" }, "fiveHour"],
    [{ name: "5 小时 AFP" }, "fiveHour"],
    [{ name: "5 小时" }, "fiveHour"],
    [{ name: "5h" }, "fiveHour"],
    [{ name: "5 h" }, "fiveHour"],
    [{ name: "每周" }, "weekly"],
    [{ name: "近 1 周" }, "weekly"],
    [{ name: "7d" }, "weekly"],
    [{ name: "每月" }, "monthly"],
    [{ name: "近 1 月" }, "monthly"],
    [{ name: "30d" }, "monthly"],
    [{ id: "coding-weekly" }, "weekly"],
    [{ id: "coding-monthly" }, "monthly"],
    [{ periodType: "Weekly" }, "weekly"],
    [{ periodType: "monthly" }, "monthly"],
    [{ periodType: "rolling", periodSeconds: 18000 }, "fiveHour"],
    [{ periodType: "rolling", periodSeconds: 604800 }, "weekly"],
    [{ periodSeconds: 2592000 }, "monthly"]
  ];
  for (const [props, expected] of cases) assert.equal(windowPeriodKey(window(props)), expected, JSON.stringify(props));
});

test("explicit type wins over conflicting aliases and durations", () => {
  assert.equal(windowPeriodKey(window({ periodType: "weekly", name: "5h", periodSeconds: 18000 })), "weekly");
  assert.equal(windowPeriodKey(window({ periodType: "monthly", name: "weekly" })), "monthly");
  assert.equal(windowPeriodKey(window({ periodSeconds: 604800, name: "5h" })), "weekly");
});

test("daily and ambiguous session windows are not presented as supported periods", () => {
  for (const props of [
    { periodType: "daily", id: "5h", periodSeconds: 18000 },
    { id: "afp-daily", name: "每日 AFP" },
    { id: "coding-web-session", name: "当前会话" },
    { id: "coding-short-term", name: "短期用量" },
    { name: "15h" }, { name: "15 小时" }, { name: "17d" }, { name: "130d" }
  ]) assert.equal(windowPeriodKey(window(props)), "other");
});

test("three periods retain three independent reset times", () => {
  const data = account([
    window({ name: "5h", resetAt: localDate(10, 29, 18, 30), remainingPercent: 80 }),
    window({ periodType: "weekly", resetAt: localDate(11, 2, 0, 0), remainingPercent: 60 }),
    window({ periodType: "monthly", resetAt: localDate(11, 20, 0, 0), remainingPercent: 40 })
  ]);
  assert.equal(accountPeriodResetLabel(data, "fiveHour"), "10/29 18:30");
  assert.equal(accountPeriodResetLabel(data, "weekly"), "11/02 00:00");
  assert.equal(accountPeriodResetLabel(data, "monthly"), "11/20 00:00");
});

test("missing period and reset time never borrow account, subscription or other account times", () => {
  const data = account([
    window({ name: "5h", resetAt: null, remainingPercent: 80 }),
    window({ periodType: "weekly", resetAt: localDate(11, 2, 0, 0) })
  ], { nextResetAt: localDate(10, 29, 18, 30), subscriptionUntil: localDate(11, 20, 0, 0), syncedAt: localDate(10, 29, 0, 0) });
  assert.equal(accountPeriodWindow(data, "fiveHour").remainingPercent, 80);
  assert.equal(accountPeriodResetLabel(data, "fiveHour"), "--");
  assert.equal(accountPeriodResetLabel(data, "monthly"), "--");
  assert.equal(accountPeriodWindow(data, "monthly"), null);
  assert.equal(accountPeriodResetLabel(account([window({ name: "5h", resetAt: localDate(10, 30, 12, 0) })]), "fiveHour"), "10/30 12:00");
  assert.equal(accountPeriodResetLabel(data, "fiveHour"), "--");
});

test("first matching window supplies both quota and time without mutation", () => {
  const first = Object.freeze(window({ name: "5h", remainingPercent: 80, resetAt: null, isActive: false }));
  const second = Object.freeze(window({ name: "5h", remainingPercent: 20, resetAt: localDate(10, 29, 18, 30) }));
  const data = Object.freeze(account(Object.freeze([first, second])));
  assert.equal(accountPeriodWindow(data, "fiveHour"), first);
  assert.equal(accountPeriodResetLabel(data, "fiveHour"), "--");
  assert.deepEqual(data.windows, [first, second]);
});

test("unknown quota does not hide a valid reset time", () => {
  const data = account([window({ name: "5h", remainingPercent: null, resetAt: localDate(10, 29, 18, 30) })]);
  assert.equal(accountPeriodWindow(data, "fiveHour").remainingPercent, null);
  assert.equal(accountPeriodResetLabel(data, "fiveHour"), "10/29 18:30");
});

test("missing, invalid and non-positive dates display a placeholder", () => {
  for (const value of [undefined, null, "", "not-a-date", "1970-01-01T00:00:00Z", "1969-12-31T23:59:59Z"]) {
    assert.equal(periodResetLabel(value), "--", String(value));
  }
});

test("valid past dates keep their actual local date instead of being projected forward", () => {
  assert.equal(periodResetLabel(new Date(2020, 0, 2, 3, 4).toISOString()), "01/02 03:04");
});

test("period reset colors use thresholds matched to each period length", () => {
  const now = Date.now();
  const afterHours = (hours) => new Date(now + hours * 3600000).toISOString();
  assert.equal(resetTimeClass(null), "unknown");
  assert.equal(resetTimeClass("invalid"), "unknown");

  for (const [period, critical, warning] of [
    ["fiveHour", 1, 3],
    ["weekly", 24, 72],
    ["monthly", 72, 168]
  ]) {
    assert.equal(resetTimeClass(afterHours(critical), period), "critical", `${period} critical boundary`);
    assert.equal(resetTimeClass(afterHours(critical + 0.25), period), "warning", `${period} warning begins`);
    assert.equal(resetTimeClass(afterHours(warning), period), "warning", `${period} warning boundary`);
    assert.equal(resetTimeClass(afterHours(warning + 0.25), period), "healthy", `${period} healthy begins`);
  }
  assert.equal(resetTimeClass(afterHours(-1), "fiveHour"), "critical");
  assert.equal(resetTimeClass(afterHours(48)), "critical", "unknown period uses priority fallback");
  assert.equal(resetTimeClass(afterHours(120)), "warning", "unknown period uses priority fallback");
  assert.equal(resetTimeClass(afterHours(192)), "healthy", "unknown period uses priority fallback");

  const data = account([
    window({ name: "5h", resetAt: afterHours(0.5) }),
    window({ periodType: "weekly", resetAt: afterHours(48) }),
    window({ periodType: "monthly", resetAt: afterHours(240) })
  ]);
  assert.deepEqual(
    ["fiveHour", "weekly", "monthly"].map(period => accountPeriodResetClass(data, period)),
    ["critical", "warning", "healthy"]
  );
});

test("both pages retain subscription-first ranking and priority selection", () => {
  const fixtures = [
    account([window({ id: "a-week", name: "7d", resetAt: localDate(11, 4, 0, 0) })], { id: "later", displayName: "后", subscriptionUntil: localDate(12, 2, 0, 0) }),
    account([window({ id: "b-month", periodType: "monthly", resetAt: localDate(11, 20, 0, 0) })], { id: "month", displayName: "月", subscriptionUntil: localDate(12, 1, 0, 0) }),
    account([window({ id: "c-5h", name: "5h", resetAt: localDate(10, 29, 18, 30) })], { id: "first", displayName: "先", subscriptionUntil: localDate(12, 1, 0, 0) }),
    account([], { id: "unknown", displayName: "未知", nextResetAt: null }),
    account([], { id: "fallback", displayName: "回退", nextResetAt: localDate(11, 1, 0, 0) })
  ];
  const functions = ["primaryExpiryWindow", "findExpiryWindow", "highestQuotaWindow", "windowPriority", "balanceResetTime", "accountExpiryTime"];
  for (const view of ["ManagementView", "HoverPanelView"]) {
    const path = `src-ui/src/views/${view}.vue`;
    const current = readFileSync(new URL(`../../${path}`, import.meta.url), "utf8");
    const parse = (text) => ts.createSourceFile("view.ts", text.split("<script setup lang=\"ts\">")[1].split("</script>")[0], ts.ScriptTarget.Latest, true);
    const newAst = parse(current);
    const getFunction = (ast, name) => ast.statements.find(node => ts.isFunctionDeclaration(node) && node.name?.text === name).getText(ast);
    const getRanking = (ast) => {
      for (const statement of ast.statements) {
        if (!ts.isVariableStatement(statement)) continue;
        const variable = statement.declarationList.declarations.find(node => node.name.getText(ast) === "balanceExpiryRanking");
        if (variable) return variable.initializer.arguments[0].getText(ast);
      }
      throw new Error("Missing ranking expression");
    };
    const run = (ast) => {
      const body = functions.map(name => getFunction(ast, name)).join("\n") + "\n"
        + "const windowPeriodKey = sharedPeriodKey;"
        + `\nreturn (${getRanking(ast)})();`;
      const compiled = ts.transpileModule(`function run() { ${body} }`, { compilerOptions: { target: ts.ScriptTarget.ES2020 } }).outputText;
      return new Function("store", "sharedPeriodKey", `${compiled}; return run();`)({ enabledAccounts: fixtures }, windowPeriodKey)
        .map(row => ({ id: row.account.id, priority: row.window?.id ?? null }));
    };
    assert.deepEqual(run(newAst), [
      { id: "first", priority: "c-5h" },
      { id: "month", priority: "b-month" },
      { id: "later", priority: "a-week" },
      { id: "fallback", priority: null },
      { id: "unknown", priority: null }
    ]);
  }
});
