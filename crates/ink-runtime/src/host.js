(() => {
  const timers = new Map();
  const uncurry = fn => Function.prototype.call.bind(fn);
  const get = uncurry(Map.prototype.get), set = uncurry(Map.prototype.set);
  const remove = uncurry(Map.prototype.delete), has = uncurry(Map.prototype.has);
  const number = Number, min = Math.min, max = Math.max, apply = Reflect.apply;
  const scheduleNative = __inkSchedule, cancelNative = __inkCancelTimer;
  let nextTimer = 1;

  function schedule(callback, delay, repeat, args) {
    if (typeof callback !== "function") throw new TypeError("Expected a timer callback");
    const milliseconds = max(0, min(number(delay) || 0, 2147483647));
    const id = nextTimer++;
    if (nextTimer > 4294967295) nextTimer = 1;
    set(timers, id, { callback, milliseconds, repeat, args });
    scheduleNative(id, milliseconds);
    return id;
  }

  globalThis.setTimeout = (callback, delay = 0, ...args) => schedule(callback, delay, false, args);
  globalThis.setInterval = (callback, delay = 0, ...args) => schedule(callback, delay, true, args);
  globalThis.clearTimeout = globalThis.clearInterval = id => {
    id = number(id);
    if (remove(timers, id)) cancelNative(id);
  };
  globalThis.__inkFireTimer = id => {
    const timer = get(timers, id);
    if (!timer) return;
    if (!timer.repeat) remove(timers, id);
    try {
      apply(timer.callback, timer, timer.args);
    } finally {
      if (timer.repeat && has(timers, id)) scheduleNative(id, max(1, timer.milliseconds));
    }
  };
  globalThis.queueMicrotask = callback => {
    if (typeof callback !== "function") throw new TypeError("Expected a microtask callback");
    Promise.resolve().then(callback);
  };
  globalThis.performance = { now: __inkNow, timeOrigin: Date.now() };
  globalThis.console = Object.fromEntries(["log", "info", "warn", "error", "debug"].map(level => [
    level,
    (...values) => __inkPost(JSON.stringify({ type: "log", level, message: values.map(String).join(" ") })),
  ]));
})();
