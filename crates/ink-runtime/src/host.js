(() => {
  const timers = new Map();
  let nextTimer = 1;

  function schedule(callback, delay, repeat, args) {
    if (typeof callback !== "function") throw new TypeError("Expected a timer callback");
    const milliseconds = Math.max(0, Math.min(Number(delay) || 0, 2147483647));
    const id = nextTimer++;
    if (nextTimer > 4294967295) nextTimer = 1;
    timers.set(id, { callback, milliseconds, repeat, args });
    __inkSchedule(id, milliseconds);
    return id;
  }

  globalThis.setTimeout = (callback, delay = 0, ...args) => schedule(callback, delay, false, args);
  globalThis.setInterval = (callback, delay = 0, ...args) => schedule(callback, delay, true, args);
  globalThis.clearTimeout = globalThis.clearInterval = id => {
    id = Number(id);
    if (timers.delete(id)) __inkCancelTimer(id);
  };
  globalThis.__inkFireTimer = id => {
    const timer = timers.get(id);
    if (!timer) return;
    if (!timer.repeat) timers.delete(id);
    try {
      timer.callback(...timer.args);
    } finally {
      if (timer.repeat && timers.has(id)) __inkSchedule(id, Math.max(1, timer.milliseconds));
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
