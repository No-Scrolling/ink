(() => {
  // Standalone events and cancellation: https://dom.spec.whatwg.org/#aborting-ongoing-activities
  const eventStates = new WeakMap();
  const signalStates = new WeakMap();
  const signalKey = Symbol();
  const exceptionNames = ["", "IndexSizeError", "DOMStringSizeError", "HierarchyRequestError", "WrongDocumentError", "InvalidCharacterError", "NoDataAllowedError", "NoModificationAllowedError", "NotFoundError", "NotSupportedError", "InUseAttributeError", "InvalidStateError", "SyntaxError", "InvalidModificationError", "NamespaceError", "InvalidAccessError", "ValidationError", "TypeMismatchError", "SecurityError", "NetworkError", "AbortError", "URLMismatchError", "QuotaExceededError", "TimeoutError", "InvalidNodeTypeError", "DataCloneError"];

  class DOMException extends Error {
    constructor(message = "", name = "Error") {
      super(String(message));
      this.name = String(name);
    }
    get code() { return Math.max(0, exceptionNames.indexOf(this.name)); }
    get [Symbol.toStringTag]() { return "DOMException"; }
  }

  class Event {
    constructor(type, options = {}) {
      if (arguments.length === 0) throw new TypeError("Event requires a type");
      eventStates.set(this, {
        type: String(type), bubbles: !!options?.bubbles, cancelable: !!options?.cancelable,
        composed: !!options?.composed, target: null, currentTarget: null, phase: 0,
        cancelled: false, stopped: false, immediate: false, passive: false,
        dispatching: false, time: performance.now(),
      });
    }
    get type() { return eventStates.get(this).type; }
    get bubbles() { return eventStates.get(this).bubbles; }
    get cancelable() { return eventStates.get(this).cancelable; }
    get composed() { return eventStates.get(this).composed; }
    get target() { return eventStates.get(this).target; }
    get currentTarget() { return eventStates.get(this).currentTarget; }
    get eventPhase() { return eventStates.get(this).phase; }
    get defaultPrevented() { return eventStates.get(this).cancelled; }
    get timeStamp() { return eventStates.get(this).time; }
    get isTrusted() { return false; }
    get cancelBubble() { return eventStates.get(this).stopped; }
    set cancelBubble(value) { if (value) this.stopPropagation(); }
    get returnValue() { return !this.defaultPrevented; }
    set returnValue(value) { if (!value) this.preventDefault(); }
    stopPropagation() { eventStates.get(this).stopped = true; }
    stopImmediatePropagation() {
      const state = eventStates.get(this);
      state.stopped = state.immediate = true;
    }
    preventDefault() {
      const state = eventStates.get(this);
      if (state.cancelable && !state.passive) state.cancelled = true;
    }
    composedPath() { return this.currentTarget ? [this.currentTarget] : []; }
    get [Symbol.toStringTag]() { return "Event"; }
  }
  for (const [name, value] of Object.entries({ NONE: 0, CAPTURING_PHASE: 1, AT_TARGET: 2, BUBBLING_PHASE: 3 })) {
    Object.defineProperty(Event, name, { value, enumerable: true });
    Object.defineProperty(Event.prototype, name, { value, enumerable: true });
  }

  class CustomEvent extends Event {
    #detail;
    constructor(type, options = {}) { super(type, options); this.#detail = options?.detail ?? null; }
    get detail() { return this.#detail; }
    get [Symbol.toStringTag]() { return "CustomEvent"; }
  }

  class EventTarget {
    #listeners = new Map();
    addEventListener(type, callback, options = {}) {
      if (callback == null) return;
      if (typeof callback !== "function" && typeof callback !== "object") throw new TypeError("Invalid event listener");
      type = String(type);
      const capture = typeof options === "boolean" ? options : !!options?.capture;
      const signal = typeof options === "object" ? options?.signal : undefined;
      if (signal != null && !signalStates.has(signal)) throw new TypeError("Invalid abort signal");
      if (signal?.aborted) return;
      const listeners = this.#listeners.get(type) ?? [];
      if (listeners.some(listener => listener.callback === callback && listener.capture === capture)) return;
      const listener = { callback, capture, once: !!options?.once, passive: !!options?.passive, removed: false, cleanup: undefined };
      if (signal) {
        const remove = () => this.removeEventListener(type, callback, capture);
        const algorithms = signalStates.get(signal).algorithms;
        algorithms.add(remove);
        listener.cleanup = () => algorithms.delete(remove);
      }
      listeners.push(listener);
      this.#listeners.set(type, listeners);
    }
    removeEventListener(type, callback, options = {}) {
      type = String(type);
      const capture = typeof options === "boolean" ? options : !!options?.capture;
      const listeners = this.#listeners.get(type);
      const index = listeners?.findIndex(listener => listener.callback === callback && listener.capture === capture) ?? -1;
      if (index < 0) return;
      const [listener] = listeners.splice(index, 1);
      listener.removed = true;
      listener.cleanup?.();
      if (!listeners.length) this.#listeners.delete(type);
    }
    dispatchEvent(event) {
      const state = eventStates.get(event);
      if (!state) throw new TypeError("Expected an Event");
      if (state.dispatching) throw new DOMException("Event is already being dispatched", "InvalidStateError");
      state.dispatching = true;
      state.target = state.currentTarget = this;
      state.phase = Event.AT_TARGET;
      const listeners = this.#listeners.get(event.type)?.slice() ?? [];
      try {
        for (const capture of [true, false]) {
          for (const listener of listeners) {
            if (state.immediate) break;
            if (listener.removed || listener.capture !== capture) continue;
            if (listener.once) this.removeEventListener(event.type, listener.callback, capture);
            state.passive = listener.passive;
            try {
              if (typeof listener.callback === "function") listener.callback.call(this, event);
              else listener.callback.handleEvent(event);
            } catch (error) {
              console.error(error?.stack ?? error);
            }
          }
        }
      } finally {
        state.currentTarget = null;
        state.phase = 0;
        state.dispatching = state.passive = state.stopped = state.immediate = false;
      }
      return !state.cancelled;
    }
    get [Symbol.toStringTag]() { return "EventTarget"; }
  }

  const dependents = new FinalizationRegistry(sources => {
    for (const { source, reference } of sources) signalStates.get(source)?.dependents.delete(reference);
  });
  function abort(signal, reason) {
    const queue = [signal];
    const changed = [];
    for (let index = 0; index < queue.length; index++) {
      const current = queue[index];
      const state = signalStates.get(current);
      if (state.aborted) continue;
      state.aborted = true;
      state.reason = reason;
      changed.push(current);
      for (const reference of state.dependents) {
        const dependent = reference.deref();
        if (dependent) queue.push(dependent);
      }
      state.dependents.clear();
      for (const { source, reference } of state.sources) signalStates.get(source)?.dependents.delete(reference);
      state.sources.length = 0;
      dependents.unregister(current);
    }
    for (const current of changed) {
      const state = signalStates.get(current);
      for (const algorithm of state.algorithms) algorithm();
      state.algorithms.clear();
      current.dispatchEvent(new Event("abort"));
    }
  }
  const defaultReason = () => new DOMException("The operation was aborted", "AbortError");

  class AbortSignal extends EventTarget {
    #onabort = null;
    #handler = event => this.#onabort?.call(this, event);
    constructor(key) {
      super();
      if (key !== signalKey) throw new TypeError("Illegal constructor");
      signalStates.set(this, { aborted: false, reason: undefined, dependent: false, algorithms: new Set(), dependents: new Set(), sources: [] });
    }
    get aborted() { return signalStates.get(this).aborted; }
    get reason() { return signalStates.get(this).reason; }
    get onabort() { return this.#onabort; }
    set onabort(callback) {
      const next = typeof callback === "function" ? callback : null;
      if (!this.#onabort && next) this.addEventListener("abort", this.#handler);
      else if (this.#onabort && !next) this.removeEventListener("abort", this.#handler);
      this.#onabort = next;
    }
    throwIfAborted() { if (this.aborted) throw this.reason; }
    static abort(reason = defaultReason()) {
      const signal = new AbortSignal(signalKey);
      abort(signal, reason);
      return signal;
    }
    static timeout(milliseconds) {
      milliseconds = Number(milliseconds);
      if (!Number.isSafeInteger(milliseconds) || milliseconds < 0) throw new RangeError("Invalid timeout");
      const signal = new AbortSignal(signalKey);
      const deadline = performance.now() + milliseconds;
      function expire() {
        const remaining = deadline - performance.now();
        if (remaining > 0) setTimeout(expire, Math.min(Math.ceil(remaining), 2147483647));
        else abort(signal, new DOMException("The operation timed out", "TimeoutError"));
      }
      setTimeout(expire, Math.min(milliseconds, 2147483647));
      return signal;
    }
    static any(signals) {
      const sources = [...signals];
      for (const source of sources) if (!signalStates.has(source)) throw new TypeError("Expected AbortSignal values");
      const aborted = sources.find(source => source.aborted);
      if (aborted) return AbortSignal.abort(aborted.reason);
      const signal = new AbortSignal(signalKey);
      const state = signalStates.get(signal);
      state.dependent = true;
      const references = state.sources;
      const roots = new Set();
      // Flatten composed signals so collecting an intermediate signal cannot break cancellation.
      for (const source of sources) {
        const state = signalStates.get(source);
        if (!state.dependent) roots.add(source);
        else for (const { source } of state.sources) roots.add(source);
      }
      for (const source of roots) {
        const reference = new WeakRef(signal);
        signalStates.get(source).dependents.add(reference);
        references.push({ source, reference });
      }
      if (references.length) dependents.register(signal, references, signal);
      return signal;
    }
    get [Symbol.toStringTag]() { return "AbortSignal"; }
  }
  class AbortController {
    #signal = new AbortSignal(signalKey);
    get signal() { return this.#signal; }
    abort(reason = defaultReason()) { abort(this.#signal, reason); }
    get [Symbol.toStringTag]() { return "AbortController"; }
  }
  Object.assign(globalThis, { DOMException, Event, CustomEvent, EventTarget, AbortSignal, AbortController });
})();
