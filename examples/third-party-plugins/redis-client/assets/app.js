(function() {
	//#region ../../../node_modules/.pnpm/@vue+shared@3.5.39/node_modules/@vue/shared/dist/shared.esm-bundler.js
	/**
	* @vue/shared v3.5.39
	* (c) 2018-present Yuxi (Evan) You and Vue contributors
	* @license MIT
	**/
	// @__NO_SIDE_EFFECTS__
	function makeMap(str) {
		const map = /* @__PURE__ */ Object.create(null);
		for (const key of str.split(",")) map[key] = 1;
		return (val) => val in map;
	}
	var EMPTY_OBJ = {};
	var EMPTY_ARR = [];
	var NOOP = () => {};
	var NO = () => false;
	var isOn = (key) => key.charCodeAt(0) === 111 && key.charCodeAt(1) === 110 && (key.charCodeAt(2) > 122 || key.charCodeAt(2) < 97);
	var isModelListener = (key) => key.startsWith("onUpdate:");
	var extend = Object.assign;
	var remove = (arr, el) => {
		const i = arr.indexOf(el);
		if (i > -1) arr.splice(i, 1);
	};
	var hasOwnProperty$1 = Object.prototype.hasOwnProperty;
	var hasOwn = (val, key) => hasOwnProperty$1.call(val, key);
	var isArray = Array.isArray;
	var isMap = (val) => toTypeString(val) === "[object Map]";
	var isSet = (val) => toTypeString(val) === "[object Set]";
	var isDate = (val) => toTypeString(val) === "[object Date]";
	var isFunction = (val) => typeof val === "function";
	var isString = (val) => typeof val === "string";
	var isSymbol = (val) => typeof val === "symbol";
	var isObject = (val) => val !== null && typeof val === "object";
	var isPromise = (val) => {
		return (isObject(val) || isFunction(val)) && isFunction(val.then) && isFunction(val.catch);
	};
	var objectToString = Object.prototype.toString;
	var toTypeString = (value) => objectToString.call(value);
	var toRawType = (value) => {
		return toTypeString(value).slice(8, -1);
	};
	var isPlainObject = (val) => toTypeString(val) === "[object Object]";
	var isIntegerKey = (key) => isString(key) && key !== "NaN" && key[0] !== "-" && "" + parseInt(key, 10) === key;
	var isReservedProp = /* @__PURE__ */ makeMap(",key,ref,ref_for,ref_key,onVnodeBeforeMount,onVnodeMounted,onVnodeBeforeUpdate,onVnodeUpdated,onVnodeBeforeUnmount,onVnodeUnmounted");
	var cacheStringFunction = (fn) => {
		const cache = /* @__PURE__ */ Object.create(null);
		return ((str) => {
			return cache[str] || (cache[str] = fn(str));
		});
	};
	var camelizeRE = /-\w/g;
	var camelize = cacheStringFunction((str) => {
		return str.replace(camelizeRE, (c) => c.slice(1).toUpperCase());
	});
	var hyphenateRE = /\B([A-Z])/g;
	var hyphenate = cacheStringFunction((str) => str.replace(hyphenateRE, "-$1").toLowerCase());
	var capitalize = cacheStringFunction((str) => {
		return str.charAt(0).toUpperCase() + str.slice(1);
	});
	var toHandlerKey = cacheStringFunction((str) => {
		return str ? `on${capitalize(str)}` : ``;
	});
	var hasChanged = (value, oldValue) => !Object.is(value, oldValue);
	var invokeArrayFns = (fns, ...arg) => {
		for (let i = 0; i < fns.length; i++) fns[i](...arg);
	};
	var def = (obj, key, value, writable = false) => {
		Object.defineProperty(obj, key, {
			configurable: true,
			enumerable: false,
			writable,
			value
		});
	};
	var looseToNumber = (val) => {
		const n = parseFloat(val);
		return isNaN(n) ? val : n;
	};
	var _globalThis;
	var getGlobalThis = () => {
		return _globalThis || (_globalThis = typeof globalThis !== "undefined" ? globalThis : typeof self !== "undefined" ? self : typeof window !== "undefined" ? window : typeof global !== "undefined" ? global : {});
	};
	function normalizeStyle(value) {
		if (isArray(value)) {
			const res = {};
			for (let i = 0; i < value.length; i++) {
				const item = value[i];
				const normalized = isString(item) ? parseStringStyle(item) : normalizeStyle(item);
				if (normalized) for (const key in normalized) res[key] = normalized[key];
			}
			return res;
		} else if (isString(value) || isObject(value)) return value;
	}
	var listDelimiterRE = /;(?![^(]*\))/g;
	var propertyDelimiterRE = /:([^]+)/;
	var styleCommentRE = /\/\*[^]*?\*\//g;
	function parseStringStyle(cssText) {
		const ret = {};
		cssText.replace(styleCommentRE, "").split(listDelimiterRE).forEach((item) => {
			if (item) {
				const tmp = item.split(propertyDelimiterRE);
				tmp.length > 1 && (ret[tmp[0].trim()] = tmp[1].trim());
			}
		});
		return ret;
	}
	function normalizeClass(value) {
		let res = "";
		if (isString(value)) res = value;
		else if (isArray(value)) for (let i = 0; i < value.length; i++) {
			const normalized = normalizeClass(value[i]);
			if (normalized) res += normalized + " ";
		}
		else if (isObject(value)) {
			for (const name in value) if (value[name]) res += name + " ";
		}
		return res.trim();
	}
	var specialBooleanAttrs = `itemscope,allowfullscreen,formnovalidate,ismap,nomodule,novalidate,readonly`;
	var isSpecialBooleanAttr = /* @__PURE__ */ makeMap(specialBooleanAttrs);
	specialBooleanAttrs + "";
	function includeBooleanAttr(value) {
		return !!value || value === "";
	}
	function looseCompareArrays(a, b) {
		if (a.length !== b.length) return false;
		let equal = true;
		for (let i = 0; equal && i < a.length; i++) equal = looseEqual(a[i], b[i]);
		return equal;
	}
	function looseEqual(a, b) {
		if (a === b) return true;
		let aValidType = isDate(a);
		let bValidType = isDate(b);
		if (aValidType || bValidType) return aValidType && bValidType ? a.getTime() === b.getTime() : false;
		aValidType = isSymbol(a);
		bValidType = isSymbol(b);
		if (aValidType || bValidType) return a === b;
		aValidType = isArray(a);
		bValidType = isArray(b);
		if (aValidType || bValidType) return aValidType && bValidType ? looseCompareArrays(a, b) : false;
		aValidType = isObject(a);
		bValidType = isObject(b);
		if (aValidType || bValidType) {
			if (!aValidType || !bValidType) return false;
			if (Object.keys(a).length !== Object.keys(b).length) return false;
			for (const key in a) {
				const aHasKey = a.hasOwnProperty(key);
				const bHasKey = b.hasOwnProperty(key);
				if (aHasKey && !bHasKey || !aHasKey && bHasKey || !looseEqual(a[key], b[key])) return false;
			}
		}
		return String(a) === String(b);
	}
	function looseIndexOf(arr, val) {
		return arr.findIndex((item) => looseEqual(item, val));
	}
	var isRef$1 = (val) => {
		return !!(val && val["__v_isRef"] === true);
	};
	var toDisplayString = (val) => {
		return isString(val) ? val : val == null ? "" : isArray(val) || isObject(val) && (val.toString === objectToString || !isFunction(val.toString)) ? isRef$1(val) ? toDisplayString(val.value) : JSON.stringify(val, replacer, 2) : String(val);
	};
	var replacer = (_key, val) => {
		if (isRef$1(val)) return replacer(_key, val.value);
		else if (isMap(val)) return { [`Map(${val.size})`]: [...val.entries()].reduce((entries, [key, val2], i) => {
			entries[stringifySymbol(key, i) + " =>"] = val2;
			return entries;
		}, {}) };
		else if (isSet(val)) return { [`Set(${val.size})`]: [...val.values()].map((v) => stringifySymbol(v)) };
		else if (isSymbol(val)) return stringifySymbol(val);
		else if (isObject(val) && !isArray(val) && !isPlainObject(val)) return String(val);
		return val;
	};
	var stringifySymbol = (v, i = "") => {
		var _a;
		return isSymbol(v) ? `Symbol(${(_a = v.description) != null ? _a : i})` : v;
	};
	//#endregion
	//#region ../../../node_modules/.pnpm/@vue+reactivity@3.5.39/node_modules/@vue/reactivity/dist/reactivity.esm-bundler.js
	/**
	* @vue/reactivity v3.5.39
	* (c) 2018-present Yuxi (Evan) You and Vue contributors
	* @license MIT
	**/
	var activeEffectScope;
	var EffectScope = class {
		constructor(detached = false) {
			this.detached = detached;
			/**
			* @internal
			*/
			this._active = true;
			/**
			* @internal track `on` calls, allow `on` call multiple times
			*/
			this._on = 0;
			/**
			* @internal
			*/
			this.effects = [];
			/**
			* @internal
			*/
			this.cleanups = [];
			this._isPaused = false;
			this._warnOnRun = true;
			this.__v_skip = true;
			if (!detached && activeEffectScope) if (activeEffectScope.active) {
				this.parent = activeEffectScope;
				this.index = (activeEffectScope.scopes || (activeEffectScope.scopes = [])).push(this) - 1;
			} else {
				this._active = false;
				this._warnOnRun = false;
			}
		}
		get active() {
			return this._active;
		}
		pause() {
			if (this._active) {
				this._isPaused = true;
				let i, l;
				if (this.scopes) for (i = 0, l = this.scopes.length; i < l; i++) this.scopes[i].pause();
				for (i = 0, l = this.effects.length; i < l; i++) this.effects[i].pause();
			}
		}
		/**
		* Resumes the effect scope, including all child scopes and effects.
		*/
		resume() {
			if (this._active) {
				if (this._isPaused) {
					this._isPaused = false;
					let i, l;
					if (this.scopes) for (i = 0, l = this.scopes.length; i < l; i++) this.scopes[i].resume();
					for (i = 0, l = this.effects.length; i < l; i++) this.effects[i].resume();
				}
			}
		}
		run(fn) {
			if (this._active) {
				const currentEffectScope = activeEffectScope;
				try {
					activeEffectScope = this;
					return fn();
				} finally {
					activeEffectScope = currentEffectScope;
				}
			}
		}
		/**
		* This should only be called on non-detached scopes
		* @internal
		*/
		on() {
			if (++this._on === 1) {
				this.prevScope = activeEffectScope;
				activeEffectScope = this;
			}
		}
		/**
		* This should only be called on non-detached scopes
		* @internal
		*/
		off() {
			if (this._on > 0 && --this._on === 0) {
				if (activeEffectScope === this) activeEffectScope = this.prevScope;
				else {
					let current = activeEffectScope;
					while (current) {
						if (current.prevScope === this) {
							current.prevScope = this.prevScope;
							break;
						}
						current = current.prevScope;
					}
				}
				this.prevScope = void 0;
			}
		}
		stop(fromParent) {
			if (this._active) {
				this._active = false;
				let i, l;
				for (i = 0, l = this.effects.length; i < l; i++) this.effects[i].stop();
				this.effects.length = 0;
				for (i = 0, l = this.cleanups.length; i < l; i++) this.cleanups[i]();
				this.cleanups.length = 0;
				if (this.scopes) {
					for (i = 0, l = this.scopes.length; i < l; i++) this.scopes[i].stop(true);
					this.scopes.length = 0;
				}
				if (!this.detached && this.parent && !fromParent) {
					const last = this.parent.scopes.pop();
					if (last && last !== this) {
						this.parent.scopes[this.index] = last;
						last.index = this.index;
					}
				}
				this.parent = void 0;
			}
		}
	};
	function getCurrentScope() {
		return activeEffectScope;
	}
	var activeSub;
	var pausedQueueEffects = /* @__PURE__ */ new WeakSet();
	var ReactiveEffect = class {
		constructor(fn) {
			this.fn = fn;
			/**
			* @internal
			*/
			this.deps = void 0;
			/**
			* @internal
			*/
			this.depsTail = void 0;
			/**
			* @internal
			*/
			this.flags = 5;
			/**
			* @internal
			*/
			this.next = void 0;
			/**
			* @internal
			*/
			this.cleanup = void 0;
			this.scheduler = void 0;
			if (activeEffectScope) if (activeEffectScope.active) activeEffectScope.effects.push(this);
			else this.flags &= -2;
		}
		pause() {
			this.flags |= 64;
		}
		resume() {
			if (this.flags & 64) {
				this.flags &= -65;
				if (pausedQueueEffects.has(this)) {
					pausedQueueEffects.delete(this);
					this.trigger();
				}
			}
		}
		/**
		* @internal
		*/
		notify() {
			if (this.flags & 2 && !(this.flags & 32)) return;
			if (!(this.flags & 8)) batch(this);
		}
		run() {
			if (!(this.flags & 1)) return this.fn();
			this.flags |= 2;
			cleanupEffect(this);
			prepareDeps(this);
			const prevEffect = activeSub;
			const prevShouldTrack = shouldTrack;
			activeSub = this;
			shouldTrack = true;
			try {
				return this.fn();
			} finally {
				cleanupDeps(this);
				activeSub = prevEffect;
				shouldTrack = prevShouldTrack;
				this.flags &= -3;
			}
		}
		stop() {
			if (this.flags & 1) {
				for (let link = this.deps; link; link = link.nextDep) removeSub(link);
				this.deps = this.depsTail = void 0;
				cleanupEffect(this);
				this.onStop && this.onStop();
				this.flags &= -2;
			}
		}
		trigger() {
			if (this.flags & 64) pausedQueueEffects.add(this);
			else if (this.scheduler) this.scheduler();
			else this.runIfDirty();
		}
		/**
		* @internal
		*/
		runIfDirty() {
			if (isDirty(this)) this.run();
		}
		get dirty() {
			return isDirty(this);
		}
	};
	var batchDepth = 0;
	var batchedSub;
	var batchedComputed;
	function batch(sub, isComputed = false) {
		sub.flags |= 8;
		if (isComputed) {
			sub.next = batchedComputed;
			batchedComputed = sub;
			return;
		}
		sub.next = batchedSub;
		batchedSub = sub;
	}
	function startBatch() {
		batchDepth++;
	}
	function endBatch() {
		if (--batchDepth > 0) return;
		if (batchedComputed) {
			let e = batchedComputed;
			batchedComputed = void 0;
			while (e) {
				const next = e.next;
				e.next = void 0;
				e.flags &= -9;
				e = next;
			}
		}
		let error;
		while (batchedSub) {
			let e = batchedSub;
			batchedSub = void 0;
			while (e) {
				const next = e.next;
				e.next = void 0;
				e.flags &= -9;
				if (e.flags & 1) try {
					e.trigger();
				} catch (err) {
					if (!error) error = err;
				}
				e = next;
			}
		}
		if (error) throw error;
	}
	function prepareDeps(sub) {
		for (let link = sub.deps; link; link = link.nextDep) {
			link.version = -1;
			link.prevActiveLink = link.dep.activeLink;
			link.dep.activeLink = link;
		}
	}
	function cleanupDeps(sub) {
		let head;
		let tail = sub.depsTail;
		let link = tail;
		while (link) {
			const prev = link.prevDep;
			if (link.version === -1) {
				if (link === tail) tail = prev;
				removeSub(link);
				removeDep(link);
			} else head = link;
			link.dep.activeLink = link.prevActiveLink;
			link.prevActiveLink = void 0;
			link = prev;
		}
		sub.deps = head;
		sub.depsTail = tail;
	}
	function isDirty(sub) {
		for (let link = sub.deps; link; link = link.nextDep) if (link.dep.version !== link.version || link.dep.computed && (refreshComputed(link.dep.computed) || link.dep.version !== link.version)) return true;
		if (sub._dirty) return true;
		return false;
	}
	function refreshComputed(computed) {
		if (computed.flags & 4 && !(computed.flags & 16)) return;
		computed.flags &= -17;
		if (computed.globalVersion === globalVersion) return;
		computed.globalVersion = globalVersion;
		if (!computed.isSSR && computed.flags & 128 && (!computed.deps && !computed._dirty || !isDirty(computed))) return;
		computed.flags |= 2;
		const dep = computed.dep;
		const prevSub = activeSub;
		const prevShouldTrack = shouldTrack;
		activeSub = computed;
		shouldTrack = true;
		try {
			prepareDeps(computed);
			const value = computed.fn(computed._value);
			if (dep.version === 0 || hasChanged(value, computed._value)) {
				computed.flags |= 128;
				computed._value = value;
				dep.version++;
			}
		} catch (err) {
			dep.version++;
			throw err;
		} finally {
			activeSub = prevSub;
			shouldTrack = prevShouldTrack;
			cleanupDeps(computed);
			computed.flags &= -3;
		}
	}
	function removeSub(link, soft = false) {
		const { dep, prevSub, nextSub } = link;
		if (prevSub) {
			prevSub.nextSub = nextSub;
			link.prevSub = void 0;
		}
		if (nextSub) {
			nextSub.prevSub = prevSub;
			link.nextSub = void 0;
		}
		if (dep.subs === link) {
			dep.subs = prevSub;
			if (!prevSub && dep.computed) {
				dep.computed.flags &= -5;
				for (let l = dep.computed.deps; l; l = l.nextDep) removeSub(l, true);
			}
		}
		if (!soft && !--dep.sc && dep.map) dep.map.delete(dep.key);
	}
	function removeDep(link) {
		const { prevDep, nextDep } = link;
		if (prevDep) {
			prevDep.nextDep = nextDep;
			link.prevDep = void 0;
		}
		if (nextDep) {
			nextDep.prevDep = prevDep;
			link.nextDep = void 0;
		}
	}
	var shouldTrack = true;
	var trackStack = [];
	function pauseTracking() {
		trackStack.push(shouldTrack);
		shouldTrack = false;
	}
	function resetTracking() {
		const last = trackStack.pop();
		shouldTrack = last === void 0 ? true : last;
	}
	function cleanupEffect(e) {
		const { cleanup } = e;
		e.cleanup = void 0;
		if (cleanup) {
			const prevSub = activeSub;
			activeSub = void 0;
			try {
				cleanup();
			} finally {
				activeSub = prevSub;
			}
		}
	}
	var globalVersion = 0;
	var Link = class {
		constructor(sub, dep) {
			this.sub = sub;
			this.dep = dep;
			this.version = dep.version;
			this.nextDep = this.prevDep = this.nextSub = this.prevSub = this.prevActiveLink = void 0;
		}
	};
	var Dep = class {
		constructor(computed) {
			this.computed = computed;
			this.version = 0;
			/**
			* Link between this dep and the current active effect
			*/
			this.activeLink = void 0;
			/**
			* Doubly linked list representing the subscribing effects (tail)
			*/
			this.subs = void 0;
			/**
			* For object property deps cleanup
			*/
			this.map = void 0;
			this.key = void 0;
			/**
			* Subscriber counter
			*/
			this.sc = 0;
			/**
			* @internal
			*/
			this.__v_skip = true;
		}
		track(debugInfo) {
			if (!activeSub || !shouldTrack || activeSub === this.computed) return;
			let link = this.activeLink;
			if (link === void 0 || link.sub !== activeSub) {
				link = this.activeLink = new Link(activeSub, this);
				if (!activeSub.deps) activeSub.deps = activeSub.depsTail = link;
				else {
					link.prevDep = activeSub.depsTail;
					activeSub.depsTail.nextDep = link;
					activeSub.depsTail = link;
				}
				addSub(link);
			} else if (link.version === -1) {
				link.version = this.version;
				if (link.nextDep) {
					const next = link.nextDep;
					next.prevDep = link.prevDep;
					if (link.prevDep) link.prevDep.nextDep = next;
					link.prevDep = activeSub.depsTail;
					link.nextDep = void 0;
					activeSub.depsTail.nextDep = link;
					activeSub.depsTail = link;
					if (activeSub.deps === link) activeSub.deps = next;
				}
			}
			return link;
		}
		trigger(debugInfo) {
			this.version++;
			globalVersion++;
			this.notify(debugInfo);
		}
		notify(debugInfo) {
			startBatch();
			try {
				for (let link = this.subs; link; link = link.prevSub) if (link.sub.notify()) link.sub.dep.notify();
			} finally {
				endBatch();
			}
		}
	};
	function addSub(link) {
		link.dep.sc++;
		if (link.sub.flags & 4) {
			const computed = link.dep.computed;
			if (computed && !link.dep.subs) {
				computed.flags |= 20;
				for (let l = computed.deps; l; l = l.nextDep) addSub(l);
			}
			const currentTail = link.dep.subs;
			if (currentTail !== link) {
				link.prevSub = currentTail;
				if (currentTail) currentTail.nextSub = link;
			}
			link.dep.subs = link;
		}
	}
	var targetMap = /* @__PURE__ */ new WeakMap();
	var ITERATE_KEY = /* @__PURE__ */ Symbol("");
	var MAP_KEY_ITERATE_KEY = /* @__PURE__ */ Symbol("");
	var ARRAY_ITERATE_KEY = /* @__PURE__ */ Symbol("");
	function track(target, type, key) {
		if (shouldTrack && activeSub) {
			let depsMap = targetMap.get(target);
			if (!depsMap) targetMap.set(target, depsMap = /* @__PURE__ */ new Map());
			let dep = depsMap.get(key);
			if (!dep) {
				depsMap.set(key, dep = new Dep());
				dep.map = depsMap;
				dep.key = key;
			}
			dep.track();
		}
	}
	function trigger(target, type, key, newValue, oldValue, oldTarget) {
		const depsMap = targetMap.get(target);
		if (!depsMap) {
			globalVersion++;
			return;
		}
		const run = (dep) => {
			if (dep) dep.trigger();
		};
		startBatch();
		if (type === "clear") depsMap.forEach(run);
		else {
			const targetIsArray = isArray(target);
			const isArrayIndex = targetIsArray && isIntegerKey(key);
			if (targetIsArray && key === "length") {
				const newLength = Number(newValue);
				depsMap.forEach((dep, key2) => {
					if (key2 === "length" || key2 === ARRAY_ITERATE_KEY || !isSymbol(key2) && key2 >= newLength) run(dep);
				});
			} else {
				if (key !== void 0 || depsMap.has(void 0)) run(depsMap.get(key));
				if (isArrayIndex) run(depsMap.get(ARRAY_ITERATE_KEY));
				switch (type) {
					case "add":
						if (!targetIsArray) {
							run(depsMap.get(ITERATE_KEY));
							if (isMap(target)) run(depsMap.get(MAP_KEY_ITERATE_KEY));
						} else if (isArrayIndex) run(depsMap.get("length"));
						break;
					case "delete":
						if (!targetIsArray) {
							run(depsMap.get(ITERATE_KEY));
							if (isMap(target)) run(depsMap.get(MAP_KEY_ITERATE_KEY));
						}
						break;
					case "set":
						if (isMap(target)) run(depsMap.get(ITERATE_KEY));
						break;
				}
			}
		}
		endBatch();
	}
	function reactiveReadArray(array) {
		const raw = /* @__PURE__ */ toRaw(array);
		if (raw === array) return raw;
		track(raw, "iterate", ARRAY_ITERATE_KEY);
		return /* @__PURE__ */ isShallow(array) ? raw : raw.map(toReactive);
	}
	function shallowReadArray(arr) {
		track(arr = /* @__PURE__ */ toRaw(arr), "iterate", ARRAY_ITERATE_KEY);
		return arr;
	}
	function toWrapped(target, item) {
		if (/* @__PURE__ */ isReadonly(target)) return /* @__PURE__ */ isReactive(target) ? toReadonly(toReactive(item)) : toReadonly(item);
		return toReactive(item);
	}
	var arrayInstrumentations = {
		__proto__: null,
		[Symbol.iterator]() {
			return iterator(this, Symbol.iterator, (item) => toWrapped(this, item));
		},
		concat(...args) {
			return reactiveReadArray(this).concat(...args.map((x) => isArray(x) ? reactiveReadArray(x) : x));
		},
		entries() {
			return iterator(this, "entries", (value) => {
				value[1] = toWrapped(this, value[1]);
				return value;
			});
		},
		every(fn, thisArg) {
			return apply(this, "every", fn, thisArg, void 0, arguments);
		},
		filter(fn, thisArg) {
			return apply(this, "filter", fn, thisArg, (v) => v.map((item) => toWrapped(this, item)), arguments);
		},
		find(fn, thisArg) {
			return apply(this, "find", fn, thisArg, (item) => toWrapped(this, item), arguments);
		},
		findIndex(fn, thisArg) {
			return apply(this, "findIndex", fn, thisArg, void 0, arguments);
		},
		findLast(fn, thisArg) {
			return apply(this, "findLast", fn, thisArg, (item) => toWrapped(this, item), arguments);
		},
		findLastIndex(fn, thisArg) {
			return apply(this, "findLastIndex", fn, thisArg, void 0, arguments);
		},
		forEach(fn, thisArg) {
			return apply(this, "forEach", fn, thisArg, void 0, arguments);
		},
		includes(...args) {
			return searchProxy(this, "includes", args);
		},
		indexOf(...args) {
			return searchProxy(this, "indexOf", args);
		},
		join(separator) {
			return reactiveReadArray(this).join(separator);
		},
		lastIndexOf(...args) {
			return searchProxy(this, "lastIndexOf", args);
		},
		map(fn, thisArg) {
			return apply(this, "map", fn, thisArg, void 0, arguments);
		},
		pop() {
			return noTracking(this, "pop");
		},
		push(...args) {
			return noTracking(this, "push", args);
		},
		reduce(fn, ...args) {
			return reduce(this, "reduce", fn, args);
		},
		reduceRight(fn, ...args) {
			return reduce(this, "reduceRight", fn, args);
		},
		shift() {
			return noTracking(this, "shift");
		},
		some(fn, thisArg) {
			return apply(this, "some", fn, thisArg, void 0, arguments);
		},
		splice(...args) {
			return noTracking(this, "splice", args);
		},
		toReversed() {
			return reactiveReadArray(this).toReversed();
		},
		toSorted(comparer) {
			return reactiveReadArray(this).toSorted(comparer);
		},
		toSpliced(...args) {
			return reactiveReadArray(this).toSpliced(...args);
		},
		unshift(...args) {
			return noTracking(this, "unshift", args);
		},
		values() {
			return iterator(this, "values", (item) => toWrapped(this, item));
		}
	};
	function iterator(self, method, wrapValue) {
		const arr = shallowReadArray(self);
		const iter = arr[method]();
		if (arr !== self && !/* @__PURE__ */ isShallow(self)) {
			iter._next = iter.next;
			iter.next = () => {
				const result = iter._next();
				if (!result.done) result.value = wrapValue(result.value);
				return result;
			};
		}
		return iter;
	}
	var arrayProto = Array.prototype;
	function apply(self, method, fn, thisArg, wrappedRetFn, args) {
		const arr = shallowReadArray(self);
		const needsWrap = arr !== self && !/* @__PURE__ */ isShallow(self);
		const methodFn = arr[method];
		if (methodFn !== arrayProto[method]) {
			const result2 = methodFn.apply(self, args);
			return needsWrap ? toReactive(result2) : result2;
		}
		let wrappedFn = fn;
		if (arr !== self) {
			if (needsWrap) wrappedFn = function(item, index) {
				return fn.call(this, toWrapped(self, item), index, self);
			};
			else if (fn.length > 2) wrappedFn = function(item, index) {
				return fn.call(this, item, index, self);
			};
		}
		const result = methodFn.call(arr, wrappedFn, thisArg);
		return needsWrap && wrappedRetFn ? wrappedRetFn(result) : result;
	}
	function reduce(self, method, fn, args) {
		const arr = shallowReadArray(self);
		const needsWrap = arr !== self && !/* @__PURE__ */ isShallow(self);
		let wrappedFn = fn;
		let wrapInitialAccumulator = false;
		if (arr !== self) {
			if (needsWrap) {
				wrapInitialAccumulator = args.length === 0;
				wrappedFn = function(acc, item, index) {
					if (wrapInitialAccumulator) {
						wrapInitialAccumulator = false;
						acc = toWrapped(self, acc);
					}
					return fn.call(this, acc, toWrapped(self, item), index, self);
				};
			} else if (fn.length > 3) wrappedFn = function(acc, item, index) {
				return fn.call(this, acc, item, index, self);
			};
		}
		const result = arr[method](wrappedFn, ...args);
		return wrapInitialAccumulator ? toWrapped(self, result) : result;
	}
	function searchProxy(self, method, args) {
		const arr = /* @__PURE__ */ toRaw(self);
		track(arr, "iterate", ARRAY_ITERATE_KEY);
		const res = arr[method](...args);
		if ((res === -1 || res === false) && /* @__PURE__ */ isProxy(args[0])) {
			args[0] = /* @__PURE__ */ toRaw(args[0]);
			return arr[method](...args);
		}
		return res;
	}
	function noTracking(self, method, args = []) {
		pauseTracking();
		startBatch();
		const res = (/* @__PURE__ */ toRaw(self))[method].apply(self, args);
		endBatch();
		resetTracking();
		return res;
	}
	var isNonTrackableKeys = /* @__PURE__ */ makeMap(`__proto__,__v_isRef,__isVue`);
	var builtInSymbols = new Set(/* @__PURE__ */ Object.getOwnPropertyNames(Symbol).filter((key) => key !== "arguments" && key !== "caller").map((key) => Symbol[key]).filter(isSymbol));
	function hasOwnProperty(key) {
		if (!isSymbol(key)) key = String(key);
		const obj = /* @__PURE__ */ toRaw(this);
		track(obj, "has", key);
		return obj.hasOwnProperty(key);
	}
	var BaseReactiveHandler = class {
		constructor(_isReadonly = false, _isShallow = false) {
			this._isReadonly = _isReadonly;
			this._isShallow = _isShallow;
		}
		get(target, key, receiver) {
			if (key === "__v_skip") return target["__v_skip"];
			const isReadonly2 = this._isReadonly, isShallow2 = this._isShallow;
			if (key === "__v_isReactive") return !isReadonly2;
			else if (key === "__v_isReadonly") return isReadonly2;
			else if (key === "__v_isShallow") return isShallow2;
			else if (key === "__v_raw") {
				if (receiver === (isReadonly2 ? isShallow2 ? shallowReadonlyMap : readonlyMap : isShallow2 ? shallowReactiveMap : reactiveMap).get(target) || Object.getPrototypeOf(target) === Object.getPrototypeOf(receiver)) return target;
				return;
			}
			const targetIsArray = isArray(target);
			if (!isReadonly2) {
				let fn;
				if (targetIsArray && (fn = arrayInstrumentations[key])) return fn;
				if (key === "hasOwnProperty") return hasOwnProperty;
			}
			const res = Reflect.get(target, key, /* @__PURE__ */ isRef(target) ? target : receiver);
			if (isSymbol(key) ? builtInSymbols.has(key) : isNonTrackableKeys(key)) return res;
			if (!isReadonly2) track(target, "get", key);
			if (isShallow2) return res;
			if (/* @__PURE__ */ isRef(res)) {
				const value = targetIsArray && isIntegerKey(key) ? res : res.value;
				return isReadonly2 && isObject(value) ? /* @__PURE__ */ readonly(value) : value;
			}
			if (isObject(res)) return isReadonly2 ? /* @__PURE__ */ readonly(res) : /* @__PURE__ */ reactive(res);
			return res;
		}
	};
	var MutableReactiveHandler = class extends BaseReactiveHandler {
		constructor(isShallow2 = false) {
			super(false, isShallow2);
		}
		set(target, key, value, receiver) {
			let oldValue = target[key];
			const isArrayWithIntegerKey = isArray(target) && isIntegerKey(key);
			if (!this._isShallow) {
				const isOldValueReadonly = /* @__PURE__ */ isReadonly(oldValue);
				if (!/* @__PURE__ */ isShallow(value) && !/* @__PURE__ */ isReadonly(value)) {
					oldValue = /* @__PURE__ */ toRaw(oldValue);
					value = /* @__PURE__ */ toRaw(value);
				}
				if (!isArrayWithIntegerKey && /* @__PURE__ */ isRef(oldValue) && !/* @__PURE__ */ isRef(value)) if (isOldValueReadonly) return true;
				else {
					oldValue.value = value;
					return true;
				}
			}
			const hadKey = isArrayWithIntegerKey ? Number(key) < target.length : hasOwn(target, key);
			const result = Reflect.set(target, key, value, /* @__PURE__ */ isRef(target) ? target : receiver);
			if (target === /* @__PURE__ */ toRaw(receiver) && result) {
				if (!hadKey) trigger(target, "add", key, value);
				else if (hasChanged(value, oldValue)) trigger(target, "set", key, value, oldValue);
			}
			return result;
		}
		deleteProperty(target, key) {
			const hadKey = hasOwn(target, key);
			const oldValue = target[key];
			const result = Reflect.deleteProperty(target, key);
			if (result && hadKey) trigger(target, "delete", key, void 0, oldValue);
			return result;
		}
		has(target, key) {
			const result = Reflect.has(target, key);
			if (!isSymbol(key) || !builtInSymbols.has(key)) track(target, "has", key);
			return result;
		}
		ownKeys(target) {
			track(target, "iterate", isArray(target) ? "length" : ITERATE_KEY);
			return Reflect.ownKeys(target);
		}
	};
	var ReadonlyReactiveHandler = class extends BaseReactiveHandler {
		constructor(isShallow2 = false) {
			super(true, isShallow2);
		}
		set(target, key) {
			return true;
		}
		deleteProperty(target, key) {
			return true;
		}
	};
	var mutableHandlers = /* @__PURE__ */ new MutableReactiveHandler();
	var readonlyHandlers = /* @__PURE__ */ new ReadonlyReactiveHandler();
	var shallowReactiveHandlers = /* @__PURE__ */ new MutableReactiveHandler(true);
	var toShallow = (value) => value;
	var getProto = (v) => Reflect.getPrototypeOf(v);
	function createIterableMethod(method, isReadonly2, isShallow2) {
		return function(...args) {
			const target = this["__v_raw"];
			const rawTarget = /* @__PURE__ */ toRaw(target);
			const targetIsMap = isMap(rawTarget);
			const isPair = method === "entries" || method === Symbol.iterator && targetIsMap;
			const isKeyOnly = method === "keys" && targetIsMap;
			const innerIterator = target[method](...args);
			const wrap = isShallow2 ? toShallow : isReadonly2 ? toReadonly : toReactive;
			!isReadonly2 && track(rawTarget, "iterate", isKeyOnly ? MAP_KEY_ITERATE_KEY : ITERATE_KEY);
			return extend(Object.create(innerIterator), { next() {
				const { value, done } = innerIterator.next();
				return done ? {
					value,
					done
				} : {
					value: isPair ? [wrap(value[0]), wrap(value[1])] : wrap(value),
					done
				};
			} });
		};
	}
	function createReadonlyMethod(type) {
		return function(...args) {
			return type === "delete" ? false : type === "clear" ? void 0 : this;
		};
	}
	function createInstrumentations(readonly, shallow) {
		const instrumentations = {
			get(key) {
				const target = this["__v_raw"];
				const rawTarget = /* @__PURE__ */ toRaw(target);
				const rawKey = /* @__PURE__ */ toRaw(key);
				if (!readonly) {
					if (hasChanged(key, rawKey)) track(rawTarget, "get", key);
					track(rawTarget, "get", rawKey);
				}
				const { has } = getProto(rawTarget);
				const wrap = shallow ? toShallow : readonly ? toReadonly : toReactive;
				if (has.call(rawTarget, key)) return wrap(target.get(key));
				else if (has.call(rawTarget, rawKey)) return wrap(target.get(rawKey));
				else if (target !== rawTarget) target.get(key);
			},
			get size() {
				const target = this["__v_raw"];
				!readonly && track(/* @__PURE__ */ toRaw(target), "iterate", ITERATE_KEY);
				return target.size;
			},
			has(key) {
				const target = this["__v_raw"];
				const rawTarget = /* @__PURE__ */ toRaw(target);
				const rawKey = /* @__PURE__ */ toRaw(key);
				if (!readonly) {
					if (hasChanged(key, rawKey)) track(rawTarget, "has", key);
					track(rawTarget, "has", rawKey);
				}
				return key === rawKey ? target.has(key) : target.has(key) || target.has(rawKey);
			},
			forEach(callback, thisArg) {
				const observed = this;
				const target = observed["__v_raw"];
				const rawTarget = /* @__PURE__ */ toRaw(target);
				const wrap = shallow ? toShallow : readonly ? toReadonly : toReactive;
				!readonly && track(rawTarget, "iterate", ITERATE_KEY);
				return target.forEach((value, key) => {
					return callback.call(thisArg, wrap(value), wrap(key), observed);
				});
			}
		};
		extend(instrumentations, readonly ? {
			add: createReadonlyMethod("add"),
			set: createReadonlyMethod("set"),
			delete: createReadonlyMethod("delete"),
			clear: createReadonlyMethod("clear")
		} : {
			add(value) {
				const target = /* @__PURE__ */ toRaw(this);
				const proto = getProto(target);
				const rawValue = /* @__PURE__ */ toRaw(value);
				const valueToAdd = !shallow && !/* @__PURE__ */ isShallow(value) && !/* @__PURE__ */ isReadonly(value) ? rawValue : value;
				if (!(proto.has.call(target, valueToAdd) || hasChanged(value, valueToAdd) && proto.has.call(target, value) || hasChanged(rawValue, valueToAdd) && proto.has.call(target, rawValue))) {
					target.add(valueToAdd);
					trigger(target, "add", valueToAdd, valueToAdd);
				}
				return this;
			},
			set(key, value) {
				if (!shallow && !/* @__PURE__ */ isShallow(value) && !/* @__PURE__ */ isReadonly(value)) value = /* @__PURE__ */ toRaw(value);
				const target = /* @__PURE__ */ toRaw(this);
				const { has, get } = getProto(target);
				let hadKey = has.call(target, key);
				if (!hadKey) {
					key = /* @__PURE__ */ toRaw(key);
					hadKey = has.call(target, key);
				}
				const oldValue = get.call(target, key);
				target.set(key, value);
				if (!hadKey) trigger(target, "add", key, value);
				else if (hasChanged(value, oldValue)) trigger(target, "set", key, value, oldValue);
				return this;
			},
			delete(key) {
				const target = /* @__PURE__ */ toRaw(this);
				const { has, get } = getProto(target);
				let hadKey = has.call(target, key);
				if (!hadKey) {
					key = /* @__PURE__ */ toRaw(key);
					hadKey = has.call(target, key);
				}
				const oldValue = get ? get.call(target, key) : void 0;
				const result = target.delete(key);
				if (hadKey) trigger(target, "delete", key, void 0, oldValue);
				return result;
			},
			clear() {
				const target = /* @__PURE__ */ toRaw(this);
				const hadItems = target.size !== 0;
				const oldTarget = void 0;
				const result = target.clear();
				if (hadItems) trigger(target, "clear", void 0, void 0, oldTarget);
				return result;
			}
		});
		[
			"keys",
			"values",
			"entries",
			Symbol.iterator
		].forEach((method) => {
			instrumentations[method] = createIterableMethod(method, readonly, shallow);
		});
		return instrumentations;
	}
	function createInstrumentationGetter(isReadonly2, shallow) {
		const instrumentations = createInstrumentations(isReadonly2, shallow);
		return (target, key, receiver) => {
			if (key === "__v_isReactive") return !isReadonly2;
			else if (key === "__v_isReadonly") return isReadonly2;
			else if (key === "__v_raw") return target;
			return Reflect.get(hasOwn(instrumentations, key) && key in target ? instrumentations : target, key, receiver);
		};
	}
	var mutableCollectionHandlers = { get: /* @__PURE__ */ createInstrumentationGetter(false, false) };
	var shallowCollectionHandlers = { get: /* @__PURE__ */ createInstrumentationGetter(false, true) };
	var readonlyCollectionHandlers = { get: /* @__PURE__ */ createInstrumentationGetter(true, false) };
	var reactiveMap = /* @__PURE__ */ new WeakMap();
	var shallowReactiveMap = /* @__PURE__ */ new WeakMap();
	var readonlyMap = /* @__PURE__ */ new WeakMap();
	var shallowReadonlyMap = /* @__PURE__ */ new WeakMap();
	function targetTypeMap(rawType) {
		switch (rawType) {
			case "Object":
			case "Array": return 1;
			case "Map":
			case "Set":
			case "WeakMap":
			case "WeakSet": return 2;
			default: return 0;
		}
	}
	// @__NO_SIDE_EFFECTS__
	function reactive(target) {
		if (/* @__PURE__ */ isReadonly(target)) return target;
		return createReactiveObject(target, false, mutableHandlers, mutableCollectionHandlers, reactiveMap);
	}
	// @__NO_SIDE_EFFECTS__
	function shallowReactive(target) {
		return createReactiveObject(target, false, shallowReactiveHandlers, shallowCollectionHandlers, shallowReactiveMap);
	}
	// @__NO_SIDE_EFFECTS__
	function readonly(target) {
		return createReactiveObject(target, true, readonlyHandlers, readonlyCollectionHandlers, readonlyMap);
	}
	function createReactiveObject(target, isReadonly2, baseHandlers, collectionHandlers, proxyMap) {
		if (!isObject(target)) return target;
		if (target["__v_raw"] && !(isReadonly2 && target["__v_isReactive"])) return target;
		if (target["__v_skip"] || !Object.isExtensible(target)) return target;
		const existingProxy = proxyMap.get(target);
		if (existingProxy) return existingProxy;
		const targetType = targetTypeMap(toRawType(target));
		if (targetType === 0) return target;
		const proxy = new Proxy(target, targetType === 2 ? collectionHandlers : baseHandlers);
		proxyMap.set(target, proxy);
		return proxy;
	}
	// @__NO_SIDE_EFFECTS__
	function isReactive(value) {
		if (/* @__PURE__ */ isReadonly(value)) return /* @__PURE__ */ isReactive(value["__v_raw"]);
		return !!(value && value["__v_isReactive"]);
	}
	// @__NO_SIDE_EFFECTS__
	function isReadonly(value) {
		return !!(value && value["__v_isReadonly"]);
	}
	// @__NO_SIDE_EFFECTS__
	function isShallow(value) {
		return !!(value && value["__v_isShallow"]);
	}
	// @__NO_SIDE_EFFECTS__
	function isProxy(value) {
		return value ? !!value["__v_raw"] : false;
	}
	// @__NO_SIDE_EFFECTS__
	function toRaw(observed) {
		const raw = observed && observed["__v_raw"];
		return raw ? /* @__PURE__ */ toRaw(raw) : observed;
	}
	function markRaw(value) {
		if (!hasOwn(value, "__v_skip") && Object.isExtensible(value)) def(value, "__v_skip", true);
		return value;
	}
	var toReactive = (value) => isObject(value) ? /* @__PURE__ */ reactive(value) : value;
	var toReadonly = (value) => isObject(value) ? /* @__PURE__ */ readonly(value) : value;
	// @__NO_SIDE_EFFECTS__
	function isRef(r) {
		return r ? r["__v_isRef"] === true : false;
	}
	// @__NO_SIDE_EFFECTS__
	function ref(value) {
		return createRef(value, false);
	}
	function createRef(rawValue, shallow) {
		if (/* @__PURE__ */ isRef(rawValue)) return rawValue;
		return new RefImpl(rawValue, shallow);
	}
	var RefImpl = class {
		constructor(value, isShallow2) {
			this.dep = new Dep();
			this["__v_isRef"] = true;
			this["__v_isShallow"] = false;
			this._rawValue = isShallow2 ? value : /* @__PURE__ */ toRaw(value);
			this._value = isShallow2 ? value : toReactive(value);
			this["__v_isShallow"] = isShallow2;
		}
		get value() {
			this.dep.track();
			return this._value;
		}
		set value(newValue) {
			const oldValue = this._rawValue;
			const useDirectValue = this["__v_isShallow"] || /* @__PURE__ */ isShallow(newValue) || /* @__PURE__ */ isReadonly(newValue);
			newValue = useDirectValue ? newValue : /* @__PURE__ */ toRaw(newValue);
			if (hasChanged(newValue, oldValue)) {
				this._rawValue = newValue;
				this._value = useDirectValue ? newValue : toReactive(newValue);
				this.dep.trigger();
			}
		}
	};
	function unref(ref2) {
		return /* @__PURE__ */ isRef(ref2) ? ref2.value : ref2;
	}
	var shallowUnwrapHandlers = {
		get: (target, key, receiver) => key === "__v_raw" ? target : unref(Reflect.get(target, key, receiver)),
		set: (target, key, value, receiver) => {
			const oldValue = target[key];
			if (/* @__PURE__ */ isRef(oldValue) && !/* @__PURE__ */ isRef(value)) {
				oldValue.value = value;
				return true;
			} else return Reflect.set(target, key, value, receiver);
		}
	};
	function proxyRefs(objectWithRefs) {
		return /* @__PURE__ */ isReactive(objectWithRefs) ? objectWithRefs : new Proxy(objectWithRefs, shallowUnwrapHandlers);
	}
	var ComputedRefImpl = class {
		constructor(fn, setter, isSSR) {
			this.fn = fn;
			this.setter = setter;
			/**
			* @internal
			*/
			this._value = void 0;
			/**
			* @internal
			*/
			this.dep = new Dep(this);
			/**
			* @internal
			*/
			this.__v_isRef = true;
			/**
			* @internal
			*/
			this.deps = void 0;
			/**
			* @internal
			*/
			this.depsTail = void 0;
			/**
			* @internal
			*/
			this.flags = 16;
			/**
			* @internal
			*/
			this.globalVersion = globalVersion - 1;
			/**
			* @internal
			*/
			this.next = void 0;
			this.effect = this;
			this["__v_isReadonly"] = !setter;
			this.isSSR = isSSR;
		}
		/**
		* @internal
		*/
		notify() {
			this.flags |= 16;
			if (!(this.flags & 8) && activeSub !== this) {
				batch(this, true);
				return true;
			}
		}
		get value() {
			const link = this.dep.track();
			refreshComputed(this);
			if (link) link.version = this.dep.version;
			return this._value;
		}
		set value(newValue) {
			if (this.setter) this.setter(newValue);
		}
	};
	// @__NO_SIDE_EFFECTS__
	function computed$1(getterOrOptions, debugOptions, isSSR = false) {
		let getter;
		let setter;
		if (isFunction(getterOrOptions)) getter = getterOrOptions;
		else {
			getter = getterOrOptions.get;
			setter = getterOrOptions.set;
		}
		return new ComputedRefImpl(getter, setter, isSSR);
	}
	var INITIAL_WATCHER_VALUE = {};
	var cleanupMap = /* @__PURE__ */ new WeakMap();
	var activeWatcher = void 0;
	function onWatcherCleanup(cleanupFn, failSilently = false, owner = activeWatcher) {
		if (owner) {
			let cleanups = cleanupMap.get(owner);
			if (!cleanups) cleanupMap.set(owner, cleanups = []);
			cleanups.push(cleanupFn);
		}
	}
	function watch$1(source, cb, options = EMPTY_OBJ) {
		const { immediate, deep, once, scheduler, augmentJob, call } = options;
		const reactiveGetter = (source2) => {
			if (deep) return source2;
			if (/* @__PURE__ */ isShallow(source2) || deep === false || deep === 0) return traverse(source2, 1);
			return traverse(source2);
		};
		let effect;
		let getter;
		let cleanup;
		let boundCleanup;
		let forceTrigger = false;
		let isMultiSource = false;
		if (/* @__PURE__ */ isRef(source)) {
			getter = () => source.value;
			forceTrigger = /* @__PURE__ */ isShallow(source);
		} else if (/* @__PURE__ */ isReactive(source)) {
			getter = () => reactiveGetter(source);
			forceTrigger = true;
		} else if (isArray(source)) {
			isMultiSource = true;
			forceTrigger = source.some((s) => /* @__PURE__ */ isReactive(s) || /* @__PURE__ */ isShallow(s));
			getter = () => source.map((s) => {
				if (/* @__PURE__ */ isRef(s)) return s.value;
				else if (/* @__PURE__ */ isReactive(s)) return reactiveGetter(s);
				else if (isFunction(s)) return call ? call(s, 2) : s();
			});
		} else if (isFunction(source)) if (cb) getter = call ? () => call(source, 2) : source;
		else getter = () => {
			if (cleanup) {
				pauseTracking();
				try {
					cleanup();
				} finally {
					resetTracking();
				}
			}
			const currentEffect = activeWatcher;
			activeWatcher = effect;
			try {
				return call ? call(source, 3, [boundCleanup]) : source(boundCleanup);
			} finally {
				activeWatcher = currentEffect;
			}
		};
		else getter = NOOP;
		if (cb && deep) {
			const baseGetter = getter;
			const depth = deep === true ? Infinity : deep;
			getter = () => traverse(baseGetter(), depth);
		}
		const scope = getCurrentScope();
		const watchHandle = () => {
			effect.stop();
			if (scope && scope.active) remove(scope.effects, effect);
		};
		if (once && cb) {
			const _cb = cb;
			cb = (...args) => {
				const res = _cb(...args);
				watchHandle();
				return res;
			};
		}
		let oldValue = isMultiSource ? new Array(source.length).fill(INITIAL_WATCHER_VALUE) : INITIAL_WATCHER_VALUE;
		const job = (immediateFirstRun) => {
			if (!(effect.flags & 1) || !effect.dirty && !immediateFirstRun) return;
			if (cb) {
				const newValue = effect.run();
				if (immediateFirstRun || deep || forceTrigger || (isMultiSource ? newValue.some((v, i) => hasChanged(v, oldValue[i])) : hasChanged(newValue, oldValue))) {
					if (cleanup) cleanup();
					const currentWatcher = activeWatcher;
					activeWatcher = effect;
					try {
						const args = [
							newValue,
							oldValue === INITIAL_WATCHER_VALUE ? void 0 : isMultiSource && oldValue[0] === INITIAL_WATCHER_VALUE ? [] : oldValue,
							boundCleanup
						];
						oldValue = newValue;
						call ? call(cb, 3, args) : cb(...args);
					} finally {
						activeWatcher = currentWatcher;
					}
				}
			} else effect.run();
		};
		if (augmentJob) augmentJob(job);
		effect = new ReactiveEffect(getter);
		effect.scheduler = scheduler ? () => scheduler(job, false) : job;
		boundCleanup = (fn) => onWatcherCleanup(fn, false, effect);
		cleanup = effect.onStop = () => {
			const cleanups = cleanupMap.get(effect);
			if (cleanups) {
				if (call) call(cleanups, 4);
				else for (const cleanup2 of cleanups) cleanup2();
				cleanupMap.delete(effect);
			}
		};
		if (cb) if (immediate) job(true);
		else oldValue = effect.run();
		else if (scheduler) scheduler(job.bind(null, true), true);
		else effect.run();
		watchHandle.pause = effect.pause.bind(effect);
		watchHandle.resume = effect.resume.bind(effect);
		watchHandle.stop = watchHandle;
		return watchHandle;
	}
	function traverse(value, depth = Infinity, seen) {
		if (depth <= 0 || !isObject(value) || value["__v_skip"]) return value;
		seen = seen || /* @__PURE__ */ new Map();
		if ((seen.get(value) || 0) >= depth) return value;
		seen.set(value, depth);
		depth--;
		if (/* @__PURE__ */ isRef(value)) traverse(value.value, depth, seen);
		else if (isArray(value)) for (let i = 0; i < value.length; i++) traverse(value[i], depth, seen);
		else if (isSet(value) || isMap(value)) value.forEach((v) => {
			traverse(v, depth, seen);
		});
		else if (isPlainObject(value)) {
			for (const key in value) traverse(value[key], depth, seen);
			for (const key of Object.getOwnPropertySymbols(value)) if (Object.prototype.propertyIsEnumerable.call(value, key)) traverse(value[key], depth, seen);
		}
		return value;
	}
	//#endregion
	//#region ../../../node_modules/.pnpm/@vue+runtime-core@3.5.39/node_modules/@vue/runtime-core/dist/runtime-core.esm-bundler.js
	/**
	* @vue/runtime-core v3.5.39
	* (c) 2018-present Yuxi (Evan) You and Vue contributors
	* @license MIT
	**/
	function callWithErrorHandling(fn, instance, type, args) {
		try {
			return args ? fn(...args) : fn();
		} catch (err) {
			handleError(err, instance, type);
		}
	}
	function callWithAsyncErrorHandling(fn, instance, type, args) {
		if (isFunction(fn)) {
			const res = callWithErrorHandling(fn, instance, type, args);
			if (res && isPromise(res)) res.catch((err) => {
				handleError(err, instance, type);
			});
			return res;
		}
		if (isArray(fn)) {
			const values = [];
			for (let i = 0; i < fn.length; i++) values.push(callWithAsyncErrorHandling(fn[i], instance, type, args));
			return values;
		}
	}
	function handleError(err, instance, type, throwInDev = true) {
		const contextVNode = instance ? instance.vnode : null;
		const { errorHandler, throwUnhandledErrorInProduction } = instance && instance.appContext.config || EMPTY_OBJ;
		if (instance) {
			let cur = instance.parent;
			const exposedInstance = instance.proxy;
			const errorInfo = `https://vuejs.org/error-reference/#runtime-${type}`;
			while (cur) {
				const errorCapturedHooks = cur.ec;
				if (errorCapturedHooks) {
					for (let i = 0; i < errorCapturedHooks.length; i++) if (errorCapturedHooks[i](err, exposedInstance, errorInfo) === false) return;
				}
				cur = cur.parent;
			}
			if (errorHandler) {
				pauseTracking();
				callWithErrorHandling(errorHandler, null, 10, [
					err,
					exposedInstance,
					errorInfo
				]);
				resetTracking();
				return;
			}
		}
		logError(err, type, contextVNode, throwInDev, throwUnhandledErrorInProduction);
	}
	function logError(err, type, contextVNode, throwInDev = true, throwInProd = false) {
		if (throwInProd) throw err;
		else console.error(err);
	}
	var queue = [];
	var flushIndex = -1;
	var pendingPostFlushCbs = [];
	var activePostFlushCbs = null;
	var postFlushIndex = 0;
	var resolvedPromise = /* @__PURE__ */ Promise.resolve();
	var currentFlushPromise = null;
	function nextTick(fn) {
		const p = currentFlushPromise || resolvedPromise;
		return fn ? p.then(this ? fn.bind(this) : fn) : p;
	}
	function findInsertionIndex(id) {
		let start = flushIndex + 1;
		let end = queue.length;
		while (start < end) {
			const middle = start + end >>> 1;
			const middleJob = queue[middle];
			const middleJobId = getId(middleJob);
			if (middleJobId < id || middleJobId === id && middleJob.flags & 2) start = middle + 1;
			else end = middle;
		}
		return start;
	}
	function queueJob(job) {
		if (!(job.flags & 1)) {
			const jobId = getId(job);
			const lastJob = queue[queue.length - 1];
			if (!lastJob || !(job.flags & 2) && jobId >= getId(lastJob)) queue.push(job);
			else queue.splice(findInsertionIndex(jobId), 0, job);
			job.flags |= 1;
			queueFlush();
		}
	}
	function queueFlush() {
		if (!currentFlushPromise) currentFlushPromise = resolvedPromise.then(flushJobs);
	}
	function queuePostFlushCb(cb) {
		if (!isArray(cb)) {
			if (activePostFlushCbs && cb.id === -1) activePostFlushCbs.splice(postFlushIndex + 1, 0, cb);
			else if (!(cb.flags & 1)) {
				pendingPostFlushCbs.push(cb);
				cb.flags |= 1;
			}
		} else pendingPostFlushCbs.push(...cb);
		queueFlush();
	}
	function flushPreFlushCbs(instance, seen, i = flushIndex + 1) {
		for (; i < queue.length; i++) {
			const cb = queue[i];
			if (cb && cb.flags & 2) {
				if (instance && cb.id !== instance.uid) continue;
				queue.splice(i, 1);
				i--;
				if (cb.flags & 4) cb.flags &= -2;
				cb();
				if (!(cb.flags & 4)) cb.flags &= -2;
			}
		}
	}
	function flushPostFlushCbs(seen) {
		if (pendingPostFlushCbs.length) {
			const deduped = [...new Set(pendingPostFlushCbs)].sort((a, b) => getId(a) - getId(b));
			pendingPostFlushCbs.length = 0;
			if (activePostFlushCbs) {
				activePostFlushCbs.push(...deduped);
				return;
			}
			activePostFlushCbs = deduped;
			for (postFlushIndex = 0; postFlushIndex < activePostFlushCbs.length; postFlushIndex++) {
				const cb = activePostFlushCbs[postFlushIndex];
				if (cb.flags & 4) cb.flags &= -2;
				if (!(cb.flags & 8)) cb();
				cb.flags &= -2;
			}
			activePostFlushCbs = null;
			postFlushIndex = 0;
		}
	}
	var getId = (job) => job.id == null ? job.flags & 2 ? -1 : Infinity : job.id;
	function flushJobs(seen) {
		try {
			for (flushIndex = 0; flushIndex < queue.length; flushIndex++) {
				const job = queue[flushIndex];
				if (job && !(job.flags & 8)) {
					if (job.flags & 4) job.flags &= -2;
					callWithErrorHandling(job, job.i, job.i ? 15 : 14);
					if (!(job.flags & 4)) job.flags &= -2;
				}
			}
		} finally {
			for (; flushIndex < queue.length; flushIndex++) {
				const job = queue[flushIndex];
				if (job) job.flags &= -2;
			}
			flushIndex = -1;
			queue.length = 0;
			flushPostFlushCbs(seen);
			currentFlushPromise = null;
			if (queue.length || pendingPostFlushCbs.length) flushJobs(seen);
		}
	}
	var currentRenderingInstance = null;
	var currentScopeId = null;
	function setCurrentRenderingInstance(instance) {
		const prev = currentRenderingInstance;
		currentRenderingInstance = instance;
		currentScopeId = instance && instance.type.__scopeId || null;
		return prev;
	}
	function withCtx(fn, ctx = currentRenderingInstance, isNonScopedSlot) {
		if (!ctx) return fn;
		if (fn._n) return fn;
		const renderFnWithContext = (...args) => {
			if (renderFnWithContext._d) setBlockTracking(-1);
			const prevInstance = setCurrentRenderingInstance(ctx);
			let res;
			try {
				res = fn(...args);
			} finally {
				setCurrentRenderingInstance(prevInstance);
				if (renderFnWithContext._d) setBlockTracking(1);
			}
			return res;
		};
		renderFnWithContext._n = true;
		renderFnWithContext._c = true;
		renderFnWithContext._d = true;
		return renderFnWithContext;
	}
	function withDirectives(vnode, directives) {
		if (currentRenderingInstance === null) return vnode;
		const instance = getComponentPublicInstance(currentRenderingInstance);
		const bindings = vnode.dirs || (vnode.dirs = []);
		for (let i = 0; i < directives.length; i++) {
			let [dir, value, arg, modifiers = EMPTY_OBJ] = directives[i];
			if (dir) {
				if (isFunction(dir)) dir = {
					mounted: dir,
					updated: dir
				};
				if (dir.deep) traverse(value);
				bindings.push({
					dir,
					instance,
					value,
					oldValue: void 0,
					arg,
					modifiers
				});
			}
		}
		return vnode;
	}
	function invokeDirectiveHook(vnode, prevVNode, instance, name) {
		const bindings = vnode.dirs;
		const oldBindings = prevVNode && prevVNode.dirs;
		for (let i = 0; i < bindings.length; i++) {
			const binding = bindings[i];
			if (oldBindings) binding.oldValue = oldBindings[i].value;
			let hook = binding.dir[name];
			if (hook) {
				pauseTracking();
				callWithAsyncErrorHandling(hook, instance, 8, [
					vnode.el,
					binding,
					vnode,
					prevVNode
				]);
				resetTracking();
			}
		}
	}
	function provide(key, value) {
		if (currentInstance) {
			let provides = currentInstance.provides;
			const parentProvides = currentInstance.parent && currentInstance.parent.provides;
			if (parentProvides === provides) provides = currentInstance.provides = Object.create(parentProvides);
			provides[key] = value;
		}
	}
	function inject(key, defaultValue, treatDefaultAsFactory = false) {
		const instance = getCurrentInstance();
		if (instance || currentApp) {
			let provides = currentApp ? currentApp._context.provides : instance ? instance.parent == null || instance.ce ? instance.vnode.appContext && instance.vnode.appContext.provides : instance.parent.provides : void 0;
			if (provides && key in provides) return provides[key];
			else if (arguments.length > 1) return treatDefaultAsFactory && isFunction(defaultValue) ? defaultValue.call(instance && instance.proxy) : defaultValue;
		}
	}
	var ssrContextKey = /* @__PURE__ */ Symbol.for("v-scx");
	var useSSRContext = () => {
		{
			const ctx = inject(ssrContextKey);
			if (!ctx) {}
			return ctx;
		}
	};
	function watch(source, cb, options) {
		return doWatch(source, cb, options);
	}
	function doWatch(source, cb, options = EMPTY_OBJ) {
		const { immediate, deep, flush, once } = options;
		const baseWatchOptions = extend({}, options);
		const runsImmediately = cb && immediate || !cb && flush !== "post";
		let ssrCleanup;
		if (isInSSRComponentSetup) {
			if (flush === "sync") {
				const ctx = useSSRContext();
				ssrCleanup = ctx.__watcherHandles || (ctx.__watcherHandles = []);
			} else if (!runsImmediately) {
				const watchStopHandle = () => {};
				watchStopHandle.stop = NOOP;
				watchStopHandle.resume = NOOP;
				watchStopHandle.pause = NOOP;
				return watchStopHandle;
			}
		}
		const instance = currentInstance;
		baseWatchOptions.call = (fn, type, args) => callWithAsyncErrorHandling(fn, instance, type, args);
		let isPre = false;
		if (flush === "post") baseWatchOptions.scheduler = (job) => {
			queuePostRenderEffect(job, instance && instance.suspense);
		};
		else if (flush !== "sync") {
			isPre = true;
			baseWatchOptions.scheduler = (job, isFirstRun) => {
				if (isFirstRun) job();
				else queueJob(job);
			};
		}
		baseWatchOptions.augmentJob = (job) => {
			if (cb) job.flags |= 4;
			if (isPre) {
				job.flags |= 2;
				if (instance) {
					job.id = instance.uid;
					job.i = instance;
				}
			}
		};
		const watchHandle = watch$1(source, cb, baseWatchOptions);
		if (isInSSRComponentSetup) {
			if (ssrCleanup) ssrCleanup.push(watchHandle);
			else if (runsImmediately) watchHandle();
		}
		return watchHandle;
	}
	function instanceWatch(source, value, options) {
		const publicThis = this.proxy;
		const getter = isString(source) ? source.includes(".") ? createPathGetter(publicThis, source) : () => publicThis[source] : source.bind(publicThis, publicThis);
		let cb;
		if (isFunction(value)) cb = value;
		else {
			cb = value.handler;
			options = value;
		}
		const reset = setCurrentInstance(this);
		const res = doWatch(getter, cb.bind(publicThis), options);
		reset();
		return res;
	}
	function createPathGetter(ctx, path) {
		const segments = path.split(".");
		return () => {
			let cur = ctx;
			for (let i = 0; i < segments.length && cur; i++) cur = cur[segments[i]];
			return cur;
		};
	}
	var TeleportEndKey = /* @__PURE__ */ Symbol("_vte");
	var isTeleport = (type) => type.__isTeleport;
	var leaveCbKey = /* @__PURE__ */ Symbol("_leaveCb");
	function setTransitionHooks(vnode, hooks) {
		if (vnode.shapeFlag & 6 && vnode.component) {
			vnode.transition = hooks;
			setTransitionHooks(vnode.component.subTree, hooks);
		} else if (vnode.shapeFlag & 128) {
			vnode.ssContent.transition = hooks.clone(vnode.ssContent);
			vnode.ssFallback.transition = hooks.clone(vnode.ssFallback);
		} else vnode.transition = hooks;
	}
	// @__NO_SIDE_EFFECTS__
	function defineComponent(options, extraOptions) {
		return isFunction(options) ? /* @__PURE__ */ (() => extend({ name: options.name }, extraOptions, { setup: options }))() : options;
	}
	function markAsyncBoundary(instance) {
		instance.ids = [
			instance.ids[0] + instance.ids[2]++ + "-",
			0,
			0
		];
	}
	function isTemplateRefKey(refs, key) {
		let desc;
		return !!((desc = Object.getOwnPropertyDescriptor(refs, key)) && !desc.configurable);
	}
	var pendingSetRefMap = /* @__PURE__ */ new WeakMap();
	function setRef(rawRef, oldRawRef, parentSuspense, vnode, isUnmount = false) {
		if (isArray(rawRef)) {
			rawRef.forEach((r, i) => setRef(r, oldRawRef && (isArray(oldRawRef) ? oldRawRef[i] : oldRawRef), parentSuspense, vnode, isUnmount));
			return;
		}
		if (isAsyncWrapper(vnode) && !isUnmount) {
			if (vnode.shapeFlag & 512 && vnode.type.__asyncResolved && vnode.component.subTree.component) setRef(rawRef, oldRawRef, parentSuspense, vnode.component.subTree);
			return;
		}
		const refValue = vnode.shapeFlag & 4 ? getComponentPublicInstance(vnode.component) : vnode.el;
		const value = isUnmount ? null : refValue;
		const { i: owner, r: ref } = rawRef;
		const oldRef = oldRawRef && oldRawRef.r;
		const refs = owner.refs === EMPTY_OBJ ? owner.refs = {} : owner.refs;
		const setupState = owner.setupState;
		const rawSetupState = /* @__PURE__ */ toRaw(setupState);
		const canSetSetupRef = setupState === EMPTY_OBJ ? NO : (key) => {
			if (isTemplateRefKey(refs, key)) return false;
			return hasOwn(rawSetupState, key);
		};
		const canSetRef = (ref2, key) => {
			if (key && isTemplateRefKey(refs, key)) return false;
			return true;
		};
		if (oldRef != null && oldRef !== ref) {
			invalidatePendingSetRef(oldRawRef);
			if (isString(oldRef)) {
				refs[oldRef] = null;
				if (canSetSetupRef(oldRef)) setupState[oldRef] = null;
			} else if (/* @__PURE__ */ isRef(oldRef)) {
				const oldRawRefAtom = oldRawRef;
				if (canSetRef(oldRef, oldRawRefAtom.k)) oldRef.value = null;
				if (oldRawRefAtom.k) refs[oldRawRefAtom.k] = null;
			}
		}
		if (isFunction(ref)) {
			pauseTracking();
			try {
				callWithErrorHandling(ref, owner, 12, [value, refs]);
			} finally {
				resetTracking();
			}
		} else {
			const _isString = isString(ref);
			const _isRef = /* @__PURE__ */ isRef(ref);
			if (_isString || _isRef) {
				const doSet = () => {
					if (rawRef.f) {
						const existing = _isString ? canSetSetupRef(ref) ? setupState[ref] : refs[ref] : canSetRef(ref) || !rawRef.k ? ref.value : refs[rawRef.k];
						if (isUnmount) isArray(existing) && remove(existing, refValue);
						else if (!isArray(existing)) if (_isString) {
							refs[ref] = [refValue];
							if (canSetSetupRef(ref)) setupState[ref] = refs[ref];
						} else {
							const newVal = [refValue];
							if (canSetRef(ref, rawRef.k)) ref.value = newVal;
							if (rawRef.k) refs[rawRef.k] = newVal;
						}
						else if (!existing.includes(refValue)) existing.push(refValue);
					} else if (_isString) {
						refs[ref] = value;
						if (canSetSetupRef(ref)) setupState[ref] = value;
					} else if (_isRef) {
						if (canSetRef(ref, rawRef.k)) ref.value = value;
						if (rawRef.k) refs[rawRef.k] = value;
					}
				};
				if (value) {
					const job = () => {
						doSet();
						pendingSetRefMap.delete(rawRef);
					};
					job.id = -1;
					pendingSetRefMap.set(rawRef, job);
					queuePostRenderEffect(job, parentSuspense);
				} else {
					invalidatePendingSetRef(rawRef);
					doSet();
				}
			}
		}
	}
	function invalidatePendingSetRef(rawRef) {
		const pendingSetRef = pendingSetRefMap.get(rawRef);
		if (pendingSetRef) {
			pendingSetRef.flags |= 8;
			pendingSetRefMap.delete(rawRef);
		}
	}
	getGlobalThis().requestIdleCallback;
	getGlobalThis().cancelIdleCallback;
	var isAsyncWrapper = (i) => !!i.type.__asyncLoader;
	var isKeepAlive = (vnode) => vnode.type.__isKeepAlive;
	function onActivated(hook, target) {
		registerKeepAliveHook(hook, "a", target);
	}
	function onDeactivated(hook, target) {
		registerKeepAliveHook(hook, "da", target);
	}
	function registerKeepAliveHook(hook, type, target = currentInstance) {
		const wrappedHook = hook.__wdc || (hook.__wdc = () => {
			let current = target;
			while (current) {
				if (current.isDeactivated) return;
				current = current.parent;
			}
			return hook();
		});
		injectHook(type, wrappedHook, target);
		if (target) {
			let current = target.parent;
			while (current && current.parent) {
				if (isKeepAlive(current.parent.vnode)) injectToKeepAliveRoot(wrappedHook, type, target, current);
				current = current.parent;
			}
		}
	}
	function injectToKeepAliveRoot(hook, type, target, keepAliveRoot) {
		const injected = injectHook(type, hook, keepAliveRoot, true);
		onUnmounted(() => {
			remove(keepAliveRoot[type], injected);
		}, target);
	}
	function injectHook(type, hook, target = currentInstance, prepend = false) {
		if (target) {
			const hooks = target[type] || (target[type] = []);
			const wrappedHook = hook.__weh || (hook.__weh = (...args) => {
				pauseTracking();
				const reset = setCurrentInstance(target);
				const res = callWithAsyncErrorHandling(hook, target, type, args);
				reset();
				resetTracking();
				return res;
			});
			if (prepend) hooks.unshift(wrappedHook);
			else hooks.push(wrappedHook);
			return wrappedHook;
		}
	}
	var createHook = (lifecycle) => (hook, target = currentInstance) => {
		if (!isInSSRComponentSetup || lifecycle === "sp") injectHook(lifecycle, (...args) => hook(...args), target);
	};
	var onBeforeMount = createHook("bm");
	var onMounted = createHook("m");
	var onBeforeUpdate = createHook("bu");
	var onUpdated = createHook("u");
	var onBeforeUnmount = createHook("bum");
	var onUnmounted = createHook("um");
	var onServerPrefetch = createHook("sp");
	var onRenderTriggered = createHook("rtg");
	var onRenderTracked = createHook("rtc");
	function onErrorCaptured(hook, target = currentInstance) {
		injectHook("ec", hook, target);
	}
	var NULL_DYNAMIC_COMPONENT = /* @__PURE__ */ Symbol.for("v-ndc");
	function renderList(source, renderItem, cache, index) {
		let ret;
		const cached = cache && cache[index];
		const sourceIsArray = isArray(source);
		if (sourceIsArray || isString(source)) {
			const sourceIsReactiveArray = sourceIsArray && /* @__PURE__ */ isReactive(source);
			let needsWrap = false;
			let isReadonlySource = false;
			if (sourceIsReactiveArray) {
				needsWrap = !/* @__PURE__ */ isShallow(source);
				isReadonlySource = /* @__PURE__ */ isReadonly(source);
				source = shallowReadArray(source);
			}
			ret = new Array(source.length);
			for (let i = 0, l = source.length; i < l; i++) ret[i] = renderItem(needsWrap ? isReadonlySource ? toReadonly(toReactive(source[i])) : toReactive(source[i]) : source[i], i, void 0, cached && cached[i]);
		} else if (typeof source === "number") {
			ret = new Array(source);
			for (let i = 0; i < source; i++) ret[i] = renderItem(i + 1, i, void 0, cached && cached[i]);
		} else if (isObject(source)) if (source[Symbol.iterator]) ret = Array.from(source, (item, i) => renderItem(item, i, void 0, cached && cached[i]));
		else {
			const keys = Object.keys(source);
			ret = new Array(keys.length);
			for (let i = 0, l = keys.length; i < l; i++) {
				const key = keys[i];
				ret[i] = renderItem(source[key], key, i, cached && cached[i]);
			}
		}
		else ret = [];
		if (cache) cache[index] = ret;
		return ret;
	}
	function renderSlot(slots, name, props = {}, fallback, noSlotted) {
		if (currentRenderingInstance.ce || currentRenderingInstance.parent && isAsyncWrapper(currentRenderingInstance.parent) && currentRenderingInstance.parent.ce) {
			const hasProps = Object.keys(props).length > 0;
			if (name !== "default") props.name = name;
			return openBlock(), createBlock(Fragment, null, [createVNode("slot", props, fallback && fallback())], hasProps ? -2 : 64);
		}
		let slot = slots[name];
		if (slot && slot._c) slot._d = false;
		openBlock();
		const validSlotContent = slot && ensureValidVNode(slot(props));
		const slotKey = props.key || validSlotContent && validSlotContent.key;
		const rendered = createBlock(Fragment, { key: (slotKey && !isSymbol(slotKey) ? slotKey : `_${name}`) + (!validSlotContent && fallback ? "_fb" : "") }, validSlotContent || (fallback ? fallback() : []), validSlotContent && slots._ === 1 ? 64 : -2);
		if (!noSlotted && rendered.scopeId) rendered.slotScopeIds = [rendered.scopeId + "-s"];
		if (slot && slot._c) slot._d = true;
		return rendered;
	}
	function ensureValidVNode(vnodes) {
		return vnodes.some((child) => {
			if (!isVNode(child)) return true;
			if (child.type === Comment) return false;
			if (child.type === Fragment && !ensureValidVNode(child.children)) return false;
			return true;
		}) ? vnodes : null;
	}
	var getPublicInstance = (i) => {
		if (!i) return null;
		if (isStatefulComponent(i)) return getComponentPublicInstance(i);
		return getPublicInstance(i.parent);
	};
	var publicPropertiesMap = /* @__PURE__ */ extend(/* @__PURE__ */ Object.create(null), {
		$: (i) => i,
		$el: (i) => i.vnode.el,
		$data: (i) => i.data,
		$props: (i) => i.props,
		$attrs: (i) => i.attrs,
		$slots: (i) => i.slots,
		$refs: (i) => i.refs,
		$parent: (i) => getPublicInstance(i.parent),
		$root: (i) => getPublicInstance(i.root),
		$host: (i) => i.ce,
		$emit: (i) => i.emit,
		$options: (i) => resolveMergedOptions(i),
		$forceUpdate: (i) => i.f || (i.f = () => {
			queueJob(i.update);
		}),
		$nextTick: (i) => i.n || (i.n = nextTick.bind(i.proxy)),
		$watch: (i) => instanceWatch.bind(i)
	});
	var hasSetupBinding = (state, key) => state !== EMPTY_OBJ && !state.__isScriptSetup && hasOwn(state, key);
	var PublicInstanceProxyHandlers = {
		get({ _: instance }, key) {
			if (key === "__v_skip") return true;
			const { ctx, setupState, data, props, accessCache, type, appContext } = instance;
			if (key[0] !== "$") {
				const n = accessCache[key];
				if (n !== void 0) switch (n) {
					case 1: return setupState[key];
					case 2: return data[key];
					case 4: return ctx[key];
					case 3: return props[key];
				}
				else if (hasSetupBinding(setupState, key)) {
					accessCache[key] = 1;
					return setupState[key];
				} else if (data !== EMPTY_OBJ && hasOwn(data, key)) {
					accessCache[key] = 2;
					return data[key];
				} else if (hasOwn(props, key)) {
					accessCache[key] = 3;
					return props[key];
				} else if (ctx !== EMPTY_OBJ && hasOwn(ctx, key)) {
					accessCache[key] = 4;
					return ctx[key];
				} else if (shouldCacheAccess) accessCache[key] = 0;
			}
			const publicGetter = publicPropertiesMap[key];
			let cssModule, globalProperties;
			if (publicGetter) {
				if (key === "$attrs") track(instance.attrs, "get", "");
				return publicGetter(instance);
			} else if ((cssModule = type.__cssModules) && (cssModule = cssModule[key])) return cssModule;
			else if (ctx !== EMPTY_OBJ && hasOwn(ctx, key)) {
				accessCache[key] = 4;
				return ctx[key];
			} else if (globalProperties = appContext.config.globalProperties, hasOwn(globalProperties, key)) return globalProperties[key];
		},
		set({ _: instance }, key, value) {
			const { data, setupState, ctx } = instance;
			if (hasSetupBinding(setupState, key)) {
				setupState[key] = value;
				return true;
			} else if (data !== EMPTY_OBJ && hasOwn(data, key)) {
				data[key] = value;
				return true;
			} else if (hasOwn(instance.props, key)) return false;
			if (key[0] === "$" && key.slice(1) in instance) return false;
			else ctx[key] = value;
			return true;
		},
		has({ _: { data, setupState, accessCache, ctx, appContext, props, type } }, key) {
			let cssModules;
			return !!(accessCache[key] || data !== EMPTY_OBJ && key[0] !== "$" && hasOwn(data, key) || hasSetupBinding(setupState, key) || hasOwn(props, key) || hasOwn(ctx, key) || hasOwn(publicPropertiesMap, key) || hasOwn(appContext.config.globalProperties, key) || (cssModules = type.__cssModules) && cssModules[key]);
		},
		defineProperty(target, key, descriptor) {
			if (descriptor.get != null) target._.accessCache[key] = 0;
			else if (hasOwn(descriptor, "value")) this.set(target, key, descriptor.value, null);
			return Reflect.defineProperty(target, key, descriptor);
		}
	};
	function normalizePropsOrEmits(props) {
		return isArray(props) ? props.reduce((normalized, p) => (normalized[p] = null, normalized), {}) : props;
	}
	var shouldCacheAccess = true;
	function applyOptions(instance) {
		const options = resolveMergedOptions(instance);
		const publicThis = instance.proxy;
		const ctx = instance.ctx;
		shouldCacheAccess = false;
		if (options.beforeCreate) callHook(options.beforeCreate, instance, "bc");
		const { data: dataOptions, computed: computedOptions, methods, watch: watchOptions, provide: provideOptions, inject: injectOptions, created, beforeMount, mounted, beforeUpdate, updated, activated, deactivated, beforeDestroy, beforeUnmount, destroyed, unmounted, render, renderTracked, renderTriggered, errorCaptured, serverPrefetch, expose, inheritAttrs, components, directives, filters } = options;
		const checkDuplicateProperties = null;
		if (injectOptions) resolveInjections(injectOptions, ctx, checkDuplicateProperties);
		if (methods) for (const key in methods) {
			const methodHandler = methods[key];
			if (isFunction(methodHandler)) ctx[key] = methodHandler.bind(publicThis);
		}
		if (dataOptions) {
			const data = dataOptions.call(publicThis, publicThis);
			if (!isObject(data)) {} else instance.data = /* @__PURE__ */ reactive(data);
		}
		shouldCacheAccess = true;
		if (computedOptions) for (const key in computedOptions) {
			const opt = computedOptions[key];
			const c = computed({
				get: isFunction(opt) ? opt.bind(publicThis, publicThis) : isFunction(opt.get) ? opt.get.bind(publicThis, publicThis) : NOOP,
				set: !isFunction(opt) && isFunction(opt.set) ? opt.set.bind(publicThis) : NOOP
			});
			Object.defineProperty(ctx, key, {
				enumerable: true,
				configurable: true,
				get: () => c.value,
				set: (v) => c.value = v
			});
		}
		if (watchOptions) for (const key in watchOptions) createWatcher(watchOptions[key], ctx, publicThis, key);
		if (provideOptions) {
			const provides = isFunction(provideOptions) ? provideOptions.call(publicThis) : provideOptions;
			Reflect.ownKeys(provides).forEach((key) => {
				provide(key, provides[key]);
			});
		}
		if (created) callHook(created, instance, "c");
		function registerLifecycleHook(register, hook) {
			if (isArray(hook)) hook.forEach((_hook) => register(_hook.bind(publicThis)));
			else if (hook) register(hook.bind(publicThis));
		}
		registerLifecycleHook(onBeforeMount, beforeMount);
		registerLifecycleHook(onMounted, mounted);
		registerLifecycleHook(onBeforeUpdate, beforeUpdate);
		registerLifecycleHook(onUpdated, updated);
		registerLifecycleHook(onActivated, activated);
		registerLifecycleHook(onDeactivated, deactivated);
		registerLifecycleHook(onErrorCaptured, errorCaptured);
		registerLifecycleHook(onRenderTracked, renderTracked);
		registerLifecycleHook(onRenderTriggered, renderTriggered);
		registerLifecycleHook(onBeforeUnmount, beforeUnmount);
		registerLifecycleHook(onUnmounted, unmounted);
		registerLifecycleHook(onServerPrefetch, serverPrefetch);
		if (isArray(expose)) {
			if (expose.length) {
				const exposed = instance.exposed || (instance.exposed = {});
				expose.forEach((key) => {
					Object.defineProperty(exposed, key, {
						get: () => publicThis[key],
						set: (val) => publicThis[key] = val,
						enumerable: true
					});
				});
			} else if (!instance.exposed) instance.exposed = {};
		}
		if (render && instance.render === NOOP) instance.render = render;
		if (inheritAttrs != null) instance.inheritAttrs = inheritAttrs;
		if (components) instance.components = components;
		if (directives) instance.directives = directives;
		if (serverPrefetch) markAsyncBoundary(instance);
	}
	function resolveInjections(injectOptions, ctx, checkDuplicateProperties = NOOP) {
		if (isArray(injectOptions)) injectOptions = normalizeInject(injectOptions);
		for (const key in injectOptions) {
			const opt = injectOptions[key];
			let injected;
			if (isObject(opt)) if ("default" in opt) injected = inject(opt.from || key, opt.default, true);
			else injected = inject(opt.from || key);
			else injected = inject(opt);
			if (/* @__PURE__ */ isRef(injected)) Object.defineProperty(ctx, key, {
				enumerable: true,
				configurable: true,
				get: () => injected.value,
				set: (v) => injected.value = v
			});
			else ctx[key] = injected;
		}
	}
	function callHook(hook, instance, type) {
		callWithAsyncErrorHandling(isArray(hook) ? hook.map((h) => h.bind(instance.proxy)) : hook.bind(instance.proxy), instance, type);
	}
	function createWatcher(raw, ctx, publicThis, key) {
		let getter = key.includes(".") ? createPathGetter(publicThis, key) : () => publicThis[key];
		if (isString(raw)) {
			const handler = ctx[raw];
			if (isFunction(handler)) watch(getter, handler);
		} else if (isFunction(raw)) watch(getter, raw.bind(publicThis));
		else if (isObject(raw)) if (isArray(raw)) raw.forEach((r) => createWatcher(r, ctx, publicThis, key));
		else {
			const handler = isFunction(raw.handler) ? raw.handler.bind(publicThis) : ctx[raw.handler];
			if (isFunction(handler)) watch(getter, handler, raw);
		}
	}
	function resolveMergedOptions(instance) {
		const base = instance.type;
		const { mixins, extends: extendsOptions } = base;
		const { mixins: globalMixins, optionsCache: cache, config: { optionMergeStrategies } } = instance.appContext;
		const cached = cache.get(base);
		let resolved;
		if (cached) resolved = cached;
		else if (!globalMixins.length && !mixins && !extendsOptions) resolved = base;
		else {
			resolved = {};
			if (globalMixins.length) globalMixins.forEach((m) => mergeOptions(resolved, m, optionMergeStrategies, true));
			mergeOptions(resolved, base, optionMergeStrategies);
		}
		if (isObject(base)) cache.set(base, resolved);
		return resolved;
	}
	function mergeOptions(to, from, strats, asMixin = false) {
		const { mixins, extends: extendsOptions } = from;
		if (extendsOptions) mergeOptions(to, extendsOptions, strats, true);
		if (mixins) mixins.forEach((m) => mergeOptions(to, m, strats, true));
		for (const key in from) if (asMixin && key === "expose") {} else {
			const strat = internalOptionMergeStrats[key] || strats && strats[key];
			to[key] = strat ? strat(to[key], from[key]) : from[key];
		}
		return to;
	}
	var internalOptionMergeStrats = {
		data: mergeDataFn,
		props: mergeEmitsOrPropsOptions,
		emits: mergeEmitsOrPropsOptions,
		methods: mergeObjectOptions,
		computed: mergeObjectOptions,
		beforeCreate: mergeAsArray,
		created: mergeAsArray,
		beforeMount: mergeAsArray,
		mounted: mergeAsArray,
		beforeUpdate: mergeAsArray,
		updated: mergeAsArray,
		beforeDestroy: mergeAsArray,
		beforeUnmount: mergeAsArray,
		destroyed: mergeAsArray,
		unmounted: mergeAsArray,
		activated: mergeAsArray,
		deactivated: mergeAsArray,
		errorCaptured: mergeAsArray,
		serverPrefetch: mergeAsArray,
		components: mergeObjectOptions,
		directives: mergeObjectOptions,
		watch: mergeWatchOptions,
		provide: mergeDataFn,
		inject: mergeInject
	};
	function mergeDataFn(to, from) {
		if (!from) return to;
		if (!to) return from;
		return function mergedDataFn() {
			return extend(isFunction(to) ? to.call(this, this) : to, isFunction(from) ? from.call(this, this) : from);
		};
	}
	function mergeInject(to, from) {
		return mergeObjectOptions(normalizeInject(to), normalizeInject(from));
	}
	function normalizeInject(raw) {
		if (isArray(raw)) {
			const res = {};
			for (let i = 0; i < raw.length; i++) res[raw[i]] = raw[i];
			return res;
		}
		return raw;
	}
	function mergeAsArray(to, from) {
		return to ? [...new Set([].concat(to, from))] : from;
	}
	function mergeObjectOptions(to, from) {
		return to ? extend(/* @__PURE__ */ Object.create(null), to, from) : from;
	}
	function mergeEmitsOrPropsOptions(to, from) {
		if (to) {
			if (isArray(to) && isArray(from)) return [.../* @__PURE__ */ new Set([...to, ...from])];
			return extend(/* @__PURE__ */ Object.create(null), normalizePropsOrEmits(to), normalizePropsOrEmits(from != null ? from : {}));
		} else return from;
	}
	function mergeWatchOptions(to, from) {
		if (!to) return from;
		if (!from) return to;
		const merged = extend(/* @__PURE__ */ Object.create(null), to);
		for (const key in from) merged[key] = mergeAsArray(to[key], from[key]);
		return merged;
	}
	function createAppContext() {
		return {
			app: null,
			config: {
				isNativeTag: NO,
				performance: false,
				globalProperties: {},
				optionMergeStrategies: {},
				errorHandler: void 0,
				warnHandler: void 0,
				compilerOptions: {}
			},
			mixins: [],
			components: {},
			directives: {},
			provides: /* @__PURE__ */ Object.create(null),
			optionsCache: /* @__PURE__ */ new WeakMap(),
			propsCache: /* @__PURE__ */ new WeakMap(),
			emitsCache: /* @__PURE__ */ new WeakMap()
		};
	}
	var uid$1 = 0;
	function createAppAPI(render, hydrate) {
		return function createApp(rootComponent, rootProps = null) {
			if (!isFunction(rootComponent)) rootComponent = extend({}, rootComponent);
			if (rootProps != null && !isObject(rootProps)) rootProps = null;
			const context = createAppContext();
			const installedPlugins = /* @__PURE__ */ new WeakSet();
			const pluginCleanupFns = [];
			let isMounted = false;
			const app = context.app = {
				_uid: uid$1++,
				_component: rootComponent,
				_props: rootProps,
				_container: null,
				_context: context,
				_instance: null,
				version,
				get config() {
					return context.config;
				},
				set config(v) {},
				use(plugin, ...options) {
					if (installedPlugins.has(plugin)) {} else if (plugin && isFunction(plugin.install)) {
						installedPlugins.add(plugin);
						plugin.install(app, ...options);
					} else if (isFunction(plugin)) {
						installedPlugins.add(plugin);
						plugin(app, ...options);
					}
					return app;
				},
				mixin(mixin) {
					if (!context.mixins.includes(mixin)) context.mixins.push(mixin);
					return app;
				},
				component(name, component) {
					if (!component) return context.components[name];
					context.components[name] = component;
					return app;
				},
				directive(name, directive) {
					if (!directive) return context.directives[name];
					context.directives[name] = directive;
					return app;
				},
				mount(rootContainer, isHydrate, namespace) {
					if (!isMounted) {
						const vnode = app._ceVNode || createVNode(rootComponent, rootProps);
						vnode.appContext = context;
						if (namespace === true) namespace = "svg";
						else if (namespace === false) namespace = void 0;
						if (isHydrate && hydrate) hydrate(vnode, rootContainer);
						else render(vnode, rootContainer, namespace);
						isMounted = true;
						app._container = rootContainer;
						rootContainer.__vue_app__ = app;
						return getComponentPublicInstance(vnode.component);
					}
				},
				onUnmount(cleanupFn) {
					pluginCleanupFns.push(cleanupFn);
				},
				unmount() {
					if (isMounted) {
						callWithAsyncErrorHandling(pluginCleanupFns, app._instance, 16);
						render(null, app._container);
						delete app._container.__vue_app__;
					}
				},
				provide(key, value) {
					context.provides[key] = value;
					return app;
				},
				runWithContext(fn) {
					const lastApp = currentApp;
					currentApp = app;
					try {
						return fn();
					} finally {
						currentApp = lastApp;
					}
				}
			};
			return app;
		};
	}
	var currentApp = null;
	var getModelModifiers = (props, modelName) => {
		return modelName === "modelValue" || modelName === "model-value" ? props.modelModifiers : props[`${modelName}Modifiers`] || props[`${camelize(modelName)}Modifiers`] || props[`${hyphenate(modelName)}Modifiers`];
	};
	function emit(instance, event, ...rawArgs) {
		if (instance.isUnmounted) return;
		const props = instance.vnode.props || EMPTY_OBJ;
		let args = rawArgs;
		const isModelListener = event.startsWith("update:");
		const modifiers = isModelListener && getModelModifiers(props, event.slice(7));
		if (modifiers) {
			if (modifiers.trim) args = rawArgs.map((a) => isString(a) ? a.trim() : a);
			if (modifiers.number) args = rawArgs.map(looseToNumber);
		}
		let handlerName;
		let handler = props[handlerName = toHandlerKey(event)] || props[handlerName = toHandlerKey(camelize(event))];
		if (!handler && isModelListener) handler = props[handlerName = toHandlerKey(hyphenate(event))];
		if (handler) callWithAsyncErrorHandling(handler, instance, 6, args);
		const onceHandler = props[handlerName + `Once`];
		if (onceHandler) {
			if (!instance.emitted) instance.emitted = {};
			else if (instance.emitted[handlerName]) return;
			instance.emitted[handlerName] = true;
			callWithAsyncErrorHandling(onceHandler, instance, 6, args);
		}
	}
	var mixinEmitsCache = /* @__PURE__ */ new WeakMap();
	function normalizeEmitsOptions(comp, appContext, asMixin = false) {
		const cache = asMixin ? mixinEmitsCache : appContext.emitsCache;
		const cached = cache.get(comp);
		if (cached !== void 0) return cached;
		const raw = comp.emits;
		let normalized = {};
		let hasExtends = false;
		if (!isFunction(comp)) {
			const extendEmits = (raw2) => {
				const normalizedFromExtend = normalizeEmitsOptions(raw2, appContext, true);
				if (normalizedFromExtend) {
					hasExtends = true;
					extend(normalized, normalizedFromExtend);
				}
			};
			if (!asMixin && appContext.mixins.length) appContext.mixins.forEach(extendEmits);
			if (comp.extends) extendEmits(comp.extends);
			if (comp.mixins) comp.mixins.forEach(extendEmits);
		}
		if (!raw && !hasExtends) {
			if (isObject(comp)) cache.set(comp, null);
			return null;
		}
		if (isArray(raw)) raw.forEach((key) => normalized[key] = null);
		else extend(normalized, raw);
		if (isObject(comp)) cache.set(comp, normalized);
		return normalized;
	}
	function isEmitListener(options, key) {
		if (!options || !isOn(key)) return false;
		key = key.slice(2);
		key = key === "Once" ? key : key.replace(/Once$/, "");
		return hasOwn(options, key[0].toLowerCase() + key.slice(1)) || hasOwn(options, hyphenate(key)) || hasOwn(options, key);
	}
	function renderComponentRoot(instance) {
		const { type: Component, vnode, proxy, withProxy, propsOptions: [propsOptions], slots, attrs, emit, render, renderCache, props, data, setupState, ctx, inheritAttrs } = instance;
		const prev = setCurrentRenderingInstance(instance);
		let result;
		let fallthroughAttrs;
		try {
			if (vnode.shapeFlag & 4) {
				const proxyToUse = withProxy || proxy;
				const thisProxy = proxyToUse;
				result = normalizeVNode(render.call(thisProxy, proxyToUse, renderCache, props, setupState, data, ctx));
				fallthroughAttrs = attrs;
			} else {
				const render2 = Component;
				result = normalizeVNode(render2.length > 1 ? render2(props, {
					attrs,
					slots,
					emit
				}) : render2(props, null));
				fallthroughAttrs = Component.props ? attrs : getFunctionalFallthrough(attrs);
			}
		} catch (err) {
			blockStack.length = 0;
			handleError(err, instance, 1);
			result = createVNode(Comment);
		}
		let root = result;
		if (fallthroughAttrs && inheritAttrs !== false) {
			const keys = Object.keys(fallthroughAttrs);
			const { shapeFlag } = root;
			if (keys.length) {
				if (shapeFlag & 7) {
					if (propsOptions && keys.some(isModelListener)) fallthroughAttrs = filterModelListeners(fallthroughAttrs, propsOptions);
					root = cloneVNode(root, fallthroughAttrs, false, true);
				}
			}
		}
		if (vnode.dirs) {
			root = cloneVNode(root, null, false, true);
			root.dirs = root.dirs ? root.dirs.concat(vnode.dirs) : vnode.dirs;
		}
		if (vnode.transition) setTransitionHooks(root, vnode.transition);
		result = root;
		setCurrentRenderingInstance(prev);
		return result;
	}
	var getFunctionalFallthrough = (attrs) => {
		let res;
		for (const key in attrs) if (key === "class" || key === "style" || isOn(key)) (res || (res = {}))[key] = attrs[key];
		return res;
	};
	var filterModelListeners = (attrs, props) => {
		const res = {};
		for (const key in attrs) if (!isModelListener(key) || !(key.slice(9) in props)) res[key] = attrs[key];
		return res;
	};
	function shouldUpdateComponent(prevVNode, nextVNode, optimized) {
		const { props: prevProps, children: prevChildren, component } = prevVNode;
		const { props: nextProps, children: nextChildren, patchFlag } = nextVNode;
		const emits = component.emitsOptions;
		if (nextVNode.dirs || nextVNode.transition) return true;
		if (optimized && patchFlag >= 0) {
			if (patchFlag & 1024) return true;
			if (patchFlag & 16) {
				if (!prevProps) return !!nextProps;
				return hasPropsChanged(prevProps, nextProps, emits);
			} else if (patchFlag & 8) {
				const dynamicProps = nextVNode.dynamicProps;
				for (let i = 0; i < dynamicProps.length; i++) {
					const key = dynamicProps[i];
					if (hasPropValueChanged(nextProps, prevProps, key) && !isEmitListener(emits, key)) return true;
				}
			}
		} else {
			if (prevChildren || nextChildren) {
				if (!nextChildren || !nextChildren.$stable) return true;
			}
			if (prevProps === nextProps) return false;
			if (!prevProps) return !!nextProps;
			if (!nextProps) return true;
			return hasPropsChanged(prevProps, nextProps, emits);
		}
		return false;
	}
	function hasPropsChanged(prevProps, nextProps, emitsOptions) {
		const nextKeys = Object.keys(nextProps);
		if (nextKeys.length !== Object.keys(prevProps).length) return true;
		for (let i = 0; i < nextKeys.length; i++) {
			const key = nextKeys[i];
			if (hasPropValueChanged(nextProps, prevProps, key) && !isEmitListener(emitsOptions, key)) return true;
		}
		return false;
	}
	function hasPropValueChanged(nextProps, prevProps, key) {
		const nextProp = nextProps[key];
		const prevProp = prevProps[key];
		if (key === "style" && isObject(nextProp) && isObject(prevProp)) return !looseEqual(nextProp, prevProp);
		return nextProp !== prevProp;
	}
	function updateHOCHostEl({ vnode, parent, suspense }, el) {
		while (parent) {
			const root = parent.subTree;
			if (root.suspense && root.suspense.activeBranch === vnode) {
				root.suspense.vnode.el = root.el = el;
				vnode = root;
			}
			if (root === vnode) {
				(vnode = parent.vnode).el = el;
				parent = parent.parent;
			} else break;
		}
		if (suspense && suspense.activeBranch === vnode) suspense.vnode.el = el;
	}
	var internalObjectProto = {};
	var createInternalObject = () => Object.create(internalObjectProto);
	var isInternalObject = (obj) => Object.getPrototypeOf(obj) === internalObjectProto;
	function initProps(instance, rawProps, isStateful, isSSR = false) {
		const props = {};
		const attrs = createInternalObject();
		instance.propsDefaults = /* @__PURE__ */ Object.create(null);
		setFullProps(instance, rawProps, props, attrs);
		for (const key in instance.propsOptions[0]) if (!(key in props)) props[key] = void 0;
		if (isStateful) instance.props = isSSR ? props : /* @__PURE__ */ shallowReactive(props);
		else if (!instance.type.props) instance.props = attrs;
		else instance.props = props;
		instance.attrs = attrs;
	}
	function updateProps(instance, rawProps, rawPrevProps, optimized) {
		const { props, attrs, vnode: { patchFlag } } = instance;
		const rawCurrentProps = /* @__PURE__ */ toRaw(props);
		const [options] = instance.propsOptions;
		let hasAttrsChanged = false;
		if ((optimized || patchFlag > 0) && !(patchFlag & 16)) {
			if (patchFlag & 8) {
				const propsToUpdate = instance.vnode.dynamicProps;
				for (let i = 0; i < propsToUpdate.length; i++) {
					let key = propsToUpdate[i];
					if (isEmitListener(instance.emitsOptions, key)) continue;
					const value = rawProps[key];
					if (options) if (hasOwn(attrs, key)) {
						if (value !== attrs[key]) {
							attrs[key] = value;
							hasAttrsChanged = true;
						}
					} else {
						const camelizedKey = camelize(key);
						props[camelizedKey] = resolvePropValue(options, rawCurrentProps, camelizedKey, value, instance, false);
					}
					else if (value !== attrs[key]) {
						attrs[key] = value;
						hasAttrsChanged = true;
					}
				}
			}
		} else {
			if (setFullProps(instance, rawProps, props, attrs)) hasAttrsChanged = true;
			let kebabKey;
			for (const key in rawCurrentProps) if (!rawProps || !hasOwn(rawProps, key) && ((kebabKey = hyphenate(key)) === key || !hasOwn(rawProps, kebabKey))) if (options) {
				if (rawPrevProps && (rawPrevProps[key] !== void 0 || rawPrevProps[kebabKey] !== void 0)) props[key] = resolvePropValue(options, rawCurrentProps, key, void 0, instance, true);
			} else delete props[key];
			if (attrs !== rawCurrentProps) {
				for (const key in attrs) if (!rawProps || !hasOwn(rawProps, key) && true) {
					delete attrs[key];
					hasAttrsChanged = true;
				}
			}
		}
		if (hasAttrsChanged) trigger(instance.attrs, "set", "");
	}
	function setFullProps(instance, rawProps, props, attrs) {
		const [options, needCastKeys] = instance.propsOptions;
		let hasAttrsChanged = false;
		let rawCastValues;
		if (rawProps) for (let key in rawProps) {
			if (isReservedProp(key)) continue;
			const value = rawProps[key];
			let camelKey;
			if (options && hasOwn(options, camelKey = camelize(key))) if (!needCastKeys || !needCastKeys.includes(camelKey)) props[camelKey] = value;
			else (rawCastValues || (rawCastValues = {}))[camelKey] = value;
			else if (!isEmitListener(instance.emitsOptions, key)) {
				if (!(key in attrs) || value !== attrs[key]) {
					attrs[key] = value;
					hasAttrsChanged = true;
				}
			}
		}
		if (needCastKeys) {
			const rawCurrentProps = /* @__PURE__ */ toRaw(props);
			const castValues = rawCastValues || EMPTY_OBJ;
			for (let i = 0; i < needCastKeys.length; i++) {
				const key = needCastKeys[i];
				props[key] = resolvePropValue(options, rawCurrentProps, key, castValues[key], instance, !hasOwn(castValues, key));
			}
		}
		return hasAttrsChanged;
	}
	function resolvePropValue(options, props, key, value, instance, isAbsent) {
		const opt = options[key];
		if (opt != null) {
			const hasDefault = hasOwn(opt, "default");
			if (hasDefault && value === void 0) {
				const defaultValue = opt.default;
				if (opt.type !== Function && !opt.skipFactory && isFunction(defaultValue)) {
					const { propsDefaults } = instance;
					if (key in propsDefaults) value = propsDefaults[key];
					else {
						const reset = setCurrentInstance(instance);
						value = propsDefaults[key] = defaultValue.call(null, props);
						reset();
					}
				} else value = defaultValue;
				if (instance.ce) instance.ce._setProp(key, value);
			}
			if (opt[0]) {
				if (isAbsent && !hasDefault) value = false;
				else if (opt[1] && (value === "" || value === hyphenate(key))) value = true;
			}
		}
		return value;
	}
	var mixinPropsCache = /* @__PURE__ */ new WeakMap();
	function normalizePropsOptions(comp, appContext, asMixin = false) {
		const cache = asMixin ? mixinPropsCache : appContext.propsCache;
		const cached = cache.get(comp);
		if (cached) return cached;
		const raw = comp.props;
		const normalized = {};
		const needCastKeys = [];
		let hasExtends = false;
		if (!isFunction(comp)) {
			const extendProps = (raw2) => {
				hasExtends = true;
				const [props, keys] = normalizePropsOptions(raw2, appContext, true);
				extend(normalized, props);
				if (keys) needCastKeys.push(...keys);
			};
			if (!asMixin && appContext.mixins.length) appContext.mixins.forEach(extendProps);
			if (comp.extends) extendProps(comp.extends);
			if (comp.mixins) comp.mixins.forEach(extendProps);
		}
		if (!raw && !hasExtends) {
			if (isObject(comp)) cache.set(comp, EMPTY_ARR);
			return EMPTY_ARR;
		}
		if (isArray(raw)) for (let i = 0; i < raw.length; i++) {
			const normalizedKey = camelize(raw[i]);
			if (validatePropName(normalizedKey)) normalized[normalizedKey] = EMPTY_OBJ;
		}
		else if (raw) for (const key in raw) {
			const normalizedKey = camelize(key);
			if (validatePropName(normalizedKey)) {
				const opt = raw[key];
				const prop = normalized[normalizedKey] = isArray(opt) || isFunction(opt) ? { type: opt } : extend({}, opt);
				const propType = prop.type;
				let shouldCast = false;
				let shouldCastTrue = true;
				if (isArray(propType)) for (let index = 0; index < propType.length; ++index) {
					const type = propType[index];
					const typeName = isFunction(type) && type.name;
					if (typeName === "Boolean") {
						shouldCast = true;
						break;
					} else if (typeName === "String") shouldCastTrue = false;
				}
				else shouldCast = isFunction(propType) && propType.name === "Boolean";
				prop[0] = shouldCast;
				prop[1] = shouldCastTrue;
				if (shouldCast || hasOwn(prop, "default")) needCastKeys.push(normalizedKey);
			}
		}
		const res = [normalized, needCastKeys];
		if (isObject(comp)) cache.set(comp, res);
		return res;
	}
	function validatePropName(key) {
		if (key[0] !== "$" && !isReservedProp(key)) return true;
		return false;
	}
	var isInternalKey = (key) => key === "_" || key === "_ctx" || key === "$stable";
	var normalizeSlotValue = (value) => isArray(value) ? value.map(normalizeVNode) : [normalizeVNode(value)];
	var normalizeSlot = (key, rawSlot, ctx) => {
		if (rawSlot._n) return rawSlot;
		const normalized = withCtx((...args) => {
			return normalizeSlotValue(rawSlot(...args));
		}, ctx);
		normalized._c = false;
		return normalized;
	};
	var normalizeObjectSlots = (rawSlots, slots, instance) => {
		const ctx = rawSlots._ctx;
		for (const key in rawSlots) {
			if (isInternalKey(key)) continue;
			const value = rawSlots[key];
			if (isFunction(value)) slots[key] = normalizeSlot(key, value, ctx);
			else if (value != null) {
				const normalized = normalizeSlotValue(value);
				slots[key] = () => normalized;
			}
		}
	};
	var normalizeVNodeSlots = (instance, children) => {
		const normalized = normalizeSlotValue(children);
		instance.slots.default = () => normalized;
	};
	var assignSlots = (slots, children, optimized) => {
		for (const key in children) if (optimized || !isInternalKey(key)) slots[key] = children[key];
	};
	var initSlots = (instance, children, optimized) => {
		const slots = instance.slots = createInternalObject();
		if (instance.vnode.shapeFlag & 32) {
			const type = children._;
			if (type) {
				assignSlots(slots, children, optimized);
				if (optimized) def(slots, "_", type, true);
			} else normalizeObjectSlots(children, slots);
		} else if (children) normalizeVNodeSlots(instance, children);
	};
	var updateSlots = (instance, children, optimized) => {
		const { vnode, slots } = instance;
		let needDeletionCheck = true;
		let deletionComparisonTarget = EMPTY_OBJ;
		if (vnode.shapeFlag & 32) {
			const type = children._;
			if (type) if (optimized && type === 1) needDeletionCheck = false;
			else assignSlots(slots, children, optimized);
			else {
				needDeletionCheck = !children.$stable;
				normalizeObjectSlots(children, slots);
			}
			deletionComparisonTarget = children;
		} else if (children) {
			normalizeVNodeSlots(instance, children);
			deletionComparisonTarget = { default: 1 };
		}
		if (needDeletionCheck) {
			for (const key in slots) if (!isInternalKey(key) && deletionComparisonTarget[key] == null) delete slots[key];
		}
	};
	var queuePostRenderEffect = queueEffectWithSuspense;
	function createRenderer(options) {
		return baseCreateRenderer(options);
	}
	function baseCreateRenderer(options, createHydrationFns) {
		const target = getGlobalThis();
		target.__VUE__ = true;
		const { insert: hostInsert, remove: hostRemove, patchProp: hostPatchProp, createElement: hostCreateElement, createText: hostCreateText, createComment: hostCreateComment, setText: hostSetText, setElementText: hostSetElementText, parentNode: hostParentNode, nextSibling: hostNextSibling, setScopeId: hostSetScopeId = NOOP, insertStaticContent: hostInsertStaticContent } = options;
		const patch = (n1, n2, container, anchor = null, parentComponent = null, parentSuspense = null, namespace = void 0, slotScopeIds = null, optimized = !!n2.dynamicChildren) => {
			if (n1 === n2) return;
			if (n1 && !isSameVNodeType(n1, n2)) {
				anchor = getNextHostNode(n1);
				unmount(n1, parentComponent, parentSuspense, true);
				n1 = null;
			}
			if (n2.patchFlag === -2) {
				optimized = false;
				n2.dynamicChildren = null;
			}
			const { type, ref, shapeFlag } = n2;
			switch (type) {
				case Text:
					processText(n1, n2, container, anchor);
					break;
				case Comment:
					processCommentNode(n1, n2, container, anchor);
					break;
				case Static:
					if (n1 == null) mountStaticNode(n2, container, anchor, namespace);
					break;
				case Fragment:
					processFragment(n1, n2, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
					break;
				default: if (shapeFlag & 1) processElement(n1, n2, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
				else if (shapeFlag & 6) processComponent(n1, n2, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
				else if (shapeFlag & 64) type.process(n1, n2, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized, internals);
				else if (shapeFlag & 128) type.process(n1, n2, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized, internals);
			}
			if (ref != null && parentComponent) setRef(ref, n1 && n1.ref, parentSuspense, n2 || n1, !n2);
			else if (ref == null && n1 && n1.ref != null) setRef(n1.ref, null, parentSuspense, n1, true);
		};
		const processText = (n1, n2, container, anchor) => {
			if (n1 == null) hostInsert(n2.el = hostCreateText(n2.children), container, anchor);
			else {
				const el = n2.el = n1.el;
				if (n2.children !== n1.children) hostSetText(el, n2.children);
			}
		};
		const processCommentNode = (n1, n2, container, anchor) => {
			if (n1 == null) hostInsert(n2.el = hostCreateComment(n2.children || ""), container, anchor);
			else n2.el = n1.el;
		};
		const mountStaticNode = (n2, container, anchor, namespace) => {
			[n2.el, n2.anchor] = hostInsertStaticContent(n2.children, container, anchor, namespace, n2.el, n2.anchor);
		};
		const moveStaticNode = ({ el, anchor }, container, nextSibling) => {
			let next;
			while (el && el !== anchor) {
				next = hostNextSibling(el);
				hostInsert(el, container, nextSibling);
				el = next;
			}
			hostInsert(anchor, container, nextSibling);
		};
		const removeStaticNode = ({ el, anchor }) => {
			let next;
			while (el && el !== anchor) {
				next = hostNextSibling(el);
				hostRemove(el);
				el = next;
			}
			hostRemove(anchor);
		};
		const processElement = (n1, n2, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized) => {
			if (n2.type === "svg") namespace = "svg";
			else if (n2.type === "math") namespace = "mathml";
			if (n1 == null) mountElement(n2, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
			else {
				const customElement = n1.el && n1.el._isVueCE ? n1.el : null;
				try {
					if (customElement) customElement._beginPatch();
					patchElement(n1, n2, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
				} finally {
					if (customElement) customElement._endPatch();
				}
			}
		};
		const mountElement = (vnode, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized) => {
			let el;
			let vnodeHook;
			const { props, shapeFlag, transition, dirs } = vnode;
			el = vnode.el = hostCreateElement(vnode.type, namespace, props && props.is, props);
			if (shapeFlag & 8) hostSetElementText(el, vnode.children);
			else if (shapeFlag & 16) mountChildren(vnode.children, el, null, parentComponent, parentSuspense, resolveChildrenNamespace(vnode, namespace), slotScopeIds, optimized);
			if (dirs) invokeDirectiveHook(vnode, null, parentComponent, "created");
			setScopeId(el, vnode, vnode.scopeId, slotScopeIds, parentComponent);
			if (props) {
				for (const key in props) if (key !== "value" && !isReservedProp(key)) hostPatchProp(el, key, null, props[key], namespace, parentComponent);
				if ("value" in props) hostPatchProp(el, "value", null, props.value, namespace);
				if (vnodeHook = props.onVnodeBeforeMount) invokeVNodeHook(vnodeHook, parentComponent, vnode);
			}
			if (dirs) invokeDirectiveHook(vnode, null, parentComponent, "beforeMount");
			const needCallTransitionHooks = needTransition(parentSuspense, transition);
			if (needCallTransitionHooks) transition.beforeEnter(el);
			hostInsert(el, container, anchor);
			if ((vnodeHook = props && props.onVnodeMounted) || needCallTransitionHooks || dirs) queuePostRenderEffect(() => {
				try {
					vnodeHook && invokeVNodeHook(vnodeHook, parentComponent, vnode);
					needCallTransitionHooks && transition.enter(el);
					dirs && invokeDirectiveHook(vnode, null, parentComponent, "mounted");
				} finally {}
			}, parentSuspense);
		};
		const setScopeId = (el, vnode, scopeId, slotScopeIds, parentComponent) => {
			if (scopeId) hostSetScopeId(el, scopeId);
			if (slotScopeIds) for (let i = 0; i < slotScopeIds.length; i++) hostSetScopeId(el, slotScopeIds[i]);
			if (parentComponent) {
				let subTree = parentComponent.subTree;
				if (vnode === subTree || isSuspense(subTree.type) && (subTree.ssContent === vnode || subTree.ssFallback === vnode)) {
					const parentVNode = parentComponent.vnode;
					setScopeId(el, parentVNode, parentVNode.scopeId, parentVNode.slotScopeIds, parentComponent.parent);
				}
			}
		};
		const mountChildren = (children, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized, start = 0) => {
			for (let i = start; i < children.length; i++) {
				const child = children[i] = optimized ? cloneIfMounted(children[i]) : normalizeVNode(children[i]);
				patch(null, child, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
			}
		};
		const patchElement = (n1, n2, parentComponent, parentSuspense, namespace, slotScopeIds, optimized) => {
			const el = n2.el = n1.el;
			let { patchFlag, dynamicChildren, dirs } = n2;
			patchFlag |= n1.patchFlag & 16;
			const oldProps = n1.props || EMPTY_OBJ;
			const newProps = n2.props || EMPTY_OBJ;
			let vnodeHook;
			parentComponent && toggleRecurse(parentComponent, false);
			if (vnodeHook = newProps.onVnodeBeforeUpdate) invokeVNodeHook(vnodeHook, parentComponent, n2, n1);
			if (dirs) invokeDirectiveHook(n2, n1, parentComponent, "beforeUpdate");
			parentComponent && toggleRecurse(parentComponent, true);
			if (dynamicChildren && (!n1.dynamicChildren || n1.dynamicChildren.length !== dynamicChildren.length)) {
				patchFlag = 0;
				optimized = false;
				dynamicChildren = null;
			}
			if (oldProps.innerHTML && newProps.innerHTML == null || oldProps.textContent && newProps.textContent == null) hostSetElementText(el, "");
			if (dynamicChildren) patchBlockChildren(n1.dynamicChildren, dynamicChildren, el, parentComponent, parentSuspense, resolveChildrenNamespace(n2, namespace), slotScopeIds);
			else if (!optimized) patchChildren(n1, n2, el, null, parentComponent, parentSuspense, resolveChildrenNamespace(n2, namespace), slotScopeIds, false);
			if (patchFlag > 0) {
				if (patchFlag & 16) patchProps(el, oldProps, newProps, parentComponent, namespace);
				else {
					if (patchFlag & 2) {
						if (oldProps.class !== newProps.class) hostPatchProp(el, "class", null, newProps.class, namespace);
					}
					if (patchFlag & 4) hostPatchProp(el, "style", oldProps.style, newProps.style, namespace);
					if (patchFlag & 8) {
						const propsToUpdate = n2.dynamicProps;
						for (let i = 0; i < propsToUpdate.length; i++) {
							const key = propsToUpdate[i];
							const prev = oldProps[key];
							const next = newProps[key];
							if (next !== prev || key === "value") hostPatchProp(el, key, prev, next, namespace, parentComponent);
						}
					}
				}
				if (patchFlag & 1) {
					if (n1.children !== n2.children) hostSetElementText(el, n2.children);
				}
			} else if (!optimized && dynamicChildren == null) patchProps(el, oldProps, newProps, parentComponent, namespace);
			if ((vnodeHook = newProps.onVnodeUpdated) || dirs) queuePostRenderEffect(() => {
				vnodeHook && invokeVNodeHook(vnodeHook, parentComponent, n2, n1);
				dirs && invokeDirectiveHook(n2, n1, parentComponent, "updated");
			}, parentSuspense);
		};
		const patchBlockChildren = (oldChildren, newChildren, fallbackContainer, parentComponent, parentSuspense, namespace, slotScopeIds) => {
			for (let i = 0; i < newChildren.length; i++) {
				const oldVNode = oldChildren[i];
				const newVNode = newChildren[i];
				const container = oldVNode.el && (oldVNode.type === Fragment || !isSameVNodeType(oldVNode, newVNode) || oldVNode.shapeFlag & 198) ? hostParentNode(oldVNode.el) : fallbackContainer;
				patch(oldVNode, newVNode, container, null, parentComponent, parentSuspense, namespace, slotScopeIds, true);
			}
		};
		const patchProps = (el, oldProps, newProps, parentComponent, namespace) => {
			if (oldProps !== newProps) {
				if (oldProps !== EMPTY_OBJ) {
					for (const key in oldProps) if (!isReservedProp(key) && !(key in newProps)) hostPatchProp(el, key, oldProps[key], null, namespace, parentComponent);
				}
				for (const key in newProps) {
					if (isReservedProp(key)) continue;
					const next = newProps[key];
					const prev = oldProps[key];
					if (next !== prev && key !== "value") hostPatchProp(el, key, prev, next, namespace, parentComponent);
				}
				if ("value" in newProps) hostPatchProp(el, "value", oldProps.value, newProps.value, namespace);
			}
		};
		const processFragment = (n1, n2, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized) => {
			const fragmentStartAnchor = n2.el = n1 ? n1.el : hostCreateText("");
			const fragmentEndAnchor = n2.anchor = n1 ? n1.anchor : hostCreateText("");
			let { patchFlag, dynamicChildren, slotScopeIds: fragmentSlotScopeIds } = n2;
			if (fragmentSlotScopeIds) slotScopeIds = slotScopeIds ? slotScopeIds.concat(fragmentSlotScopeIds) : fragmentSlotScopeIds;
			if (n1 == null) {
				hostInsert(fragmentStartAnchor, container, anchor);
				hostInsert(fragmentEndAnchor, container, anchor);
				mountChildren(n2.children || [], container, fragmentEndAnchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
			} else if (patchFlag > 0 && patchFlag & 64 && dynamicChildren && n1.dynamicChildren && n1.dynamicChildren.length === dynamicChildren.length) {
				patchBlockChildren(n1.dynamicChildren, dynamicChildren, container, parentComponent, parentSuspense, namespace, slotScopeIds);
				if (n2.key != null || parentComponent && n2 === parentComponent.subTree) traverseStaticChildren(n1, n2, true);
			} else patchChildren(n1, n2, container, fragmentEndAnchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
		};
		const processComponent = (n1, n2, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized) => {
			n2.slotScopeIds = slotScopeIds;
			if (n1 == null) if (n2.shapeFlag & 512) parentComponent.ctx.activate(n2, container, anchor, namespace, optimized);
			else mountComponent(n2, container, anchor, parentComponent, parentSuspense, namespace, optimized);
			else updateComponent(n1, n2, optimized);
		};
		const mountComponent = (initialVNode, container, anchor, parentComponent, parentSuspense, namespace, optimized) => {
			const instance = initialVNode.component = createComponentInstance(initialVNode, parentComponent, parentSuspense);
			if (isKeepAlive(initialVNode)) instance.ctx.renderer = internals;
			setupComponent(instance, false, optimized);
			if (instance.asyncDep) {
				parentSuspense && parentSuspense.registerDep(instance, setupRenderEffect, optimized);
				if (!initialVNode.el) {
					const placeholder = instance.subTree = createVNode(Comment);
					processCommentNode(null, placeholder, container, anchor);
					initialVNode.placeholder = placeholder.el;
				}
			} else setupRenderEffect(instance, initialVNode, container, anchor, parentSuspense, namespace, optimized);
		};
		const updateComponent = (n1, n2, optimized) => {
			const instance = n2.component = n1.component;
			if (shouldUpdateComponent(n1, n2, optimized)) if (instance.asyncDep && !instance.asyncResolved) {
				updateComponentPreRender(instance, n2, optimized);
				return;
			} else {
				instance.next = n2;
				instance.update();
			}
			else {
				n2.el = n1.el;
				instance.vnode = n2;
			}
		};
		const setupRenderEffect = (instance, initialVNode, container, anchor, parentSuspense, namespace, optimized) => {
			const componentUpdateFn = () => {
				if (!instance.isMounted) {
					let vnodeHook;
					const { el, props } = initialVNode;
					const { bm, m, parent, root, type } = instance;
					const isAsyncWrapperVNode = isAsyncWrapper(initialVNode);
					toggleRecurse(instance, false);
					if (bm) invokeArrayFns(bm);
					if (!isAsyncWrapperVNode && (vnodeHook = props && props.onVnodeBeforeMount)) invokeVNodeHook(vnodeHook, parent, initialVNode);
					toggleRecurse(instance, true);
					if (el && hydrateNode) {
						const hydrateSubTree = () => {
							instance.subTree = renderComponentRoot(instance);
							hydrateNode(el, instance.subTree, instance, parentSuspense, null);
						};
						if (isAsyncWrapperVNode && type.__asyncHydrate) type.__asyncHydrate(el, instance, hydrateSubTree);
						else hydrateSubTree();
					} else {
						if (root.ce && root.ce._hasShadowRoot()) root.ce._injectChildStyle(type, instance.parent ? instance.parent.type : void 0);
						const subTree = instance.subTree = renderComponentRoot(instance);
						patch(null, subTree, container, anchor, instance, parentSuspense, namespace);
						initialVNode.el = subTree.el;
					}
					if (m) queuePostRenderEffect(m, parentSuspense);
					if (!isAsyncWrapperVNode && (vnodeHook = props && props.onVnodeMounted)) {
						const scopedInitialVNode = initialVNode;
						queuePostRenderEffect(() => invokeVNodeHook(vnodeHook, parent, scopedInitialVNode), parentSuspense);
					}
					if (initialVNode.shapeFlag & 256 || parent && isAsyncWrapper(parent.vnode) && parent.vnode.shapeFlag & 256) instance.a && queuePostRenderEffect(instance.a, parentSuspense);
					instance.isMounted = true;
					initialVNode = container = anchor = null;
				} else {
					let { next, bu, u, parent, vnode } = instance;
					{
						const nonHydratedAsyncRoot = locateNonHydratedAsyncRoot(instance);
						if (nonHydratedAsyncRoot) {
							if (next) {
								next.el = vnode.el;
								updateComponentPreRender(instance, next, optimized);
							}
							nonHydratedAsyncRoot.asyncDep.then(() => {
								queuePostRenderEffect(() => {
									if (!instance.isUnmounted) update();
								}, parentSuspense);
							});
							return;
						}
					}
					let originNext = next;
					let vnodeHook;
					toggleRecurse(instance, false);
					if (next) {
						next.el = vnode.el;
						updateComponentPreRender(instance, next, optimized);
					} else next = vnode;
					if (bu) invokeArrayFns(bu);
					if (vnodeHook = next.props && next.props.onVnodeBeforeUpdate) invokeVNodeHook(vnodeHook, parent, next, vnode);
					toggleRecurse(instance, true);
					const nextTree = renderComponentRoot(instance);
					const prevTree = instance.subTree;
					instance.subTree = nextTree;
					patch(prevTree, nextTree, hostParentNode(prevTree.el), getNextHostNode(prevTree), instance, parentSuspense, namespace);
					next.el = nextTree.el;
					if (originNext === null) updateHOCHostEl(instance, nextTree.el);
					if (u) queuePostRenderEffect(u, parentSuspense);
					if (vnodeHook = next.props && next.props.onVnodeUpdated) queuePostRenderEffect(() => invokeVNodeHook(vnodeHook, parent, next, vnode), parentSuspense);
				}
			};
			instance.scope.on();
			const effect = instance.effect = new ReactiveEffect(componentUpdateFn);
			instance.scope.off();
			const update = instance.update = effect.run.bind(effect);
			const job = instance.job = effect.runIfDirty.bind(effect);
			job.i = instance;
			job.id = instance.uid;
			effect.scheduler = () => queueJob(job);
			toggleRecurse(instance, true);
			update();
		};
		const updateComponentPreRender = (instance, nextVNode, optimized) => {
			nextVNode.component = instance;
			const prevProps = instance.vnode.props;
			instance.vnode = nextVNode;
			instance.next = null;
			updateProps(instance, nextVNode.props, prevProps, optimized);
			updateSlots(instance, nextVNode.children, optimized);
			pauseTracking();
			flushPreFlushCbs(instance);
			resetTracking();
		};
		const patchChildren = (n1, n2, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized = false) => {
			const c1 = n1 && n1.children;
			const prevShapeFlag = n1 ? n1.shapeFlag : 0;
			const c2 = n2.children;
			const { patchFlag, shapeFlag } = n2;
			if (patchFlag > 0) {
				if (patchFlag & 128) {
					patchKeyedChildren(c1, c2, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
					return;
				} else if (patchFlag & 256) {
					patchUnkeyedChildren(c1, c2, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
					return;
				}
			}
			if (shapeFlag & 8) {
				if (prevShapeFlag & 16) unmountChildren(c1, parentComponent, parentSuspense);
				if (c2 !== c1) hostSetElementText(container, c2);
			} else if (prevShapeFlag & 16) if (shapeFlag & 16) patchKeyedChildren(c1, c2, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
			else unmountChildren(c1, parentComponent, parentSuspense, true);
			else {
				if (prevShapeFlag & 8) hostSetElementText(container, "");
				if (shapeFlag & 16) mountChildren(c2, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
			}
		};
		const patchUnkeyedChildren = (c1, c2, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized) => {
			c1 = c1 || EMPTY_ARR;
			c2 = c2 || EMPTY_ARR;
			const oldLength = c1.length;
			const newLength = c2.length;
			const commonLength = Math.min(oldLength, newLength);
			let i;
			for (i = 0; i < commonLength; i++) {
				const nextChild = c2[i] = optimized ? cloneIfMounted(c2[i]) : normalizeVNode(c2[i]);
				patch(c1[i], nextChild, container, null, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
			}
			if (oldLength > newLength) unmountChildren(c1, parentComponent, parentSuspense, true, false, commonLength);
			else mountChildren(c2, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized, commonLength);
		};
		const patchKeyedChildren = (c1, c2, container, parentAnchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized) => {
			let i = 0;
			const l2 = c2.length;
			let e1 = c1.length - 1;
			let e2 = l2 - 1;
			while (i <= e1 && i <= e2) {
				const n1 = c1[i];
				const n2 = c2[i] = optimized ? cloneIfMounted(c2[i]) : normalizeVNode(c2[i]);
				if (isSameVNodeType(n1, n2)) patch(n1, n2, container, null, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
				else break;
				i++;
			}
			while (i <= e1 && i <= e2) {
				const n1 = c1[e1];
				const n2 = c2[e2] = optimized ? cloneIfMounted(c2[e2]) : normalizeVNode(c2[e2]);
				if (isSameVNodeType(n1, n2)) patch(n1, n2, container, null, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
				else break;
				e1--;
				e2--;
			}
			if (i > e1) {
				if (i <= e2) {
					const nextPos = e2 + 1;
					const anchor = nextPos < l2 ? c2[nextPos].el : parentAnchor;
					while (i <= e2) {
						patch(null, c2[i] = optimized ? cloneIfMounted(c2[i]) : normalizeVNode(c2[i]), container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
						i++;
					}
				}
			} else if (i > e2) while (i <= e1) {
				unmount(c1[i], parentComponent, parentSuspense, true);
				i++;
			}
			else {
				const s1 = i;
				const s2 = i;
				const keyToNewIndexMap = /* @__PURE__ */ new Map();
				for (i = s2; i <= e2; i++) {
					const nextChild = c2[i] = optimized ? cloneIfMounted(c2[i]) : normalizeVNode(c2[i]);
					if (nextChild.key != null) keyToNewIndexMap.set(nextChild.key, i);
				}
				let j;
				let patched = 0;
				const toBePatched = e2 - s2 + 1;
				let moved = false;
				let maxNewIndexSoFar = 0;
				const newIndexToOldIndexMap = new Array(toBePatched);
				for (i = 0; i < toBePatched; i++) newIndexToOldIndexMap[i] = 0;
				for (i = s1; i <= e1; i++) {
					const prevChild = c1[i];
					if (patched >= toBePatched) {
						unmount(prevChild, parentComponent, parentSuspense, true);
						continue;
					}
					let newIndex;
					if (prevChild.key != null) newIndex = keyToNewIndexMap.get(prevChild.key);
					else for (j = s2; j <= e2; j++) if (newIndexToOldIndexMap[j - s2] === 0 && isSameVNodeType(prevChild, c2[j])) {
						newIndex = j;
						break;
					}
					if (newIndex === void 0) unmount(prevChild, parentComponent, parentSuspense, true);
					else {
						newIndexToOldIndexMap[newIndex - s2] = i + 1;
						if (newIndex >= maxNewIndexSoFar) maxNewIndexSoFar = newIndex;
						else moved = true;
						patch(prevChild, c2[newIndex], container, null, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
						patched++;
					}
				}
				const increasingNewIndexSequence = moved ? getSequence(newIndexToOldIndexMap) : EMPTY_ARR;
				j = increasingNewIndexSequence.length - 1;
				for (i = toBePatched - 1; i >= 0; i--) {
					const nextIndex = s2 + i;
					const nextChild = c2[nextIndex];
					const anchorVNode = c2[nextIndex + 1];
					const anchor = nextIndex + 1 < l2 ? anchorVNode.el || resolveAsyncComponentPlaceholder(anchorVNode) : parentAnchor;
					if (newIndexToOldIndexMap[i] === 0) patch(null, nextChild, container, anchor, parentComponent, parentSuspense, namespace, slotScopeIds, optimized);
					else if (moved) if (j < 0 || i !== increasingNewIndexSequence[j]) move(nextChild, container, anchor, 2);
					else j--;
				}
			}
		};
		const move = (vnode, container, anchor, moveType, parentSuspense = null) => {
			const { el, type, transition, children, shapeFlag } = vnode;
			if (shapeFlag & 6) {
				move(vnode.component.subTree, container, anchor, moveType);
				return;
			}
			if (shapeFlag & 128) {
				vnode.suspense.move(container, anchor, moveType);
				return;
			}
			if (shapeFlag & 64) {
				type.move(vnode, container, anchor, internals);
				return;
			}
			if (type === Fragment) {
				hostInsert(el, container, anchor);
				for (let i = 0; i < children.length; i++) move(children[i], container, anchor, moveType);
				hostInsert(vnode.anchor, container, anchor);
				return;
			}
			if (type === Static) {
				moveStaticNode(vnode, container, anchor);
				return;
			}
			if (moveType !== 2 && shapeFlag & 1 && transition) if (moveType === 0) if (transition.persisted && !el[leaveCbKey]) hostInsert(el, container, anchor);
			else {
				transition.beforeEnter(el);
				hostInsert(el, container, anchor);
				queuePostRenderEffect(() => transition.enter(el), parentSuspense);
			}
			else {
				const { leave, delayLeave, afterLeave } = transition;
				const remove2 = () => {
					if (vnode.ctx.isUnmounted) hostRemove(el);
					else hostInsert(el, container, anchor);
				};
				const performLeave = () => {
					const wasLeaving = el._isLeaving || !!el[leaveCbKey];
					if (el._isLeaving) el[leaveCbKey](true);
					if (transition.persisted && !wasLeaving) remove2();
					else leave(el, () => {
						remove2();
						afterLeave && afterLeave();
					});
				};
				if (delayLeave) delayLeave(el, remove2, performLeave);
				else performLeave();
			}
			else hostInsert(el, container, anchor);
		};
		const unmount = (vnode, parentComponent, parentSuspense, doRemove = false, optimized = false) => {
			const { type, props, ref, children, dynamicChildren, shapeFlag, patchFlag, dirs, cacheIndex, memo } = vnode;
			if (patchFlag === -2) optimized = false;
			if (ref != null) {
				pauseTracking();
				setRef(ref, null, parentSuspense, vnode, true);
				resetTracking();
			}
			if (cacheIndex != null) parentComponent.renderCache[cacheIndex] = void 0;
			if (shapeFlag & 256) {
				parentComponent.ctx.deactivate(vnode);
				return;
			}
			const shouldInvokeDirs = shapeFlag & 1 && dirs;
			const shouldInvokeVnodeHook = !isAsyncWrapper(vnode);
			let vnodeHook;
			if (shouldInvokeVnodeHook && (vnodeHook = props && props.onVnodeBeforeUnmount)) invokeVNodeHook(vnodeHook, parentComponent, vnode);
			if (shapeFlag & 6) unmountComponent(vnode.component, parentSuspense, doRemove);
			else {
				if (shapeFlag & 128) {
					vnode.suspense.unmount(parentSuspense, doRemove);
					return;
				}
				if (shouldInvokeDirs) invokeDirectiveHook(vnode, null, parentComponent, "beforeUnmount");
				if (shapeFlag & 64) vnode.type.remove(vnode, parentComponent, parentSuspense, internals, doRemove);
				else if (dynamicChildren && !dynamicChildren.hasOnce && (type !== Fragment || patchFlag > 0 && patchFlag & 64)) unmountChildren(dynamicChildren, parentComponent, parentSuspense, false, true);
				else if (type === Fragment && patchFlag & 384 || !optimized && shapeFlag & 16) unmountChildren(children, parentComponent, parentSuspense);
				if (doRemove) remove(vnode);
			}
			const shouldInvalidateMemo = memo != null && cacheIndex == null;
			if (shouldInvokeVnodeHook && (vnodeHook = props && props.onVnodeUnmounted) || shouldInvokeDirs || shouldInvalidateMemo) queuePostRenderEffect(() => {
				vnodeHook && invokeVNodeHook(vnodeHook, parentComponent, vnode);
				shouldInvokeDirs && invokeDirectiveHook(vnode, null, parentComponent, "unmounted");
				if (shouldInvalidateMemo) vnode.el = null;
			}, parentSuspense);
		};
		const remove = (vnode) => {
			const { type, el, anchor, transition } = vnode;
			if (type === Fragment) {
				removeFragment(el, anchor);
				return;
			}
			if (type === Static) {
				removeStaticNode(vnode);
				return;
			}
			const performRemove = () => {
				hostRemove(el);
				if (transition && !transition.persisted && transition.afterLeave) transition.afterLeave();
			};
			if (vnode.shapeFlag & 1 && transition && !transition.persisted) {
				const { leave, delayLeave } = transition;
				const performLeave = () => leave(el, performRemove);
				if (delayLeave) delayLeave(vnode.el, performRemove, performLeave);
				else performLeave();
			} else performRemove();
		};
		const removeFragment = (cur, end) => {
			let next;
			while (cur !== end) {
				next = hostNextSibling(cur);
				hostRemove(cur);
				cur = next;
			}
			hostRemove(end);
		};
		const unmountComponent = (instance, parentSuspense, doRemove) => {
			const { bum, scope, job, subTree, um, m, a } = instance;
			invalidateMount(m);
			invalidateMount(a);
			if (bum) invokeArrayFns(bum);
			scope.stop();
			if (job) {
				job.flags |= 8;
				unmount(subTree, instance, parentSuspense, doRemove);
			}
			if (um) queuePostRenderEffect(um, parentSuspense);
			queuePostRenderEffect(() => {
				instance.isUnmounted = true;
			}, parentSuspense);
		};
		const unmountChildren = (children, parentComponent, parentSuspense, doRemove = false, optimized = false, start = 0) => {
			for (let i = start; i < children.length; i++) unmount(children[i], parentComponent, parentSuspense, doRemove, optimized);
		};
		const getNextHostNode = (vnode) => {
			if (vnode.shapeFlag & 6) return getNextHostNode(vnode.component.subTree);
			if (vnode.shapeFlag & 128) return vnode.suspense.next();
			const el = hostNextSibling(vnode.anchor || vnode.el);
			const teleportEnd = el && el[TeleportEndKey];
			return teleportEnd ? hostNextSibling(teleportEnd) : el;
		};
		let isFlushing = false;
		const render = (vnode, container, namespace) => {
			let instance;
			if (vnode == null) {
				if (container._vnode) {
					unmount(container._vnode, null, null, true);
					instance = container._vnode.component;
				}
			} else patch(container._vnode || null, vnode, container, null, null, null, namespace);
			container._vnode = vnode;
			if (!isFlushing) {
				isFlushing = true;
				flushPreFlushCbs(instance);
				flushPostFlushCbs();
				isFlushing = false;
			}
		};
		const internals = {
			p: patch,
			um: unmount,
			m: move,
			r: remove,
			mt: mountComponent,
			mc: mountChildren,
			pc: patchChildren,
			pbc: patchBlockChildren,
			n: getNextHostNode,
			o: options
		};
		let hydrate;
		let hydrateNode;
		if (createHydrationFns) [hydrate, hydrateNode] = createHydrationFns(internals);
		return {
			render,
			hydrate,
			createApp: createAppAPI(render, hydrate)
		};
	}
	function resolveChildrenNamespace({ type, props }, currentNamespace) {
		return currentNamespace === "svg" && type === "foreignObject" || currentNamespace === "mathml" && type === "annotation-xml" && props && props.encoding && props.encoding.includes("html") ? void 0 : currentNamespace;
	}
	function toggleRecurse({ effect, job }, allowed) {
		if (allowed) {
			effect.flags |= 32;
			job.flags |= 4;
		} else {
			effect.flags &= -33;
			job.flags &= -5;
		}
	}
	function needTransition(parentSuspense, transition) {
		return (!parentSuspense || parentSuspense && !parentSuspense.pendingBranch) && transition && !transition.persisted;
	}
	function traverseStaticChildren(n1, n2, shallow = false) {
		const ch1 = n1.children;
		const ch2 = n2.children;
		if (isArray(ch1) && isArray(ch2)) for (let i = 0; i < ch1.length; i++) {
			const c1 = ch1[i];
			let c2 = ch2[i];
			if (c2.shapeFlag & 1 && !c2.dynamicChildren) {
				if (c2.patchFlag <= 0 || c2.patchFlag === 32) {
					c2 = ch2[i] = cloneIfMounted(ch2[i]);
					c2.el = c1.el;
				}
				if (!shallow && c2.patchFlag !== -2) traverseStaticChildren(c1, c2);
			}
			if (c2.type === Text) {
				if (c2.patchFlag === -1) c2 = ch2[i] = cloneIfMounted(c2);
				c2.el = c1.el;
			}
			if (c2.type === Comment && !c2.el) c2.el = c1.el;
		}
	}
	function getSequence(arr) {
		const p = arr.slice();
		const result = [0];
		let i, j, u, v, c;
		const len = arr.length;
		for (i = 0; i < len; i++) {
			const arrI = arr[i];
			if (arrI !== 0) {
				j = result[result.length - 1];
				if (arr[j] < arrI) {
					p[i] = j;
					result.push(i);
					continue;
				}
				u = 0;
				v = result.length - 1;
				while (u < v) {
					c = u + v >> 1;
					if (arr[result[c]] < arrI) u = c + 1;
					else v = c;
				}
				if (arrI < arr[result[u]]) {
					if (u > 0) p[i] = result[u - 1];
					result[u] = i;
				}
			}
		}
		u = result.length;
		v = result[u - 1];
		while (u-- > 0) {
			result[u] = v;
			v = p[v];
		}
		return result;
	}
	function locateNonHydratedAsyncRoot(instance) {
		const subComponent = instance.subTree.component;
		if (subComponent) if (subComponent.asyncDep && !subComponent.asyncResolved) return subComponent;
		else return locateNonHydratedAsyncRoot(subComponent);
	}
	function invalidateMount(hooks) {
		if (hooks) for (let i = 0; i < hooks.length; i++) hooks[i].flags |= 8;
	}
	function resolveAsyncComponentPlaceholder(anchorVnode) {
		if (anchorVnode.placeholder) return anchorVnode.placeholder;
		const instance = anchorVnode.component;
		if (instance) return resolveAsyncComponentPlaceholder(instance.subTree);
		return null;
	}
	var isSuspense = (type) => type.__isSuspense;
	function queueEffectWithSuspense(fn, suspense) {
		if (suspense && suspense.pendingBranch) if (isArray(fn)) suspense.effects.push(...fn);
		else suspense.effects.push(fn);
		else queuePostFlushCb(fn);
	}
	var Fragment = /* @__PURE__ */ Symbol.for("v-fgt");
	var Text = /* @__PURE__ */ Symbol.for("v-txt");
	var Comment = /* @__PURE__ */ Symbol.for("v-cmt");
	var Static = /* @__PURE__ */ Symbol.for("v-stc");
	var blockStack = [];
	var currentBlock = null;
	function openBlock(disableTracking = false) {
		blockStack.push(currentBlock = disableTracking ? null : []);
	}
	function closeBlock() {
		blockStack.pop();
		currentBlock = blockStack[blockStack.length - 1] || null;
	}
	var isBlockTreeEnabled = 1;
	function setBlockTracking(value, inVOnce = false) {
		isBlockTreeEnabled += value;
		if (value < 0 && currentBlock && inVOnce) currentBlock.hasOnce = true;
	}
	function setupBlock(vnode) {
		vnode.dynamicChildren = isBlockTreeEnabled > 0 ? currentBlock || EMPTY_ARR : null;
		closeBlock();
		if (isBlockTreeEnabled > 0 && currentBlock) currentBlock.push(vnode);
		return vnode;
	}
	function createElementBlock(type, props, children, patchFlag, dynamicProps, shapeFlag) {
		return setupBlock(createBaseVNode(type, props, children, patchFlag, dynamicProps, shapeFlag, true));
	}
	function createBlock(type, props, children, patchFlag, dynamicProps) {
		return setupBlock(createVNode(type, props, children, patchFlag, dynamicProps, true));
	}
	function isVNode(value) {
		return value ? value.__v_isVNode === true : false;
	}
	function isSameVNodeType(n1, n2) {
		return n1.type === n2.type && n1.key === n2.key;
	}
	var normalizeKey = ({ key }) => key != null ? key : null;
	var normalizeRef = ({ ref, ref_key, ref_for }) => {
		if (typeof ref === "number") ref = "" + ref;
		return ref != null ? isString(ref) || /* @__PURE__ */ isRef(ref) || isFunction(ref) ? {
			i: currentRenderingInstance,
			r: ref,
			k: ref_key,
			f: !!ref_for
		} : ref : null;
	};
	function createBaseVNode(type, props = null, children = null, patchFlag = 0, dynamicProps = null, shapeFlag = type === Fragment ? 0 : 1, isBlockNode = false, needFullChildrenNormalization = false) {
		const vnode = {
			__v_isVNode: true,
			__v_skip: true,
			type,
			props,
			key: props && normalizeKey(props),
			ref: props && normalizeRef(props),
			scopeId: currentScopeId,
			slotScopeIds: null,
			children,
			component: null,
			suspense: null,
			ssContent: null,
			ssFallback: null,
			dirs: null,
			transition: null,
			el: null,
			anchor: null,
			target: null,
			targetStart: null,
			targetAnchor: null,
			staticCount: 0,
			shapeFlag,
			patchFlag,
			dynamicProps,
			dynamicChildren: null,
			appContext: null,
			ctx: currentRenderingInstance
		};
		if (needFullChildrenNormalization) {
			normalizeChildren(vnode, children);
			if (shapeFlag & 128) type.normalize(vnode);
		} else if (children) vnode.shapeFlag |= isString(children) ? 8 : 16;
		if (isBlockTreeEnabled > 0 && !isBlockNode && currentBlock && (vnode.patchFlag > 0 || shapeFlag & 6) && vnode.patchFlag !== 32) currentBlock.push(vnode);
		return vnode;
	}
	var createVNode = _createVNode;
	function _createVNode(type, props = null, children = null, patchFlag = 0, dynamicProps = null, isBlockNode = false) {
		if (!type || type === NULL_DYNAMIC_COMPONENT) type = Comment;
		if (isVNode(type)) {
			const cloned = cloneVNode(type, props, true);
			if (children) normalizeChildren(cloned, children);
			if (isBlockTreeEnabled > 0 && !isBlockNode && currentBlock) if (cloned.shapeFlag & 6) currentBlock[currentBlock.indexOf(type)] = cloned;
			else currentBlock.push(cloned);
			cloned.patchFlag = -2;
			return cloned;
		}
		if (isClassComponent(type)) type = type.__vccOpts;
		if (props) {
			props = guardReactiveProps(props);
			let { class: klass, style } = props;
			if (klass && !isString(klass)) props.class = normalizeClass(klass);
			if (isObject(style)) {
				if (/* @__PURE__ */ isProxy(style) && !isArray(style)) style = extend({}, style);
				props.style = normalizeStyle(style);
			}
		}
		const shapeFlag = isString(type) ? 1 : isSuspense(type) ? 128 : isTeleport(type) ? 64 : isObject(type) ? 4 : isFunction(type) ? 2 : 0;
		return createBaseVNode(type, props, children, patchFlag, dynamicProps, shapeFlag, isBlockNode, true);
	}
	function guardReactiveProps(props) {
		if (!props) return null;
		return /* @__PURE__ */ isProxy(props) || isInternalObject(props) ? extend({}, props) : props;
	}
	function cloneVNode(vnode, extraProps, mergeRef = false, cloneTransition = false) {
		const { props, ref, patchFlag, children, transition } = vnode;
		const mergedProps = extraProps ? mergeProps(props || {}, extraProps) : props;
		const cloned = {
			__v_isVNode: true,
			__v_skip: true,
			type: vnode.type,
			props: mergedProps,
			key: mergedProps && normalizeKey(mergedProps),
			ref: extraProps && extraProps.ref ? mergeRef && ref ? isArray(ref) ? ref.concat(normalizeRef(extraProps)) : [ref, normalizeRef(extraProps)] : normalizeRef(extraProps) : ref,
			scopeId: vnode.scopeId,
			slotScopeIds: vnode.slotScopeIds,
			children,
			target: vnode.target,
			targetStart: vnode.targetStart,
			targetAnchor: vnode.targetAnchor,
			staticCount: vnode.staticCount,
			shapeFlag: vnode.shapeFlag,
			patchFlag: extraProps && vnode.type !== Fragment ? patchFlag === -1 ? 16 : patchFlag | 16 : patchFlag,
			dynamicProps: vnode.dynamicProps,
			dynamicChildren: vnode.dynamicChildren,
			appContext: vnode.appContext,
			dirs: vnode.dirs,
			transition,
			component: vnode.component,
			suspense: vnode.suspense,
			ssContent: vnode.ssContent && cloneVNode(vnode.ssContent),
			ssFallback: vnode.ssFallback && cloneVNode(vnode.ssFallback),
			placeholder: vnode.placeholder,
			el: vnode.el,
			anchor: vnode.anchor,
			ctx: vnode.ctx,
			ce: vnode.ce
		};
		if (transition && cloneTransition) setTransitionHooks(cloned, transition.clone(cloned));
		return cloned;
	}
	function createTextVNode(text = " ", flag = 0) {
		return createVNode(Text, null, text, flag);
	}
	function createStaticVNode(content, numberOfNodes) {
		const vnode = createVNode(Static, null, content);
		vnode.staticCount = numberOfNodes;
		return vnode;
	}
	function createCommentVNode(text = "", asBlock = false) {
		return asBlock ? (openBlock(), createBlock(Comment, null, text)) : createVNode(Comment, null, text);
	}
	function normalizeVNode(child) {
		if (child == null || typeof child === "boolean") return createVNode(Comment);
		else if (isArray(child)) return createVNode(Fragment, null, child.slice());
		else if (isVNode(child)) return cloneIfMounted(child);
		else return createVNode(Text, null, String(child));
	}
	function cloneIfMounted(child) {
		return child.el === null && child.patchFlag !== -1 || child.memo ? child : cloneVNode(child);
	}
	function normalizeChildren(vnode, children) {
		let type = 0;
		const { shapeFlag } = vnode;
		if (children == null) children = null;
		else if (isArray(children)) type = 16;
		else if (typeof children === "object") if (shapeFlag & 65) {
			const slot = children.default;
			if (slot) {
				slot._c && (slot._d = false);
				normalizeChildren(vnode, slot());
				slot._c && (slot._d = true);
			}
			return;
		} else {
			type = 32;
			const slotFlag = children._;
			if (!slotFlag && !isInternalObject(children)) children._ctx = currentRenderingInstance;
			else if (slotFlag === 3 && currentRenderingInstance) if (currentRenderingInstance.slots._ === 1) children._ = 1;
			else {
				children._ = 2;
				vnode.patchFlag |= 1024;
			}
		}
		else if (isFunction(children)) {
			if (shapeFlag & 65) {
				normalizeChildren(vnode, { default: children });
				return;
			}
			children = {
				default: children,
				_ctx: currentRenderingInstance
			};
			type = 32;
		} else {
			children = String(children);
			if (shapeFlag & 64) {
				type = 16;
				children = [createTextVNode(children)];
			} else type = 8;
		}
		vnode.children = children;
		vnode.shapeFlag |= type;
	}
	function mergeProps(...args) {
		const ret = {};
		for (let i = 0; i < args.length; i++) {
			const toMerge = args[i];
			for (const key in toMerge) if (key === "class") {
				if (ret.class !== toMerge.class) ret.class = normalizeClass([ret.class, toMerge.class]);
			} else if (key === "style") ret.style = normalizeStyle([ret.style, toMerge.style]);
			else if (isOn(key)) {
				const existing = ret[key];
				const incoming = toMerge[key];
				if (incoming && existing !== incoming && !(isArray(existing) && existing.includes(incoming))) ret[key] = existing ? [].concat(existing, incoming) : incoming;
				else if (incoming == null && existing == null && !isModelListener(key)) ret[key] = incoming;
			} else if (key !== "") ret[key] = toMerge[key];
		}
		return ret;
	}
	function invokeVNodeHook(hook, instance, vnode, prevVNode = null) {
		callWithAsyncErrorHandling(hook, instance, 7, [vnode, prevVNode]);
	}
	var emptyAppContext = createAppContext();
	var uid = 0;
	function createComponentInstance(vnode, parent, suspense) {
		const type = vnode.type;
		const appContext = (parent ? parent.appContext : vnode.appContext) || emptyAppContext;
		const instance = {
			uid: uid++,
			vnode,
			type,
			parent,
			appContext,
			root: null,
			next: null,
			subTree: null,
			effect: null,
			update: null,
			job: null,
			scope: new EffectScope(true),
			render: null,
			proxy: null,
			exposed: null,
			exposeProxy: null,
			withProxy: null,
			provides: parent ? parent.provides : Object.create(appContext.provides),
			ids: parent ? parent.ids : [
				"",
				0,
				0
			],
			accessCache: null,
			renderCache: [],
			components: null,
			directives: null,
			propsOptions: normalizePropsOptions(type, appContext),
			emitsOptions: normalizeEmitsOptions(type, appContext),
			emit: null,
			emitted: null,
			propsDefaults: EMPTY_OBJ,
			inheritAttrs: type.inheritAttrs,
			ctx: EMPTY_OBJ,
			data: EMPTY_OBJ,
			props: EMPTY_OBJ,
			attrs: EMPTY_OBJ,
			slots: EMPTY_OBJ,
			refs: EMPTY_OBJ,
			setupState: EMPTY_OBJ,
			setupContext: null,
			suspense,
			suspenseId: suspense ? suspense.pendingId : 0,
			asyncDep: null,
			asyncResolved: false,
			isMounted: false,
			isUnmounted: false,
			isDeactivated: false,
			bc: null,
			c: null,
			bm: null,
			m: null,
			bu: null,
			u: null,
			um: null,
			bum: null,
			da: null,
			a: null,
			rtg: null,
			rtc: null,
			ec: null,
			sp: null
		};
		instance.ctx = { _: instance };
		instance.root = parent ? parent.root : instance;
		instance.emit = emit.bind(null, instance);
		if (vnode.ce) vnode.ce(instance);
		return instance;
	}
	var currentInstance = null;
	var getCurrentInstance = () => currentInstance || currentRenderingInstance;
	var internalSetCurrentInstance;
	var setInSSRSetupState;
	{
		const g = getGlobalThis();
		const registerGlobalSetter = (key, setter) => {
			let setters;
			if (!(setters = g[key])) setters = g[key] = [];
			setters.push(setter);
			return (v) => {
				if (setters.length > 1) setters.forEach((set) => set(v));
				else setters[0](v);
			};
		};
		internalSetCurrentInstance = registerGlobalSetter(`__VUE_INSTANCE_SETTERS__`, (v) => currentInstance = v);
		setInSSRSetupState = registerGlobalSetter(`__VUE_SSR_SETTERS__`, (v) => isInSSRComponentSetup = v);
	}
	var setCurrentInstance = (instance) => {
		const prev = currentInstance;
		internalSetCurrentInstance(instance);
		instance.scope.on();
		return () => {
			instance.scope.off();
			internalSetCurrentInstance(prev);
		};
	};
	var unsetCurrentInstance = () => {
		currentInstance && currentInstance.scope.off();
		internalSetCurrentInstance(null);
	};
	function isStatefulComponent(instance) {
		return instance.vnode.shapeFlag & 4;
	}
	var isInSSRComponentSetup = false;
	function setupComponent(instance, isSSR = false, optimized = false) {
		isSSR && setInSSRSetupState(isSSR);
		const { props, children } = instance.vnode;
		const isStateful = isStatefulComponent(instance);
		initProps(instance, props, isStateful, isSSR);
		initSlots(instance, children, optimized || isSSR);
		const setupResult = isStateful ? setupStatefulComponent(instance, isSSR) : void 0;
		isSSR && setInSSRSetupState(false);
		return setupResult;
	}
	function setupStatefulComponent(instance, isSSR) {
		const Component = instance.type;
		instance.accessCache = /* @__PURE__ */ Object.create(null);
		instance.proxy = new Proxy(instance.ctx, PublicInstanceProxyHandlers);
		const { setup } = Component;
		if (setup) {
			pauseTracking();
			const setupContext = instance.setupContext = setup.length > 1 ? createSetupContext(instance) : null;
			const reset = setCurrentInstance(instance);
			const setupResult = callWithErrorHandling(setup, instance, 0, [instance.props, setupContext]);
			const isAsyncSetup = isPromise(setupResult);
			resetTracking();
			reset();
			if ((isAsyncSetup || instance.sp) && !isAsyncWrapper(instance)) markAsyncBoundary(instance);
			if (isAsyncSetup) {
				setupResult.then(unsetCurrentInstance, unsetCurrentInstance);
				if (isSSR) return setupResult.then((resolvedResult) => {
					handleSetupResult(instance, resolvedResult, isSSR);
				}).catch((e) => {
					handleError(e, instance, 0);
				});
				else instance.asyncDep = setupResult;
			} else handleSetupResult(instance, setupResult, isSSR);
		} else finishComponentSetup(instance, isSSR);
	}
	function handleSetupResult(instance, setupResult, isSSR) {
		if (isFunction(setupResult)) if (instance.type.__ssrInlineRender) instance.ssrRender = setupResult;
		else instance.render = setupResult;
		else if (isObject(setupResult)) instance.setupState = proxyRefs(setupResult);
		finishComponentSetup(instance, isSSR);
	}
	var compile;
	var installWithProxy;
	function finishComponentSetup(instance, isSSR, skipOptions) {
		const Component = instance.type;
		if (!instance.render) {
			if (!isSSR && compile && !Component.render) {
				const template = Component.template || resolveMergedOptions(instance).template;
				if (template) {
					const { isCustomElement, compilerOptions } = instance.appContext.config;
					const { delimiters, compilerOptions: componentCompilerOptions } = Component;
					Component.render = compile(template, extend(extend({
						isCustomElement,
						delimiters
					}, compilerOptions), componentCompilerOptions));
				}
			}
			instance.render = Component.render || NOOP;
			if (installWithProxy) installWithProxy(instance);
		}
		{
			const reset = setCurrentInstance(instance);
			pauseTracking();
			try {
				applyOptions(instance);
			} finally {
				resetTracking();
				reset();
			}
		}
	}
	var attrsProxyHandlers = { get(target, key) {
		track(target, "get", "");
		return target[key];
	} };
	function createSetupContext(instance) {
		const expose = (exposed) => {
			instance.exposed = exposed || {};
		};
		return {
			attrs: new Proxy(instance.attrs, attrsProxyHandlers),
			slots: instance.slots,
			emit: instance.emit,
			expose
		};
	}
	function getComponentPublicInstance(instance) {
		if (instance.exposed) return instance.exposeProxy || (instance.exposeProxy = new Proxy(proxyRefs(markRaw(instance.exposed)), {
			get(target, key) {
				if (key in target) return target[key];
				else if (key in publicPropertiesMap) return publicPropertiesMap[key](instance);
			},
			has(target, key) {
				return key in target || key in publicPropertiesMap;
			}
		}));
		else return instance.proxy;
	}
	function isClassComponent(value) {
		return isFunction(value) && "__vccOpts" in value;
	}
	var computed = (getterOrOptions, debugOptions) => {
		return /* @__PURE__ */ computed$1(getterOrOptions, debugOptions, isInSSRComponentSetup);
	};
	var version = "3.5.39";
	//#endregion
	//#region ../../../node_modules/.pnpm/@vue+runtime-dom@3.5.39/node_modules/@vue/runtime-dom/dist/runtime-dom.esm-bundler.js
	/**
	* @vue/runtime-dom v3.5.39
	* (c) 2018-present Yuxi (Evan) You and Vue contributors
	* @license MIT
	**/
	var policy = void 0;
	var tt = typeof window !== "undefined" && window.trustedTypes;
	if (tt) try {
		policy = /* @__PURE__ */ tt.createPolicy("vue", { createHTML: (val) => val });
	} catch (e) {}
	var unsafeToTrustedHTML = policy ? (val) => policy.createHTML(val) : (val) => val;
	var svgNS = "http://www.w3.org/2000/svg";
	var mathmlNS = "http://www.w3.org/1998/Math/MathML";
	var doc = typeof document !== "undefined" ? document : null;
	var templateContainer = doc && /* @__PURE__ */ doc.createElement("template");
	var nodeOps = {
		insert: (child, parent, anchor) => {
			parent.insertBefore(child, anchor || null);
		},
		remove: (child) => {
			const parent = child.parentNode;
			if (parent) parent.removeChild(child);
		},
		createElement: (tag, namespace, is, props) => {
			const el = namespace === "svg" ? doc.createElementNS(svgNS, tag) : namespace === "mathml" ? doc.createElementNS(mathmlNS, tag) : is ? doc.createElement(tag, { is }) : doc.createElement(tag);
			if (tag === "select" && props && props.multiple != null) el.setAttribute("multiple", props.multiple);
			return el;
		},
		createText: (text) => doc.createTextNode(text),
		createComment: (text) => doc.createComment(text),
		setText: (node, text) => {
			node.nodeValue = text;
		},
		setElementText: (el, text) => {
			el.textContent = text;
		},
		parentNode: (node) => node.parentNode,
		nextSibling: (node) => node.nextSibling,
		querySelector: (selector) => doc.querySelector(selector),
		setScopeId(el, id) {
			el.setAttribute(id, "");
		},
		insertStaticContent(content, parent, anchor, namespace, start, end) {
			const before = anchor ? anchor.previousSibling : parent.lastChild;
			if (start && (start === end || start.nextSibling)) while (true) {
				parent.insertBefore(start.cloneNode(true), anchor);
				if (start === end || !(start = start.nextSibling)) break;
			}
			else {
				templateContainer.innerHTML = unsafeToTrustedHTML(namespace === "svg" ? `<svg>${content}</svg>` : namespace === "mathml" ? `<math>${content}</math>` : content);
				const template = templateContainer.content;
				if (namespace === "svg" || namespace === "mathml") {
					const wrapper = template.firstChild;
					while (wrapper.firstChild) template.appendChild(wrapper.firstChild);
					template.removeChild(wrapper);
				}
				parent.insertBefore(template, anchor);
			}
			return [before ? before.nextSibling : parent.firstChild, anchor ? anchor.previousSibling : parent.lastChild];
		}
	};
	var vtcKey = /* @__PURE__ */ Symbol("_vtc");
	function patchClass(el, value, isSVG) {
		const transitionClasses = el[vtcKey];
		if (transitionClasses) value = (value ? [value, ...transitionClasses] : [...transitionClasses]).join(" ");
		if (value == null) el.removeAttribute("class");
		else if (isSVG) el.setAttribute("class", value);
		else el.className = value;
	}
	var vShowOriginalDisplay = /* @__PURE__ */ Symbol("_vod");
	var vShowHidden = /* @__PURE__ */ Symbol("_vsh");
	var CSS_VAR_TEXT = /* @__PURE__ */ Symbol("");
	var displayRE = /(?:^|;)\s*display\s*:/;
	function patchStyle(el, prev, next) {
		const style = el.style;
		const isCssString = isString(next);
		let hasControlledDisplay = false;
		if (next && !isCssString) {
			if (prev) if (!isString(prev)) {
				for (const key in prev) if (next[key] == null) setStyle(style, key, "");
			} else for (const prevStyle of prev.split(";")) {
				const key = prevStyle.slice(0, prevStyle.indexOf(":")).trim();
				if (next[key] == null) setStyle(style, key, "");
			}
			for (const key in next) {
				if (key === "display") hasControlledDisplay = true;
				const value = next[key];
				if (value != null) {
					if (!shouldPreserveTextareaResizeStyle(el, key, !isString(prev) && prev ? prev[key] : void 0, value)) setStyle(style, key, value);
				} else setStyle(style, key, "");
			}
		} else if (isCssString) {
			if (prev !== next) {
				const cssVarText = style[CSS_VAR_TEXT];
				if (cssVarText) next += ";" + cssVarText;
				style.cssText = next;
				hasControlledDisplay = displayRE.test(next);
			}
		} else if (prev) el.removeAttribute("style");
		if (vShowOriginalDisplay in el) {
			el[vShowOriginalDisplay] = hasControlledDisplay ? style.display : "";
			if (el[vShowHidden]) style.display = "none";
		}
	}
	var importantRE = /\s*!important$/;
	function setStyle(style, name, val) {
		if (isArray(val)) val.forEach((v) => setStyle(style, name, v));
		else {
			if (val == null) val = "";
			if (name.startsWith("--")) style.setProperty(name, val);
			else {
				const prefixed = autoPrefix(style, name);
				if (importantRE.test(val)) style.setProperty(hyphenate(prefixed), val.replace(importantRE, ""), "important");
				else style[prefixed] = val;
			}
		}
	}
	var prefixes = [
		"Webkit",
		"Moz",
		"ms"
	];
	var prefixCache = {};
	function autoPrefix(style, rawName) {
		const cached = prefixCache[rawName];
		if (cached) return cached;
		let name = camelize(rawName);
		if (name !== "filter" && name in style) return prefixCache[rawName] = name;
		name = capitalize(name);
		for (let i = 0; i < prefixes.length; i++) {
			const prefixed = prefixes[i] + name;
			if (prefixed in style) return prefixCache[rawName] = prefixed;
		}
		return rawName;
	}
	function shouldPreserveTextareaResizeStyle(el, key, prev, next) {
		return el.tagName === "TEXTAREA" && (key === "width" || key === "height") && isString(next) && prev === next;
	}
	var xlinkNS = "http://www.w3.org/1999/xlink";
	function patchAttr(el, key, value, isSVG, instance, isBoolean = isSpecialBooleanAttr(key)) {
		if (isSVG && key.startsWith("xlink:")) if (value == null) el.removeAttributeNS(xlinkNS, key.slice(6, key.length));
		else el.setAttributeNS(xlinkNS, key, value);
		else if (value == null || isBoolean && !includeBooleanAttr(value)) el.removeAttribute(key);
		else el.setAttribute(key, isBoolean ? "" : isSymbol(value) ? String(value) : value);
	}
	function patchDOMProp(el, key, value, parentComponent, attrName) {
		if (key === "innerHTML" || key === "textContent") {
			if (value != null) el[key] = key === "innerHTML" ? unsafeToTrustedHTML(value) : value;
			return;
		}
		const tag = el.tagName;
		if (key === "value" && tag !== "PROGRESS" && !tag.includes("-")) {
			const oldValue = tag === "OPTION" ? el.getAttribute("value") || "" : el.value;
			const newValue = value == null ? el.type === "checkbox" ? "on" : "" : String(value);
			if (oldValue !== newValue || !("_value" in el)) el.value = newValue;
			if (value == null) el.removeAttribute(key);
			el._value = value;
			return;
		}
		let needRemove = false;
		if (value === "" || value == null) {
			const type = typeof el[key];
			if (type === "boolean") value = includeBooleanAttr(value);
			else if (value == null && type === "string") {
				value = "";
				needRemove = true;
			} else if (type === "number") {
				value = 0;
				needRemove = true;
			}
		}
		try {
			el[key] = value;
		} catch (e) {}
		needRemove && el.removeAttribute(attrName || key);
	}
	function addEventListener(el, event, handler, options) {
		el.addEventListener(event, handler, options);
	}
	function removeEventListener(el, event, handler, options) {
		el.removeEventListener(event, handler, options);
	}
	var veiKey = /* @__PURE__ */ Symbol("_vei");
	function patchEvent(el, rawName, prevValue, nextValue, instance = null) {
		const invokers = el[veiKey] || (el[veiKey] = {});
		const existingInvoker = invokers[rawName];
		if (nextValue && existingInvoker) existingInvoker.value = nextValue;
		else {
			const [name, options] = parseName(rawName);
			if (nextValue) addEventListener(el, name, invokers[rawName] = createInvoker(nextValue, instance), options);
			else if (existingInvoker) {
				removeEventListener(el, name, existingInvoker, options);
				invokers[rawName] = void 0;
			}
		}
	}
	var optionsModifierRE = /(Once|Passive|Capture)$/;
	var optionsModifierEventRE = /^on:?(?:Once|Passive|Capture)$/;
	function parseName(name) {
		let options;
		let m;
		while ((m = name.match(optionsModifierRE)) && !optionsModifierEventRE.test(name)) {
			if (!options) options = {};
			name = name.slice(0, name.length - m[1].length);
			options[m[1].toLowerCase()] = true;
		}
		return [name[2] === ":" ? name.slice(3) : hyphenate(name.slice(2)), options];
	}
	var cachedNow = 0;
	var p = /* @__PURE__ */ Promise.resolve();
	var getNow = () => cachedNow || (p.then(() => cachedNow = 0), cachedNow = Date.now());
	function createInvoker(initialValue, instance) {
		const invoker = (e) => {
			if (!e._vts) e._vts = Date.now();
			else if (e._vts <= invoker.attached) return;
			const value = invoker.value;
			if (isArray(value)) {
				const originalStop = e.stopImmediatePropagation;
				e.stopImmediatePropagation = () => {
					originalStop.call(e);
					e._stopped = true;
				};
				const handlers = value.slice();
				const args = [e];
				for (let i = 0; i < handlers.length; i++) {
					if (e._stopped) break;
					const handler = handlers[i];
					if (handler) callWithAsyncErrorHandling(handler, instance, 5, args);
				}
			} else callWithAsyncErrorHandling(value, instance, 5, [e]);
		};
		invoker.value = initialValue;
		invoker.attached = getNow();
		return invoker;
	}
	var isNativeOn = (key) => key.charCodeAt(0) === 111 && key.charCodeAt(1) === 110 && key.charCodeAt(2) > 96 && key.charCodeAt(2) < 123;
	var patchProp = (el, key, prevValue, nextValue, namespace, parentComponent) => {
		const isSVG = namespace === "svg";
		if (key === "class") patchClass(el, nextValue, isSVG);
		else if (key === "style") patchStyle(el, prevValue, nextValue);
		else if (isOn(key)) {
			if (!isModelListener(key)) patchEvent(el, key, prevValue, nextValue, parentComponent);
		} else if (key[0] === "." ? (key = key.slice(1), true) : key[0] === "^" ? (key = key.slice(1), false) : shouldSetAsProp(el, key, nextValue, isSVG)) {
			patchDOMProp(el, key, nextValue);
			if (!el.tagName.includes("-") && (key === "value" || key === "checked" || key === "selected")) patchAttr(el, key, nextValue, isSVG, parentComponent, key !== "value");
		} else if (el._isVueCE && (shouldSetAsPropForVueCE(el, key) || el._def.__asyncLoader && (/[A-Z]/.test(key) || !isString(nextValue)))) patchDOMProp(el, camelize(key), nextValue, parentComponent, key);
		else {
			if (key === "true-value") el._trueValue = nextValue;
			else if (key === "false-value") el._falseValue = nextValue;
			patchAttr(el, key, nextValue, isSVG);
		}
	};
	function shouldSetAsProp(el, key, value, isSVG) {
		if (isSVG) {
			if (key === "innerHTML" || key === "textContent") return true;
			if (key in el && isNativeOn(key) && isFunction(value)) return true;
			return false;
		}
		if (key === "spellcheck" || key === "draggable" || key === "translate" || key === "autocorrect") return false;
		if (key === "sandbox" && el.tagName === "IFRAME") return false;
		if (key === "form") return false;
		if (key === "list" && el.tagName === "INPUT") return false;
		if (key === "type" && el.tagName === "TEXTAREA") return false;
		if (key === "width" || key === "height") {
			const tag = el.tagName;
			if (tag === "IMG" || tag === "VIDEO" || tag === "CANVAS" || tag === "SOURCE") return false;
		}
		if (isNativeOn(key) && isString(value)) return false;
		return key in el;
	}
	function shouldSetAsPropForVueCE(el, key) {
		const props = el._def.props;
		if (!props) return false;
		const camelKey = camelize(key);
		return Array.isArray(props) ? props.some((prop) => camelize(prop) === camelKey) : Object.keys(props).some((prop) => camelize(prop) === camelKey);
	}
	var getModelAssigner = (vnode) => {
		const fn = vnode.props["onUpdate:modelValue"] || false;
		return isArray(fn) ? (value) => invokeArrayFns(fn, value) : fn;
	};
	function onCompositionStart(e) {
		e.target.composing = true;
	}
	function onCompositionEnd(e) {
		const target = e.target;
		if (target.composing) {
			target.composing = false;
			target.dispatchEvent(new Event("input"));
		}
	}
	var assignKey = /* @__PURE__ */ Symbol("_assign");
	function castValue(value, trim, number) {
		if (trim) value = value.trim();
		if (number) value = looseToNumber(value);
		return value;
	}
	var vModelText = {
		created(el, { modifiers: { lazy, trim, number } }, vnode) {
			el[assignKey] = getModelAssigner(vnode);
			const castToNumber = number || vnode.props && vnode.props.type === "number";
			addEventListener(el, lazy ? "change" : "input", (e) => {
				if (e.target.composing) return;
				el[assignKey](castValue(el.value, trim, castToNumber));
			});
			if (trim || castToNumber) addEventListener(el, "change", () => {
				el.value = castValue(el.value, trim, castToNumber);
			});
			if (!lazy) {
				addEventListener(el, "compositionstart", onCompositionStart);
				addEventListener(el, "compositionend", onCompositionEnd);
				addEventListener(el, "change", onCompositionEnd);
			}
		},
		mounted(el, { value }) {
			el.value = value == null ? "" : value;
		},
		beforeUpdate(el, { value, oldValue, modifiers: { lazy, trim, number } }, vnode) {
			el[assignKey] = getModelAssigner(vnode);
			if (el.composing) return;
			const elValue = (number || el.type === "number") && !/^0\d/.test(el.value) ? looseToNumber(el.value) : el.value;
			const newValue = value == null ? "" : value;
			if (elValue === newValue) return;
			const rootNode = el.getRootNode();
			if ((rootNode instanceof Document || rootNode instanceof ShadowRoot) && rootNode.activeElement === el && el.type !== "range") {
				if (lazy && value === oldValue) return;
				if (trim && el.value.trim() === newValue) return;
			}
			el.value = newValue;
		}
	};
	var vModelCheckbox = {
		deep: true,
		created(el, _, vnode) {
			el[assignKey] = getModelAssigner(vnode);
			addEventListener(el, "change", () => {
				const modelValue = el._modelValue;
				const elementValue = getValue(el);
				const checked = el.checked;
				const assign = el[assignKey];
				if (isArray(modelValue)) {
					const index = looseIndexOf(modelValue, elementValue);
					const found = index !== -1;
					if (checked && !found) assign(modelValue.concat(elementValue));
					else if (!checked && found) {
						const filtered = [...modelValue];
						filtered.splice(index, 1);
						assign(filtered);
					}
				} else if (isSet(modelValue)) {
					const cloned = new Set(modelValue);
					if (checked) cloned.add(elementValue);
					else cloned.delete(elementValue);
					assign(cloned);
				} else assign(getCheckboxValue(el, checked));
			});
		},
		mounted: setChecked,
		beforeUpdate(el, binding, vnode) {
			el[assignKey] = getModelAssigner(vnode);
			setChecked(el, binding, vnode);
		}
	};
	function setChecked(el, { value, oldValue }, vnode) {
		el._modelValue = value;
		let checked;
		if (isArray(value)) checked = looseIndexOf(value, vnode.props.value) > -1;
		else if (isSet(value)) checked = value.has(vnode.props.value);
		else {
			if (value === oldValue) return;
			checked = looseEqual(value, getCheckboxValue(el, true));
		}
		if (el.checked !== checked) el.checked = checked;
	}
	var vModelSelect = {
		deep: true,
		created(el, { value, modifiers: { number } }, vnode) {
			const isSetModel = isSet(value);
			addEventListener(el, "change", () => {
				const selectedVal = Array.prototype.filter.call(el.options, (o) => o.selected).map((o) => number ? looseToNumber(getValue(o)) : getValue(o));
				el[assignKey](el.multiple ? isSetModel ? new Set(selectedVal) : selectedVal : selectedVal[0]);
				el._assigning = true;
				nextTick(() => {
					el._assigning = false;
				});
			});
			el[assignKey] = getModelAssigner(vnode);
		},
		mounted(el, { value }) {
			setSelected(el, value);
		},
		beforeUpdate(el, _binding, vnode) {
			el[assignKey] = getModelAssigner(vnode);
		},
		updated(el, { value }) {
			if (!el._assigning) setSelected(el, value);
		}
	};
	function setSelected(el, value) {
		const isMultiple = el.multiple;
		const isArrayValue = isArray(value);
		if (isMultiple && !isArrayValue && !isSet(value)) return;
		for (let i = 0, l = el.options.length; i < l; i++) {
			const option = el.options[i];
			const optionValue = getValue(option);
			if (isMultiple) if (isArrayValue) {
				const optionType = typeof optionValue;
				if (optionType === "string" || optionType === "number") option.selected = value.some((v) => String(v) === String(optionValue));
				else option.selected = looseIndexOf(value, optionValue) > -1;
			} else option.selected = value.has(optionValue);
			else if (looseEqual(getValue(option), value)) {
				if (el.selectedIndex !== i) el.selectedIndex = i;
				return;
			}
		}
		if (!isMultiple && el.selectedIndex !== -1) el.selectedIndex = -1;
	}
	function getValue(el) {
		return "_value" in el ? el._value : el.value;
	}
	function getCheckboxValue(el, checked) {
		const key = checked ? "_trueValue" : "_falseValue";
		return key in el ? el[key] : checked;
	}
	var systemModifiers = [
		"ctrl",
		"shift",
		"alt",
		"meta"
	];
	var modifierGuards = {
		stop: (e) => e.stopPropagation(),
		prevent: (e) => e.preventDefault(),
		self: (e) => e.target !== e.currentTarget,
		ctrl: (e) => !e.ctrlKey,
		shift: (e) => !e.shiftKey,
		alt: (e) => !e.altKey,
		meta: (e) => !e.metaKey,
		left: (e) => "button" in e && e.button !== 0,
		middle: (e) => "button" in e && e.button !== 1,
		right: (e) => "button" in e && e.button !== 2,
		exact: (e, modifiers) => systemModifiers.some((m) => e[`${m}Key`] && !modifiers.includes(m))
	};
	var withModifiers = (fn, modifiers) => {
		if (!fn) return fn;
		const cache = fn._withMods || (fn._withMods = {});
		const cacheKey = modifiers.join(".");
		return cache[cacheKey] || (cache[cacheKey] = ((event, ...args) => {
			for (let i = 0; i < modifiers.length; i++) {
				const guard = modifierGuards[modifiers[i]];
				if (guard && guard(event, modifiers)) return;
			}
			return fn(event, ...args);
		}));
	};
	var keyNames = {
		esc: "escape",
		space: " ",
		up: "arrow-up",
		left: "arrow-left",
		right: "arrow-right",
		down: "arrow-down",
		delete: "backspace"
	};
	var withKeys = (fn, modifiers) => {
		const cache = fn._withKeys || (fn._withKeys = {});
		const cacheKey = modifiers.join(".");
		return cache[cacheKey] || (cache[cacheKey] = ((event) => {
			if (!("key" in event)) return;
			const eventKey = hyphenate(event.key);
			if (modifiers.some((k) => k === eventKey || keyNames[k] === eventKey)) return fn(event);
		}));
	};
	var rendererOptions = /* @__PURE__ */ extend({ patchProp }, nodeOps);
	var renderer;
	function ensureRenderer() {
		return renderer || (renderer = createRenderer(rendererOptions));
	}
	var createApp = ((...args) => {
		const app = ensureRenderer().createApp(...args);
		const { mount } = app;
		app.mount = (containerOrSelector) => {
			const container = normalizeContainer(containerOrSelector);
			if (!container) return;
			const component = app._component;
			if (!isFunction(component) && !component.render && !component.template) component.template = container.innerHTML;
			if (container.nodeType === 1) container.textContent = "";
			const proxy = mount(container, false, resolveRootNamespace(container));
			if (container instanceof Element) {
				container.removeAttribute("v-cloak");
				container.setAttribute("data-v-app", "");
			}
			return proxy;
		};
		return app;
	});
	function resolveRootNamespace(container) {
		if (container instanceof SVGElement) return "svg";
		if (typeof MathMLElement === "function" && container instanceof MathMLElement) return "mathml";
	}
	function normalizeContainer(container) {
		if (isString(container)) return document.querySelector(container);
		return container;
	}
	//#endregion
	//#region src/utils.ts
	function byteLen(s) {
		if (s == null) return 0;
		const str = String(s);
		try {
			return unescape(encodeURIComponent(str)).length;
		} catch {
			return str.length;
		}
	}
	/** 计算字符串的 UTF-8 字节数组（用于 Hex 视图）。 */
	function toUtf8Bytes(s) {
		try {
			const esc = unescape(encodeURIComponent(s));
			const out = [];
			for (let i = 0; i < esc.length; i++) out.push(esc.charCodeAt(i) & 255);
			return out;
		} catch {
			const out = [];
			for (let i = 0; i < s.length; i++) out.push(s.charCodeAt(i) & 255);
			return out;
		}
	}
	/** glob 模式 → RegExp（支持 * ? [abc] [^a]）。 */
	function globToRegExp(pattern) {
		let out = "^";
		for (let i = 0; i < pattern.length; i++) {
			const c = pattern[i];
			if (c === "*") out += ".*";
			else if (c === "?") out += ".";
			else if (c === "[") {
				let j = i + 1;
				let neg = false;
				let cls = "";
				if (pattern[j] === "^" || pattern[j] === "!") {
					neg = true;
					j++;
				}
				while (j < pattern.length && pattern[j] !== "]") {
					cls += pattern[j];
					j++;
				}
				if (pattern[j] === "]") {
					i = j;
					out += "[" + (neg ? "^" : "") + cls + "]";
				} else out += "\\[";
			} else if (".+^${}()|\\/".indexOf(c) >= 0) out += "\\" + c;
			else out += c;
		}
		return new RegExp(out + "$");
	}
	function formatBytes(n) {
		if (n < 1024) return n + " B";
		if (n < 1024 * 1024) return (n / 1024).toFixed(1) + " KB";
		return (n / 1024 / 1024).toFixed(2) + " MB";
	}
	/** 把 "GET foo" / HSET user:1 name Alice 拆成参数表（支持双引号与转义）。 */
	function parseCommandLine(line) {
		const args = [];
		let cur = "";
		let q = null;
		let i = 0;
		while (i < line.length) {
			const c = line[i];
			if (q) {
				if (c === "\\" && i + 1 < line.length) {
					cur += line[i + 1];
					i += 2;
					continue;
				}
				if (c === q) {
					q = null;
					i++;
					continue;
				}
				cur += c;
				i++;
				continue;
			}
			if (c === "\"" || c === "'") {
				q = c;
				i++;
				continue;
			}
			if (c === " " || c === "	") {
				if (cur) {
					args.push(cur);
					cur = "";
				}
				i++;
				continue;
			}
			cur += c;
			i++;
		}
		if (cur) args.push(cur);
		return args;
	}
	function parseBound(s, def) {
		if (s == null) return def;
		let neg = false;
		if (s[0] === "(") {
			neg = true;
			s = s.substring(1);
		}
		const v = parseFloat(s);
		if (isNaN(v)) return def;
		return neg ? v + (v > 0 ? 1e-9 : -1e-9) : v;
	}
	function formatUptime(sec) {
		const d = Math.floor(sec / 86400);
		const h = Math.floor(sec % 86400 / 3600);
		const m = Math.floor(sec % 3600 / 60);
		if (d > 0) return `${d}天 ${h}时 ${m}分`;
		if (h > 0) return `${h}时 ${m}分`;
		return `${m}分`;
	}
	/**
	* 敏感数据脱敏：对命令行 / 审计明细中的密码、token、密钥类内容做掩码。
	* 命中规则：AUTH/CONFIG SET requirepass 等命令的密码参数；key 名含 pass/token/secret 的值。
	*/
	function maskSensitive(line) {
		let s = line.trim();
		const m = /^(AUTH|CONFIG\s+SET\s+requirepass)\b/i.exec(s);
		if (m) {
			s = m[0] + " ******";
			return s;
		}
		const parts = s.split(/\s+/);
		if (parts.length >= 2) {
			const key = parts[1];
			if (/pass|token|secret|pwd|password|密钥|口令/i.test(key)) return parts[0] + " " + key + " ******";
		}
		return line;
	}
	//#endregion
	//#region src/engine.ts
	function status(v) {
		return {
			t: "status",
			v
		};
	}
	function bulk(v) {
		return {
			t: "bulk",
			v: v === void 0 ? null : v
		};
	}
	function integer(v) {
		return {
			t: "int",
			v
		};
	}
	function array(v) {
		return {
			t: "array",
			v
		};
	}
	function error(v) {
		return {
			t: "error",
			v
		};
	}
	function newEntry(type) {
		let value;
		if (type === "list") value = [];
		else if (type === "hash" || type === "set" || type === "zset" || type === "stream") value = Object.create(null);
		else value = "";
		return {
			type,
			value,
			expireAt: null
		};
	}
	var MockEngine = class {
		constructor() {
			this.dbs = [];
			this.db = 0;
			this.startedAt = Date.now() - 1e3 * 60 * 60 * 26;
			this.lastStreamMs = 0;
			this.lastStreamSeq = 0;
			for (let i = 0; i < 16; i++) this.dbs.push({ keys: Object.create(null) });
		}
		cur() {
			return this.dbs[this.db];
		}
		now() {
			return Date.now();
		}
		expired(name) {
			const db = this.cur();
			const e = db.keys[name];
			if (!e) return true;
			if (e.expireAt != null && e.expireAt <= this.now()) {
				delete db.keys[name];
				return true;
			}
			return false;
		}
		ttlOf(name) {
			const e = this.cur().keys[name];
			if (!e) return -2;
			if (e.expireAt == null) return -1;
			return Math.max(0, Math.ceil((e.expireAt - this.now()) / 1e3));
		}
		sizeOf(e) {
			let n = 0;
			if (e.type === "string") n = byteLen(e.value);
			else if (e.type === "hash") for (const f in e.value) n += byteLen(f) + byteLen(e.value[f]);
			else if (e.type === "list") e.value.forEach((x) => n += byteLen(x));
			else if (e.type === "set") for (const k in e.value) n += byteLen(k);
			else if (e.type === "zset") for (const k in e.value) n += byteLen(k) + 8;
			else if (e.type === "stream") for (const id in e.value) {
				n += byteLen(id);
				for (const f in e.value[id]) n += byteLen(f) + byteLen(e.value[id][f]);
			}
			return n;
		}
		totalMemoryBytes() {
			let total = 0;
			for (const d of this.dbs) for (const k in d.keys) total += this.sizeOf(d.keys[k]);
			return total;
		}
		execute(args) {
			if (!args || !args.length) return error("ERR wrong number of arguments for (empty)");
			const cmd = String(args[0]).toUpperCase();
			const a = args.slice(1);
			try {
				return this.dispatch(cmd, a);
			} catch (e) {
				return error("ERR " + (e && e.message ? e.message : String(e)));
			}
		}
		dispatch(cmd, a) {
			const self = this;
			const db = () => self.cur();
			const requireArg = (i) => {
				if (a[i] == null) throw new Error(`wrong number of arguments for '${cmd}'`);
			};
			switch (cmd) {
				case "PING": return a.length ? bulk(a[0]) : status("PONG");
				case "ECHO": return a.length ? bulk(a[0]) : error("ERR wrong number of arguments for 'echo'");
				case "SELECT": {
					const idx = parseInt(a[0], 10);
					if (isNaN(idx) || idx < 0 || idx > 15) return error("ERR DB index is out of range");
					self.db = idx;
					return status("OK");
				}
				case "DBSIZE": return integer(Object.keys(db().keys).length);
				case "FLUSHDB":
					db().keys = Object.create(null);
					return status("OK");
				case "FLUSHALL":
					for (let i = 0; i < 16; i++) self.dbs[i].keys = Object.create(null);
					return status("OK");
				case "RANDOMKEY": {
					const ks = Object.keys(db().keys);
					return bulk(ks.length ? ks[Math.floor(Math.random() * ks.length)] : null);
				}
				case "EXISTS": {
					let c = 0;
					for (const k of a) if (!self.expired(k)) c++;
					return integer(c);
				}
				case "DEL": {
					let d = 0;
					for (const k of a) if (db().keys[k]) {
						delete db().keys[k];
						d++;
					}
					return integer(d);
				}
				case "TYPE":
					if (self.expired(a[0])) return status("none");
					return status(db().keys[a[0]].type);
				case "RENAME":
					if (self.expired(a[0])) return error("ERR no such key");
					if (a[0] === a[1]) return error("ERR source and destination objects are the same");
					db().keys[a[1]] = db().keys[a[0]];
					delete db().keys[a[0]];
					return status("OK");
				case "COPY": {
					if (self.expired(a[0])) return integer(0);
					const dest = a[1];
					const overwrite = a.slice(2).indexOf("REPLACE") >= 0;
					if (db().keys[dest] && !overwrite) return integer(0);
					db().keys[dest] = JSON.parse(JSON.stringify(db().keys[a[0]]));
					return integer(1);
				}
				case "TTL": return integer(self.ttlOf(a[0]));
				case "PTTL": {
					const e = db().keys[a[0]];
					if (!e) return integer(-2);
					if (e.expireAt == null) return integer(-1);
					return integer(Math.max(0, e.expireAt - self.now()));
				}
				case "EXPIRE": {
					if (self.expired(a[0])) return integer(0);
					const sec = parseInt(a[1], 10);
					if (isNaN(sec)) return error("ERR value is not an integer or out of range");
					db().keys[a[0]].expireAt = self.now() + sec * 1e3;
					return integer(1);
				}
				case "PERSIST":
					if (!db().keys[a[0]]) return integer(0);
					db().keys[a[0]].expireAt = null;
					return integer(1);
				case "KEYS": {
					const re = globToRegExp(a[0] || "*");
					return array(Object.keys(db().keys).filter((n) => !self.expired(n) && re.test(n)));
				}
				case "SCAN": {
					const cur = parseInt(a[0], 10) || 0;
					let match = "*";
					let count = 20;
					for (let s = 1; s < a.length; s++) {
						const up = String(a[s]).toUpperCase();
						if (up === "MATCH" && a[s + 1] != null) match = a[++s];
						else if (up === "COUNT" && a[s + 1] != null) count = parseInt(a[++s], 10) || 20;
					}
					const mre = globToRegExp(match);
					const names = Object.keys(db().keys).filter((n) => !self.expired(n) && mre.test(n)).sort();
					const end = Math.min(cur + count, names.length);
					const slice = names.slice(cur, end);
					return array([end < names.length ? String(end) : "0", slice]);
				}
				case "GET": return self.expired(a[0]) ? bulk(null) : bulk(db().keys[a[0]].value);
				case "SET": {
					let val = a[1];
					let ttl = null;
					for (let si = 2; si < a.length; si++) {
						const su = String(a[si]).toUpperCase();
						if (su === "EX" && a[si + 1] != null) ttl = parseInt(a[++si], 10) * 1e3;
						else if (su === "PX" && a[si + 1] != null) ttl = parseInt(a[++si], 10);
					}
					db().keys[a[0]] = {
						type: "string",
						value: val,
						expireAt: ttl ? self.now() + ttl : null
					};
					return status("OK");
				}
				case "MSET":
					for (let i = 0; i + 1 < a.length; i += 2) db().keys[a[i]] = {
						type: "string",
						value: a[i + 1],
						expireAt: null
					};
					return status("OK");
				case "APPEND": {
					let en = db().keys[a[0]];
					if (!en) en = db().keys[a[0]] = newEntry("string");
					en.value += a[1];
					return integer(byteLen(en.value));
				}
				case "STRLEN": return self.expired(a[0]) ? integer(0) : integer(byteLen(db().keys[a[0]].value));
				case "GETRANGE": {
					if (self.expired(a[0])) return bulk("");
					const str = db().keys[a[0]].value;
					const len = str.length;
					let s0 = parseInt(a[1], 10);
					let e0 = parseInt(a[2], 10);
					if (s0 < 0) s0 = len + s0;
					if (e0 < 0) e0 = len + e0;
					return bulk(str.substring(s0, e0 + 1));
				}
				case "SETRANGE": {
					let ee = db().keys[a[0]];
					if (!ee) ee = db().keys[a[0]] = newEntry("string");
					const off = parseInt(a[1], 10);
					let pre = ee.value.substring(0, off);
					while (pre.length < off) pre += "\0";
					ee.value = pre + a[2] + ee.value.substring(off + a[2].length);
					return integer(byteLen(ee.value));
				}
				case "INCR":
				case "INCRBY":
				case "DECR":
				case "DECRBY": {
					if (self.expired(a[0])) db().keys[a[0]] = newEntry("string");
					const cur2 = parseInt(db().keys[a[0]].value, 10);
					if (isNaN(cur2)) return error("ERR value is not an integer or out of range");
					const delta = cmd === "INCR" ? 1 : cmd === "DECR" ? -1 : parseInt(a[1], 10);
					if (isNaN(delta)) return error("ERR value is not an integer or out of range");
					const nv = cur2 + delta;
					db().keys[a[0]].value = String(nv);
					return integer(nv);
				}
				case "HSET": {
					let h = db().keys[a[0]];
					if (!h) h = db().keys[a[0]] = newEntry("hash");
					let added = 0;
					for (let hi = 1; hi + 1 < a.length; hi += 2) {
						if (h.value[a[hi]] === void 0) added++;
						h.value[a[hi]] = a[hi + 1];
					}
					return integer(added);
				}
				case "HSETNX": {
					let h2 = db().keys[a[0]];
					if (!h2) h2 = db().keys[a[0]] = newEntry("hash");
					if (h2.value[a[1]] !== void 0) return integer(0);
					h2.value[a[1]] = a[2];
					return integer(1);
				}
				case "HGET": return self.expired(a[0]) ? bulk(null) : bulk(a[1] in db().keys[a[0]].value ? db().keys[a[0]].value[a[1]] : null);
				case "HMGET": {
					if (self.expired(a[0])) return array(a.slice(1).map(() => null));
					const hm = db().keys[a[0]].value;
					return array(a.slice(1).map((f) => f in hm ? hm[f] : null));
				}
				case "HGETALL": {
					if (self.expired(a[0])) return array([]);
					const ha = db().keys[a[0]].value;
					const out = [];
					for (const fk in ha) {
						out.push(fk);
						out.push(ha[fk]);
					}
					return array(out);
				}
				case "HDEL": {
					if (self.expired(a[0])) return integer(0);
					const hd = db().keys[a[0]].value;
					let removed = 0;
					for (let d2 = 1; d2 < a.length; d2++) if (a[d2] in hd) {
						delete hd[a[d2]];
						removed++;
					}
					return integer(removed);
				}
				case "HLEN": return self.expired(a[0]) ? integer(0) : integer(Object.keys(db().keys[a[0]].value).length);
				case "HKEYS": return self.expired(a[0]) ? array([]) : array(Object.keys(db().keys[a[0]].value));
				case "HVALS": return self.expired(a[0]) ? array([]) : array(Object.keys(db().keys[a[0]].value).map((f) => db().keys[a[0]].value[f]));
				case "HEXISTS": return self.expired(a[0]) ? integer(0) : integer(a[1] in db().keys[a[0]].value ? 1 : 0);
				case "HINCRBY": {
					if (self.expired(a[0])) db().keys[a[0]] = newEntry("hash");
					const hv = db().keys[a[0]].value;
					const base = parseInt(hv[a[1]] || "0", 10);
					const inc = parseInt(a[2], 10);
					if (isNaN(base) || isNaN(inc)) return error("ERR value is not an integer or out of range");
					hv[a[1]] = String(base + inc);
					return integer(base + inc);
				}
				case "LPUSH":
				case "RPUSH": {
					let l = db().keys[a[0]];
					if (!l) l = db().keys[a[0]] = newEntry("list");
					for (let li = 1; li < a.length; li++) if (cmd === "LPUSH") l.value.unshift(a[li]);
					else l.value.push(a[li]);
					return integer(l.value.length);
				}
				case "LPOP":
				case "RPOP": {
					if (self.expired(a[0])) return bulk(null);
					const lv = db().keys[a[0]].value;
					if (!lv.length) return bulk(null);
					return bulk(cmd === "LPOP" ? lv.shift() : lv.pop());
				}
				case "LRANGE": {
					if (self.expired(a[0])) return array([]);
					const arr = db().keys[a[0]].value;
					const L = arr.length;
					let f0 = parseInt(a[1], 10);
					let t0 = parseInt(a[2], 10);
					if (f0 < 0) f0 = L + f0;
					if (t0 < 0) t0 = L + t0;
					return array(arr.slice(f0, t0 + 1));
				}
				case "LLEN": return self.expired(a[0]) ? integer(0) : integer(db().keys[a[0]].value.length);
				case "LINDEX": {
					if (self.expired(a[0])) return bulk(null);
					const la = db().keys[a[0]].value;
					let idx = parseInt(a[1], 10);
					if (idx < 0) idx = la.length + idx;
					return idx >= 0 && idx < la.length ? bulk(la[idx]) : bulk(null);
				}
				case "LSET": {
					if (self.expired(a[0])) return error("ERR no such key");
					const lb = db().keys[a[0]].value;
					let ix = parseInt(a[1], 10);
					if (ix < 0) ix = lb.length + ix;
					if (ix < 0 || ix >= lb.length) return error("ERR index out of range");
					lb[ix] = a[2];
					return status("OK");
				}
				case "LREM": {
					if (self.expired(a[0])) return integer(0);
					const lc = db().keys[a[0]].value;
					const cnt = parseInt(a[1], 10);
					const val3 = a[2];
					let removed2 = 0;
					const kept = [];
					for (let c = 0; c < lc.length; c++) {
						if (lc[c] === val3 && cnt !== 0 && (cnt > 0 ? removed2 < cnt : removed2 < -cnt)) {
							removed2++;
							continue;
						}
						kept.push(lc[c]);
					}
					db().keys[a[0]].value = kept;
					return integer(removed2);
				}
				case "LTRIM": {
					if (self.expired(a[0])) return status("OK");
					const lt = db().keys[a[0]].value;
					const n = lt.length;
					let s3 = parseInt(a[1], 10);
					let e3 = parseInt(a[2], 10);
					if (s3 < 0) s3 = n + s3;
					if (e3 < 0) e3 = n + e3;
					db().keys[a[0]].value = lt.slice(s3, e3 + 1);
					return status("OK");
				}
				case "SADD": {
					let st = db().keys[a[0]];
					if (!st) st = db().keys[a[0]] = newEntry("set");
					let add2 = 0;
					for (let ai = 1; ai < a.length; ai++) if (st.value[a[ai]] === void 0) {
						st.value[a[ai]] = 1;
						add2++;
					}
					return integer(add2);
				}
				case "SREM": {
					if (self.expired(a[0])) return integer(0);
					const sr = db().keys[a[0]].value;
					let rm = 0;
					for (let ri = 1; ri < a.length; ri++) if (sr[a[ri]] !== void 0) {
						delete sr[a[ri]];
						rm++;
					}
					return integer(rm);
				}
				case "SMEMBERS": return self.expired(a[0]) ? array([]) : array(Object.keys(db().keys[a[0]].value));
				case "SCARD": return self.expired(a[0]) ? integer(0) : integer(Object.keys(db().keys[a[0]].value).length);
				case "SISMEMBER": return self.expired(a[0]) ? integer(0) : integer(a[1] in db().keys[a[0]].value ? 1 : 0);
				case "ZADD": {
					let z = db().keys[a[0]];
					if (!z) z = db().keys[a[0]] = newEntry("zset");
					let added3 = 0;
					for (let zi = 1; zi + 1 < a.length; zi += 2) {
						const sc = parseFloat(a[zi]);
						if (isNaN(sc)) return error("ERR value is not a valid float");
						if (z.value[a[zi + 1]] === void 0) added3++;
						z.value[a[zi + 1]] = sc;
					}
					return integer(added3);
				}
				case "ZREM": {
					if (self.expired(a[0])) return integer(0);
					const zr = db().keys[a[0]].value;
					let rm3 = 0;
					for (let zri = 1; zri < a.length; zri++) if (zr[a[zri]] !== void 0) {
						delete zr[a[zri]];
						rm3++;
					}
					return integer(rm3);
				}
				case "ZCARD": return self.expired(a[0]) ? integer(0) : integer(Object.keys(db().keys[a[0]].value).length);
				case "ZSCORE": {
					if (self.expired(a[0])) return bulk(null);
					const zs = db().keys[a[0]].value;
					return a[1] in zs ? bulk(String(zs[a[1]])) : bulk(null);
				}
				case "ZRANGE": {
					if (self.expired(a[0])) return array([]);
					const za = db().keys[a[0]].value;
					const sorted = Object.keys(za).map((m) => [m, za[m]]).sort((x, y) => x[1] - y[1] || (x[0] < y[0] ? -1 : 1));
					let withScore = false;
					for (let w = 2; w < a.length; w++) if (String(a[w]).toUpperCase() === "WITHSCORES") withScore = true;
					let start = parseInt(a[1], 10);
					let stop = parseInt(a[2], 10);
					const N = sorted.length;
					if (start < 0) start = N + start;
					if (stop < 0) stop = N + stop;
					const slice3 = sorted.slice(start, stop + 1);
					const out3 = [];
					slice3.forEach((p) => {
						out3.push(p[0]);
						if (withScore) out3.push(String(p[1]));
					});
					return array(out3);
				}
				case "ZRANGEBYSCORE": {
					if (self.expired(a[0])) return array([]);
					const zb = db().keys[a[0]].value;
					const min = parseBound(a[1], -Infinity);
					const max = parseBound(a[2], Infinity);
					const out4 = [];
					Object.keys(zb).forEach((m) => {
						const sc = zb[m];
						if (sc >= min && sc <= max) out4.push([m, sc]);
					});
					out4.sort((x, y) => x[1] - y[1]);
					return array(out4.map((p) => p[0]));
				}
				case "XADD": {
					requireArg(0);
					let key = a[0];
					let i = 1;
					const upper = (s) => s.toUpperCase();
					if (upper(a[i]) === "NOMKSTREAM") i++;
					const idArg = a[i++];
					const e = db().keys[key] || (db().keys[key] = newEntry("stream"));
					if (e.type !== "stream") return error("ERR Existing key has wrong value type");
					const fields = {};
					for (; i + 1 < a.length; i += 2) fields[a[i]] = a[i + 1];
					const id = this.genStreamId(idArg);
					e.value[id] = fields;
					return bulk(id);
				}
				case "XLEN": return self.expired(a[0]) ? integer(0) : integer(Object.keys(db().keys[a[0]].value).length);
				case "XDEL": {
					if (self.expired(a[0])) return integer(0);
					const sv = db().keys[a[0]].value;
					let d = 0;
					for (let k = 1; k < a.length; k++) if (a[k] in sv) {
						delete sv[a[k]];
						d++;
					}
					return integer(d);
				}
				case "XRANGE":
				case "XREVRANGE": {
					if (self.expired(a[0])) return array([]);
					const sv = db().keys[a[0]].value;
					const ids = Object.keys(sv);
					let start = this.normStreamId(a[1]);
					let stop = this.normStreamId(a[2]);
					let count = 0;
					for (let s = 3; s < a.length; s++) if (a[s].toUpperCase() === "COUNT") count = parseInt(a[s + 1], 10) || 0;
					ids.sort();
					if (cmd === "XREVRANGE") {
						ids.reverse();
						const t = start;
						start = stop;
						stop = t;
					}
					const out = [];
					let n = 0;
					for (const id of ids) {
						if (id < start || id > stop) continue;
						const fv = sv[id];
						const flat = [id];
						for (const f in fv) {
							flat.push(f);
							flat.push(fv[f]);
						}
						out.push(flat);
						if (count && ++n >= count) break;
					}
					return array(out);
				}
				case "XREAD": {
					let count = 0;
					let i = 0;
					while (i < a.length && a[i].toUpperCase() !== "STREAMS") {
						if (a[i].toUpperCase() === "COUNT") count = parseInt(a[i + 1], 10) || 0;
						i++;
					}
					i++;
					const keys = [];
					while (i < a.length && a[i] !== ">") {
						if (a[i].indexOf("-") >= 0 || /^\d+$/.test(a[i])) break;
						keys.push(a[i]);
						i++;
					}
					const ids = a.slice(i);
					const out = [];
					keys.forEach((k, idx) => {
						if (self.expired(k)) return;
						const sv = db().keys[k].value;
						const all = Object.keys(sv).sort();
						const last = ids[idx];
						const from = last === "$" ? "" : last;
						const entries = all.filter((id) => from ? id > from : true);
						if (!entries.length) return;
						const slice = count ? entries.slice(0, count) : entries;
						const flat = [k];
						slice.forEach((id) => {
							const fv = sv[id];
							const e = [id];
							for (const f in fv) {
								e.push(f);
								e.push(fv[f]);
							}
							flat.push(e);
						});
						out.push(flat);
					});
					return out.length ? array(out) : bulk(null);
				}
				case "MEMORY":
					if (String(a[0]).toUpperCase() === "USAGE") return self.expired(a[1]) ? integer(0) : integer(self.sizeOf(db().keys[a[1]]));
					return error("ERR unknown subcommand");
				case "RCSIZE": return self.expired(a[0]) ? integer(0) : integer(self.sizeOf(db().keys[a[0]]));
				case "INFO": return bulk(this.synthesizeInfo());
				case "SLOWLOG": return this.slowlog(a);
				case "CLIENT":
					if (String(a[0]).toUpperCase() === "LIST") return bulk(this.synthesizeClientList());
					return error("ERR unknown subcommand");
				case "RCANALYZE": return bulk(this.analyzeMemory());
				case "RCSLOW": return bulk(this.analyzeSlow());
				case "RCCLUSTER": return bulk(this.clusterTopology());
				case "RCREPL": return bulk(this.replicationInfo());
				case "RCSENTINEL":
					if (String(a[0] || "MASTERS").toUpperCase() === "GET-MASTER-ADDR-BY-NAME" && a[1]) return bulk(this.sentinelGetMasterAddr(a[1]));
					return array(this.sentinelMasters());
				default: return error(`ERR unknown command '${cmd}'`);
			}
		}
		genStreamId(idArg) {
			if (!idArg || idArg === "*") {
				const ms = Date.now();
				if (ms === this.lastStreamMs) this.lastStreamSeq++;
				else {
					this.lastStreamMs = ms;
					this.lastStreamSeq = 0;
				}
				return `${ms}-${this.lastStreamSeq}`;
			}
			return idArg;
		}
		normStreamId(id) {
			if (!id || id === "-" || id === "+" || id === "$") return id === "-" ? "" : "9".repeat(24);
			return id;
		}
		keyExpiresCount() {
			let n = 0;
			for (const k in this.cur().keys) if (this.cur().keys[k].expireAt != null) n++;
			return n;
		}
		synthesizeInfo() {
			const used = this.totalMemoryBytes();
			const max = 256 * 1024 * 1024;
			const dbKeys = Object.keys(this.cur().keys).length;
			const expires = this.keyExpiresCount();
			const uptime = Math.floor((this.now() - this.startedAt) / 1e3);
			const ops = 800 + Math.floor(Math.sin(this.now() / 3e3) * 200) + Math.floor(Math.random() * 80);
			const lines = [];
			const section = (name, fields) => {
				lines.push(`# ${name}`);
				fields.forEach(([k, v]) => lines.push(`${k}:${v}`));
				lines.push("");
			};
			section("Server", [
				["redis_version", "7.2.0"],
				["redis_mode", "standalone"],
				["arch_bits", "64"],
				["uptime_in_seconds", uptime],
				["uptime_in_days", Math.floor(uptime / 86400)],
				["run_id", "mock_run_id_00000000000000000000000000000000"]
			]);
			section("Clients", [
				["connected_clients", 2],
				["blocked_clients", 0],
				["maxclients", 1e4]
			]);
			section("Memory", [
				["used_memory", used],
				["used_memory_human", (used / 1024 / 1024).toFixed(2) + "M"],
				["used_memory_rss", used + 1024 * 1024],
				["maxmemory", max],
				["maxmemory_policy", "noeviction"],
				["mem_fragmentation_ratio", "1.02"]
			]);
			section("Persistence", [
				["loading", 0],
				["rdb_changes_since_last_save", 12],
				["rdb_last_save_time", Math.floor(this.now() / 1e3) - 45],
				["aof_enabled", 0]
			]);
			section("Stats", [
				["total_connections_received", 18],
				["total_commands_processed", 12453],
				["instantaneous_ops_per_sec", ops],
				["expired_keys", 7],
				["evicted_keys", 0],
				["keyspace_hits", 9321],
				["keyspace_misses", 412]
			]);
			section("Replication", [["role", "master"], ["connected_slaves", 0]]);
			section("CPU", [["used_cpu_sys", (uptime * .02).toFixed(2)], ["used_cpu_user", (uptime * .05).toFixed(2)]]);
			section("Keyspace", [[`db${this.db}`, `keys=${dbKeys},expires=${expires},avg_ttl=${expires ? Math.floor(6e4 + Math.random() * 4e4) : 0}`]]);
			return lines.join("\n");
		}
		slowlog(a) {
			if (String(a[0] || "GET").toUpperCase() === "LEN") return integer(4);
			const cmds = [
				"KEYS *",
				"HGETALL user:session:large",
				"SORT mylist ALPHA",
				"LRANGE big:list 0 -1"
			];
			const out = [];
			for (let i = 0; i < 4; i++) out.push([
				i,
				Math.floor(this.now() / 1e3) - i * 137,
				1200 + i * 340,
				cmds[i].split(" ")
			]);
			return array(out);
		}
		synthesizeClientList() {
			return [
				"id=11 addr=127.0.0.1:51234 fd=8 name=web-app age=342 idle=0 flags=N db=0 sub=0 psub=0 multi=-1",
				"id=12 addr=10.0.0.7:48120 fd=9 name=worker-1 age=120 idle=27 flags=N db=2 sub=0 psub=0 multi=-1",
				"id=13 addr=127.0.0.1:51240 fd=10 name= age=12 idle=12 flags=S db=0 sub=0 psub=0 multi=-1"
			].join("\n");
		}
		analyzeMemory() {
			const dbKeys = this.cur().keys;
			const items = [];
			for (const name in dbKeys) {
				const ttl = this.ttlOf(name);
				if (ttl === -2) continue;
				items.push({
					name,
					type: dbKeys[name].type,
					size: this.sizeOf(dbKeys[name]),
					ttl
				});
			}
			const totalBytes = items.reduce((s, x) => s + x.size, 0);
			const totalKeys = items.length;
			const bigKeys = items.slice().sort((a, b) => b.size - a.size).slice(0, 20);
			const prefixMap = {};
			for (const it of items) {
				const p = it.name.includes(":") ? it.name.split(":")[0] : "(root)";
				const m = prefixMap[p] || (prefixMap[p] = {
					count: 0,
					size: 0
				});
				m.count++;
				m.size += it.size;
			}
			const prefixes = Object.keys(prefixMap).map((p) => ({
				prefix: p,
				count: prefixMap[p].count,
				size: prefixMap[p].size,
				pct: totalBytes ? Math.round(prefixMap[p].size / totalBytes * 100) : 0
			})).sort((a, b) => b.size - a.size);
			const typeMap = {};
			for (const it of items) {
				const m = typeMap[it.type] || (typeMap[it.type] = {
					count: 0,
					size: 0
				});
				m.count++;
				m.size += it.size;
			}
			return {
				totalBytes,
				totalKeys,
				bigKeys,
				prefixes,
				types: Object.keys(typeMap).map((t) => ({
					type: t,
					count: typeMap[t].count,
					size: typeMap[t].size,
					pct: totalBytes ? Math.round(typeMap[t].size / totalBytes * 100) : 0
				})).sort((a, b) => b.size - a.size)
			};
		}
		analyzeSlow() {
			const templates = [
				"KEYS *",
				"HGETALL user:session:large",
				"SORT mylist ALPHA",
				"LRANGE big:list 0 -1",
				"SMEMBERS tags:redis",
				"ZRANGE leaderboard 0 -1 WITHSCORES",
				"HGETALL user:1",
				"GET app:config",
				"XREAD COUNT 100 STREAMS stream:events $",
				"MGET cache:hit cache:miss",
				"HSCAN user:session:large 0 COUNT 500",
				"DEBUG SLEEP 1"
			];
			const durations = [
				14500,
				8200,
				6300,
				12400,
				2100,
				9800,
				1500,
				300,
				4500,
				800,
				7600,
				19e3
			];
			const now = Math.floor(this.now() / 1e3);
			const items = templates.map((cmd, i) => ({
				id: 100 - i,
				timestamp: now - i * 173,
				durationUs: durations[i],
				command: cmd,
				pattern: this.normalizeSlowCommand(cmd)
			}));
			const durs = items.map((x) => x.durationUs).sort((a, b) => a - b);
			const count = items.length;
			const maxUs = durs[durs.length - 1];
			const minUs = durs[0];
			const avgUs = Math.round(durs.reduce((s, x) => s + x, 0) / count);
			const p95Us = durs[Math.min(durs.length - 1, Math.ceil(.95 * durs.length) - 1)];
			const byCmdMap = {};
			for (const it of items) {
				const g = byCmdMap[it.pattern] || (byCmdMap[it.pattern] = {
					count: 0,
					totalUs: 0,
					maxUs: 0
				});
				g.count++;
				g.totalUs += it.durationUs;
				g.maxUs = Math.max(g.maxUs, it.durationUs);
			}
			return {
				items,
				count,
				maxUs,
				minUs,
				avgUs,
				p95Us,
				byCommand: Object.keys(byCmdMap).map((p) => ({
					pattern: p,
					count: byCmdMap[p].count,
					totalUs: byCmdMap[p].totalUs,
					maxUs: byCmdMap[p].maxUs
				})).sort((a, b) => b.totalUs - a.totalUs)
			};
		}
		normalizeSlowCommand(cmd) {
			const parts = cmd.split(/\s+/);
			if (parts.length === 1) return parts[0];
			const tail = parts.slice(1).map((token) => {
				if (token === "*" || token === "$" || /^-?\d+$/.test(token)) return token;
				if (token.includes(":")) return token.split(":")[0] + ":*";
				return token;
			});
			return parts[0] + " " + tail.join(" ");
		}
		clusterTopology() {
			const slotRanges = [
				"0-5460",
				"5461-10922",
				"10923-16383"
			];
			const addrs = [
				"10.0.0.11:6379",
				"10.0.0.12:6379",
				"10.0.0.13:6379"
			];
			const nodes = [];
			const myselfId = "a1b2c3d4e5f60001";
			for (let i = 0; i < 3; i++) {
				nodes.push({
					id: "a1b2c3d4e5f6000" + (i + 1),
					role: i === 0 ? "myself" : "master",
					addr: addrs[i],
					flags: "master",
					masterId: null,
					slots: slotRanges[i],
					linkState: "connected"
				});
				nodes.push({
					id: "b" + (i + 1) + "c3d4e5f6000" + (i + 1),
					role: "slave",
					addr: "10.0.0.2" + (i + 1) + ":6379",
					flags: "slave",
					masterId: "a1b2c3d4e5f6000" + (i + 1),
					slots: "-",
					linkState: "connected"
				});
			}
			return {
				mode: "cluster",
				nodeCount: nodes.length,
				slotsCovered: 16384,
				myselfId,
				nodes
			};
		}
		replicationInfo() {
			return {
				role: "master",
				connectedSlaves: 2,
				masterReplOffset: 38940123,
				slaves: [{
					id: "slave0",
					addr: "10.0.0.21:6379",
					offset: 38940098,
					lag: 0,
					state: "online"
				}, {
					id: "slave1",
					addr: "10.0.0.22:6379",
					offset: 38939950,
					lag: 1,
					state: "online"
				}]
			};
		}
		sentinelMasters() {
			return [{
				name: "mymaster",
				ip: "10.0.0.11",
				port: 6379,
				flags: "master",
				numSlaves: 2,
				numOtherSentinels: 2,
				quorum: 2
			}];
		}
		sentinelGetMasterAddr(_name) {
			return "10.0.0.11:6379";
		}
		loadSeedDemo() {
			[
				[
					"SET",
					"app:name",
					"BackendDevTools"
				],
				[
					"SET",
					"app:config",
					"{\"theme\":\"dark\",\"lang\":\"zh\",\"features\":[\"redis\",\"ssh\"]}"
				],
				[
					"SET",
					"greeting",
					"hello redis"
				],
				[
					"SET",
					"big:text",
					this.bigText()
				],
				[
					"HSET",
					"user:1",
					"name",
					"Alice",
					"age",
					"30",
					"role",
					"admin"
				],
				[
					"HSET",
					"user:2",
					"name",
					"Bob",
					"age",
					"25",
					"role",
					"dev"
				],
				[
					"LPUSH",
					"queue:tasks",
					"task-3",
					"task-2",
					"task-1"
				],
				[
					"RPUSH",
					"list:logs",
					"line-1",
					"line-2",
					"line-3",
					"line-4",
					"line-5"
				],
				[
					"SADD",
					"tags:redis",
					"cache",
					"nosql",
					"kv",
					"pubsub"
				],
				[
					"ZADD",
					"leaderboard",
					"100",
					"player:A",
					"250",
					"player:B",
					"180",
					"player:C"
				],
				[
					"XADD",
					"stream:events",
					"*",
					"event",
					"login",
					"user",
					"alice"
				],
				[
					"XADD",
					"stream:events",
					"*",
					"event",
					"click",
					"page",
					"home"
				],
				[
					"SET",
					"session:abc",
					"token-xyz",
					"EX",
					"120"
				]
			].forEach((c) => this.execute(c));
			this.execute([
				"SET",
				"big:string",
				this.bigText()
			]);
			const bigHash = ["HSET", "big:hash"];
			for (let i = 0; i < 60; i++) bigHash.push("field" + i, "value-" + i + "-" + "x".repeat(24));
			this.execute(bigHash);
			const bigList = ["RPUSH", "big:list"];
			for (let i = 0; i < 200; i++) bigList.push("item-" + i + "-" + "y".repeat(16));
			this.execute(bigList);
			const bigZ = ["ZADD", "big:zset"];
			for (let i = 0; i < 40; i++) bigZ.push(String(i), "member-" + i);
			this.execute(bigZ);
			for (let i = 0; i < 12; i++) this.execute([
				"SET",
				"session:u" + i,
				"tok-" + "a".repeat(24)
			]);
			for (let i = 0; i < 8; i++) this.execute([
				"SET",
				"cache:item:" + i,
				"v" + i + "-" + "z".repeat(32)
			]);
			this.db = 1;
			this.execute([
				"SET",
				"cache:hit",
				"42"
			]);
			this.db = 0;
		}
		bigText() {
			const lines = [];
			for (let i = 0; i < 400; i++) lines.push(`line ${i + 1}: The quick brown fox jumps over the lazy dog. 中文测试内容 ${i + 1}`);
			return lines.join("\n");
		}
	};
	//#endregion
	//#region src/diff.ts
	/** 基于 LCS 的两组行差异，返回带标记的行序列（用于并排/统一展示）。 */
	function calcLineDiff(aLines, bLines) {
		const n = aLines.length;
		const m = bLines.length;
		const dp = Array.from({ length: n + 1 }, () => new Array(m + 1).fill(0));
		for (let i = n - 1; i >= 0; i--) for (let j = m - 1; j >= 0; j--) if (aLines[i] === bLines[j]) dp[i][j] = dp[i + 1][j + 1] + 1;
		else dp[i][j] = Math.max(dp[i + 1][j], dp[i][j + 1]);
		const out = [];
		let i = 0;
		let j = 0;
		while (i < n && j < m) if (aLines[i] === bLines[j]) {
			out.push({
				kind: "eq",
				text: aLines[i]
			});
			i++;
			j++;
		} else if (dp[i + 1][j] >= dp[i][j + 1]) {
			out.push({
				kind: "del",
				text: aLines[i]
			});
			i++;
		} else {
			out.push({
				kind: "add",
				text: bLines[j]
			});
			j++;
		}
		while (i < n) out.push({
			kind: "del",
			text: aLines[i++]
		});
		while (j < m) out.push({
			kind: "add",
			text: bLines[j++]
		});
		return out;
	}
	/** 递归比较两棵 JSON 对象/数组，返回按路径的差异列表。 */
	function calcJsonDiff(a, b, base = "") {
		const out = [];
		const keyOf = (k) => base ? base + "." + k : String(k);
		if (Array.isArray(a) && Array.isArray(b)) {
			const len = Math.max(a.length, b.length);
			for (let idx = 0; idx < len; idx++) {
				const ka = keyOf(idx);
				if (idx >= a.length) out.push({
					path: ka,
					type: "added",
					to: b[idx]
				});
				else if (idx >= b.length) out.push({
					path: ka,
					type: "removed",
					from: a[idx]
				});
				else if (!shallowEqual(a[idx], b[idx])) if (isContainer(a[idx]) && isContainer(b[idx])) out.push(...calcJsonDiff(a[idx], b[idx], ka));
				else out.push({
					path: ka,
					type: "changed",
					from: a[idx],
					to: b[idx]
				});
			}
			return out;
		}
		if (isContainer(a) && isContainer(b)) {
			const keys = /* @__PURE__ */ new Set([...Object.keys(a), ...Object.keys(b)]);
			for (const k of keys) {
				const kb = keyOf(k);
				if (!(k in a)) out.push({
					path: kb,
					type: "added",
					to: b[k]
				});
				else if (!(k in b)) out.push({
					path: kb,
					type: "removed",
					from: a[k]
				});
				else if (!shallowEqual(a[k], b[k])) if (isContainer(a[k]) && isContainer(b[k])) out.push(...calcJsonDiff(a[k], b[k], kb));
				else out.push({
					path: kb,
					type: "changed",
					from: a[k],
					to: b[k]
				});
			}
			return out;
		}
		if (a !== b) out.push({
			path: base || "(root)",
			type: "changed",
			from: a,
			to: b
		});
		return out;
	}
	function isContainer(v) {
		return v !== null && typeof v === "object";
	}
	function shallowEqual(a, b) {
		if (a === b) return true;
		return JSON.stringify(a) === JSON.stringify(b);
	}
	//#endregion
	//#region src/client.ts
	function unwrap(reply) {
		if (!reply) return null;
		switch (reply.t) {
			case "status": return reply.v;
			case "bulk": return reply.v;
			case "int": return reply.v;
			case "array": return reply.v || [];
			case "error": throw new Error(reply.v);
			default: return reply.v;
		}
	}
	var RedisClient = class {
		constructor(transport) {
			this.transport = transport;
		}
		connect(cfg) {
			return this.transport.connect(cfg);
		}
		disconnect() {
			this.transport.disconnect();
		}
		isConnected() {
			return this.transport.isConnected();
		}
		cmd(...args) {
			return this.transport.exec(args).then(unwrap);
		}
		execRaw(args) {
			return this.transport.exec(args);
		}
		select(db) {
			return this.cmd("SELECT", String(db));
		}
		dbsize() {
			return this.cmd("DBSIZE");
		}
		flushdb() {
			return this.cmd("FLUSHDB");
		}
		async scanKeysWithMeta(opts = {}) {
			const args = ["SCAN", String(opts.cursor || "0")];
			if (opts.match) args.push("MATCH", opts.match);
			args.push("COUNT", String(opts.count || 100));
			const raw = await this.transport.exec(args).then(unwrap);
			const names = raw[1] || [];
			const keys = await Promise.all(names.map(async (n) => {
				const [type, ttl, size] = await Promise.all([
					this.type(n).catch(() => "none"),
					this.ttl(n).catch(() => -2),
					this.keyMemory(n).catch(() => 0)
				]);
				return {
					name: n,
					type,
					ttl,
					size
				};
			}));
			return {
				cursor: raw[0],
				keys
			};
		}
		type(k) {
			return this.cmd("TYPE", k);
		}
		ttl(k) {
			return this.cmd("TTL", k);
		}
		keyMemory(k) {
			return this.cmd("RCSIZE", k).catch(() => this.cmd("MEMORY", "USAGE", k));
		}
		exists(k) {
			return this.cmd("EXISTS", k);
		}
		del(...keys) {
			return this.cmd("DEL", ...keys);
		}
		rename(k1, k2) {
			return this.cmd("RENAME", k1, k2);
		}
		copy(k1, k2, replace = false) {
			return this.cmd("COPY", k1, k2, ...replace ? ["REPLACE"] : []);
		}
		expire(k, sec) {
			return this.cmd("EXPIRE", k, String(sec));
		}
		persist(k) {
			return this.cmd("PERSIST", k);
		}
		mset(items) {
			const args = [];
			for (const k in items) args.push(k, items[k]);
			return this.cmd("MSET", ...args);
		}
		get(k) {
			return this.cmd("GET", k);
		}
		set(k, v, exSec) {
			if (exSec) return this.cmd("SET", k, v, "EX", String(exSec));
			return this.cmd("SET", k, v);
		}
		append(k, v) {
			return this.cmd("APPEND", k, v);
		}
		strlen(k) {
			return this.cmd("STRLEN", k);
		}
		async hgetall(k) {
			const flat = await this.cmd("HGETALL", k);
			const o = {};
			for (let i = 0; i + 1 < flat.length; i += 2) o[flat[i]] = flat[i + 1];
			return o;
		}
		hset(k, field, v) {
			return this.cmd("HSET", k, field, v);
		}
		hdel(k, field) {
			return this.cmd("HDEL", k, field);
		}
		hlen(k) {
			return this.cmd("HLEN", k);
		}
		lrange(k, s, e) {
			return this.cmd("LRANGE", k, String(s), String(e));
		}
		lpush(k, v) {
			return this.cmd("LPUSH", k, v);
		}
		rpush(k, v) {
			return this.cmd("RPUSH", k, v);
		}
		lset(k, idx, v) {
			return this.cmd("LSET", k, String(idx), v);
		}
		lrem(k, val) {
			return this.cmd("LREM", k, "1", val);
		}
		llen(k) {
			return this.cmd("LLEN", k);
		}
		lindex(k, idx) {
			return this.cmd("LINDEX", k, String(idx));
		}
		smembers(k) {
			return this.cmd("SMEMBERS", k);
		}
		sadd(k, v) {
			return this.cmd("SADD", k, v);
		}
		srem(k, v) {
			return this.cmd("SREM", k, v);
		}
		scard(k) {
			return this.cmd("SCARD", k);
		}
		async zrangeAll(k) {
			const flat = await this.cmd("ZRANGE", k, "0", "-1", "WITHSCORES");
			const out = [];
			for (let i = 0; i + 1 < flat.length; i += 2) out.push({
				member: flat[i],
				score: parseFloat(flat[i + 1])
			});
			return out;
		}
		zadd(k, member, score) {
			return this.cmd("ZADD", k, String(score), member);
		}
		zrem(k, member) {
			return this.cmd("ZREM", k, member);
		}
		zcard(k) {
			return this.cmd("ZCARD", k);
		}
		zscore(k, member) {
			return this.cmd("ZSCORE", k, member);
		}
		xadd(k, fields, id = "*") {
			const args = [
				"XADD",
				k,
				id
			];
			for (const f in fields) args.push(f, fields[f]);
			return this.cmd(...args);
		}
		xlen(k) {
			return this.cmd("XLEN", k);
		}
		async xrange(k, start, end, count) {
			const args = [
				"XRANGE",
				k,
				start,
				end
			];
			if (count) args.push("COUNT", String(count));
			const raw = await this.cmd(...args);
			return this.parseStream(raw);
		}
		async xrevrange(k, end, start, count) {
			const args = [
				"XREVRANGE",
				k,
				end,
				start
			];
			if (count) args.push("COUNT", String(count));
			const raw = await this.cmd(...args);
			return this.parseStream(raw);
		}
		xdel(k, id) {
			return this.cmd("XDEL", k, id);
		}
		parseStream(raw) {
			const out = [];
			for (const item of raw) {
				const id = item[0];
				const fields = {};
				for (let i = 1; i + 1 < item.length; i += 2) fields[item[i]] = item[i + 1];
				out.push({
					id,
					fields
				});
			}
			return out;
		}
		async info() {
			return this.cmd("INFO");
		}
		async slowlog(n = 10) {
			return (await this.cmd("SLOWLOG", "GET", String(n)) || []).map((e) => ({
				id: e[0],
				timestamp: e[1],
				durationUs: e[2],
				command: Array.isArray(e[3]) ? e[3].join(" ") : String(e[3])
			}));
		}
		async clientList() {
			const text = await this.cmd("CLIENT", "LIST");
			return String(text || "").split("\n").filter((l) => l.trim()).map((line) => {
				const o = {};
				line.split(" ").forEach((kv) => {
					const i = kv.indexOf("=");
					if (i >= 0) o[kv.slice(0, i)] = kv.slice(i + 1);
				});
				return {
					id: parseInt(o.id || "0", 10),
					addr: o.addr || "",
					name: o.name || "",
					age: parseInt(o.age || "0", 10),
					idle: parseInt(o.idle || "0", 10),
					db: parseInt(o.db || "0", 10),
					flags: o.flags || ""
				};
			});
		}
		async batchDelete(keys) {
			if (!keys.length) return 0;
			return this.del(...keys);
		}
		async batchExpire(keys, sec) {
			let ok = 0;
			for (const k of keys) if (await this.expire(k, sec).catch(() => 0)) ok++;
			return ok;
		}
		async batchPersist(keys) {
			let ok = 0;
			for (const k of keys) if (await this.persist(k).catch(() => 0)) ok++;
			return ok;
		}
		async exportKeys(keys, fmt) {
			const items = [];
			for (const k of keys) {
				const type = await this.type(k).catch(() => "none");
				let value = null;
				if (type === "string") value = await this.get(k);
				else if (type === "hash") value = await this.hgetall(k);
				else if (type === "list") value = await this.lrange(k, 0, -1);
				else if (type === "set") value = await this.smembers(k);
				else if (type === "zset") value = await this.zrangeAll(k);
				else if (type === "stream") value = await this.xrange(k, "-", "+");
				const ttl = await this.ttl(k).catch(() => -1);
				items.push({
					key: k,
					type,
					ttl,
					value
				});
			}
			if (fmt === "json") return JSON.stringify(items, null, 2);
			if (fmt === "csv") return "key,type,ttl,value_json\n" + items.map((it) => `${csvCell(it.key)},${it.type},${it.ttl},"${String(it.value ?? "").replace(/"/g, "\"\"")}"`).join("\n");
			const lines = [];
			for (const it of items) {
				if (it.ttl > 0) lines.push(`# TTL ${it.ttl}s`);
				if (it.type === "string") lines.push(`SET ${it.key} ${quoteArg(it.value ?? "")}${it.ttl > 0 ? " EX " + it.ttl : ""}`);
				else if (it.type === "hash") {
					const fv = [];
					for (const f in it.value) fv.push(quoteArg(f), quoteArg(it.value[f]));
					lines.push(`HSET ${it.key} ${fv.join(" ")}`);
				} else if (it.type === "list") for (const v of it.value) lines.push(`RPUSH ${it.key} ${quoteArg(v)}`);
				else if (it.type === "set") for (const v of it.value) lines.push(`SADD ${it.key} ${quoteArg(v)}`);
				else if (it.type === "zset") for (const m of it.value) lines.push(`ZADD ${it.key} ${m.score} ${quoteArg(m.member)}`);
				else if (it.type === "stream") for (const e of it.value) {
					const fv = [];
					for (const f in e.fields) fv.push(quoteArg(f), quoteArg(e.fields[f]));
					lines.push(`XADD ${it.key} ${e.id} ${fv.join(" ")}`);
				}
			}
			return lines.join("\n");
		}
		async importKeys(text, fmt) {
			const errors = [];
			let count = 0;
			try {
				if (fmt === "json") {
					const items = JSON.parse(text);
					for (const it of items) try {
						await this.writeItem(it.key, it.type, it.value, it.ttl);
						count++;
					} catch (e) {
						errors.push(`${it.key}: ${e.message}`);
					}
				} else if (fmt === "csv") {
					const rows = text.trim().split("\n").slice(1);
					for (const row of rows) {
						const parts = splitCsv(row);
						if (parts.length < 4) continue;
						const [key, type, ttl, valueJson] = parts;
						try {
							const value = JSON.parse(valueJson);
							await this.writeItem(key, type, value, parseInt(ttl, 10));
							count++;
						} catch (e) {
							errors.push(`${key}: ${e.message}`);
						}
					}
				} else for (const line of text.split("\n")) {
					const t = line.trim();
					if (!t || t.startsWith("#")) continue;
					const args = parseCommandLine(t);
					if (!args.length) continue;
					try {
						await this.transport.exec(args);
						count++;
					} catch (e) {
						errors.push(`${t}: ${e.message}`);
					}
				}
			} catch (e) {
				errors.push("解析失败: " + e.message);
			}
			return {
				count,
				errors
			};
		}
		async writeItem(key, type, value, ttl) {
			if (type === "string") await this.set(key, String(value ?? ""), ttl && ttl > 0 ? ttl : void 0);
			else if (type === "hash") for (const f in value) await this.hset(key, f, String(value[f]));
			else if (type === "list") for (const v of value) await this.rpush(key, String(v));
			else if (type === "set") for (const v of value) await this.sadd(key, String(v));
			else if (type === "zset") for (const m of value) await this.zadd(key, m.member, m.score);
			else if (type === "stream") for (const e of value) await this.xadd(key, e.fields, e.id);
			if (ttl && ttl > 0) await this.expire(key, ttl);
		}
		async readValue(k, fmt) {
			const raw = await this.get(k);
			if (raw == null) return {
				text: "",
				ok: true
			};
			if (fmt === "hex") return {
				text: toUtf8Bytes(raw).map((b) => b.toString(16).padStart(2, "0")).join(" "),
				ok: true
			};
			if (fmt === "json") try {
				return {
					text: JSON.stringify(JSON.parse(raw), null, 2),
					ok: true
				};
			} catch (e) {
				return {
					text: raw,
					ok: false,
					error: e.message
				};
			}
			return {
				text: raw,
				ok: true
			};
		}
		async diffValues(a, b) {
			const [ra, rb] = await Promise.all([this.get(a), this.get(b)]);
			const va = ra == null ? "" : ra;
			const vb = rb == null ? "" : rb;
			let ja = null;
			let jb = null;
			try {
				ja = JSON.parse(va);
				jb = JSON.parse(vb);
			} catch {}
			if (ja !== null && jb !== null) return {
				mode: "json",
				changes: calcJsonDiff(ja, jb)
			};
			return {
				mode: "text",
				lines: calcLineDiff(va.split("\n"), vb.split("\n"))
			};
		}
		async memoryAnalyze() {
			return this.cmd("RCANALYZE");
		}
		async slowQueryAnalysis(n = 20) {
			return this.cmd("RCSLOW", String(n));
		}
		async clusterTopology() {
			return this.cmd("RCCLUSTER");
		}
		async replicationInfo() {
			return this.cmd("RCREPL");
		}
		async sentinelMasters() {
			return this.cmd("RCSENTINEL", "MASTERS");
		}
		async sentinelGetMasterAddr(name) {
			return this.cmd("RCSENTINEL", "GET-MASTER-ADDR-BY-NAME", name);
		}
		executeLine(line) {
			const args = parseCommandLine(line.trim());
			if (!args.length) return Promise.resolve({
				kind: "info",
				text: ""
			});
			return this.transport.exec(args).then((reply) => {
				return {
					kind: reply && reply.t === "error" ? "err" : "ok",
					text: formatReply(reply),
					raw: reply
				};
			}).catch((e) => ({
				kind: "err",
				text: e && e.message ? e.message : String(e)
			}));
		}
	};
	function quoteArg(s) {
		if (/^[\w\-.:/@]+$/.test(s)) return s;
		return "\"" + s.replace(/\\/g, "\\\\").replace(/"/g, "\\\"") + "\"";
	}
	function csvCell(s) {
		return /[",\n]/.test(s) ? "\"" + s.replace(/"/g, "\"\"") + "\"" : s;
	}
	function splitCsv(row) {
		const out = [];
		let cur = "";
		let q = false;
		for (let i = 0; i < row.length; i++) {
			const c = row[i];
			if (q) if (c === "\"") if (row[i + 1] === "\"") {
				cur += "\"";
				i++;
			} else q = false;
			else cur += c;
			else if (c === "\"") q = true;
			else if (c === ",") {
				out.push(cur);
				cur = "";
			} else cur += c;
		}
		out.push(cur);
		return out;
	}
	function formatReply(reply) {
		if (!reply) return "(nil)";
		switch (reply.t) {
			case "status": return "+" + reply.v;
			case "bulk":
				if (reply.v === null) return "(nil)";
				return tryPretty(reply.v);
			case "int": return "(integer) " + reply.v;
			case "array":
				if (!reply.v || !reply.v.length) return "(empty array)";
				return reply.v.map((x, i) => {
					if (x && x.t === "bulk") return i + 1 + ") " + (x.v === null ? "(nil)" : tryPretty(x.v));
					if (x && typeof x === "object") return i + 1 + ") " + JSON.stringify(x);
					return i + 1 + ") " + (x === null ? "(nil)" : String(x));
				}).join("\n");
			case "error": return "(error) " + reply.v;
			default: return String(reply.v);
		}
	}
	function tryPretty(s) {
		if (typeof s !== "string") return String(s);
		const t = s.trim();
		if (t[0] === "{" && t[t.length - 1] === "}" || t[0] === "[" && t[t.length - 1] === "]") try {
			return JSON.stringify(JSON.parse(t), null, 2);
		} catch {
			return s;
		}
		return s;
	}
	var COMMANDS = [
		"PING",
		"ECHO",
		"SELECT",
		"DBSIZE",
		"FLUSHDB",
		"FLUSHALL",
		"RANDOMKEY",
		"EXISTS",
		"DEL",
		"TYPE",
		"RENAME",
		"COPY",
		"TTL",
		"PTTL",
		"EXPIRE",
		"PERSIST",
		"KEYS",
		"SCAN",
		"GET",
		"SET",
		"MSET",
		"APPEND",
		"STRLEN",
		"GETRANGE",
		"SETRANGE",
		"INCR",
		"INCRBY",
		"DECR",
		"DECRBY",
		"HSET",
		"HSETNX",
		"HGET",
		"HMGET",
		"HGETALL",
		"HDEL",
		"HLEN",
		"HKEYS",
		"HVALS",
		"HEXISTS",
		"HINCRBY",
		"LPUSH",
		"RPUSH",
		"LPOP",
		"RPOP",
		"LRANGE",
		"LLEN",
		"LINDEX",
		"LSET",
		"LREM",
		"LTRIM",
		"SADD",
		"SREM",
		"SMEMBERS",
		"SCARD",
		"SISMEMBER",
		"ZADD",
		"ZREM",
		"ZCARD",
		"ZSCORE",
		"ZRANGE",
		"ZRANGEBYSCORE",
		"XADD",
		"XLEN",
		"XDEL",
		"XRANGE",
		"XREVRANGE",
		"XREAD",
		"INFO",
		"SLOWLOG",
		"CLIENT",
		"MEMORY",
		"RCSIZE"
	];
	function completeCommand(prefix) {
		const p = (prefix || "").toUpperCase();
		return COMMANDS.filter((c) => c.indexOf(p) === 0);
	}
	var Bridge = class {
		constructor() {
			this.runtimeWasm = null;
			this.seq = 0;
			this.pending = {};
			this.listeners = {
				theme: [],
				ready: []
			};
		}
		post(msg) {
			msg.v = 1;
			parent.postMessage(msg, "*");
		}
		call(func, input) {
			return new Promise((resolve, reject) => {
				const reqId = ++this.seq;
				this.pending[reqId] = {
					resolve,
					reject
				};
				this.post({
					kind: "call",
					reqId,
					func,
					input
				});
			});
		}
		on(evt, fn) {
			(this.listeners[evt] || (this.listeners[evt] = [])).push(fn);
		}
		handleMessage(data) {
			if (!data || data.v !== 1) return;
			if (data.kind === "theme") {
				this.runtimeWasm = data.wasm === true;
				this.emit("theme", data);
				return;
			}
			if (typeof data.reqId === "number") {
				const p = this.pending[data.reqId];
				if (!p) return;
				delete this.pending[data.reqId];
				if (data.ok) p.resolve(data.data);
				else p.reject(new Error(data.error));
			}
		}
		ready() {
			this.post({ kind: "ready" });
		}
		emit(evt, payload) {
			(this.listeners[evt] || []).forEach((f) => f(payload));
		}
	};
	var bridge = new Bridge();
	var MemoryPersistence = class {
		constructor() {
			this.m = {};
		}
		get(k) {
			return this.m[k];
		}
		set(k, v) {
			this.m[k] = v;
		}
		remove(k) {
			delete this.m[k];
		}
		keys() {
			return Object.keys(this.m);
		}
	};
	var persistence = new MemoryPersistence();
	/**
	* 预留接口标记工具：任何「计划后期实现」的扩展点都抛这个错误，便于全局检索。
	* 后期直接删除 throw 并实现逻辑即可。
	*/
	function reserved(name, detail) {
		const msg = `[redis-client] 预留接口尚未实现: ${name}${detail ? " — " + detail : ""}`;
		if (typeof console !== "undefined" && console.warn) console.warn(msg);
		const err = new Error(msg);
		err.reserved = true;
		err.feature = name;
		return err;
	}
	//#endregion
	//#region src/transport.ts
	var RedisTransport = class {};
	var MockRedisTransport = class extends RedisTransport {
		constructor(..._args) {
			super(..._args);
			this.engine = null;
			this.connected = false;
		}
		async connect(cfg) {
			this.engine = new MockEngine();
			if (cfg.seed) this.engine.loadSeedDemo();
			this.connected = true;
			return {
				ok: true,
				mode: "mock",
				id: cfg.id || "mock"
			};
		}
		disconnect() {
			this.engine = null;
			this.connected = false;
		}
		isConnected() {
			return this.connected;
		}
		exec(args) {
			return new Promise((resolve, reject) => {
				if (!this.connected || !this.engine) return reject(/* @__PURE__ */ new Error("未连接到 Redis（mock 模式请先「连接」）"));
				try {
					resolve(this.engine.execute(args));
				} catch (e) {
					reject(e);
				}
			});
		}
	};
	var WasmRedisTransport = class extends RedisTransport {
		constructor(..._args2) {
			super(..._args2);
			this.connId = null;
			this.connected = false;
		}
		/**
		* 调一次 WASM 后端动作，解出统一信封 `{ ok, message, data }`；`ok !== true` 即抛错。
		* 返回的是信封里的 `data`（形状与 MockEngine 一致：exec 是 Reply，分析是分析对象）。
		*/
		async call(action, payload = {}) {
			const res = await bridge.call("handle", {
				action,
				connId: this.connId,
				...payload
			});
			const env = JSON.parse(res);
			if (!env || env.ok !== true) throw new Error(env && env.message ? env.message : "WASM 后端返回异常");
			return env.data;
		}
		async connect(cfg) {
			this.connId = cfg.id;
			const data = await this.call("connect", {
				host: cfg.host,
				port: cfg.port,
				password: cfg.password || "",
				db: cfg.db ?? 0
			});
			this.connected = !!(data && data.ok === true);
			return {
				ok: this.connected,
				mode: "tcp",
				id: cfg.id
			};
		}
		disconnect() {
			if (this.connId) this.call("disconnect").catch(() => void 0);
			this.connected = false;
			this.connId = null;
		}
		isConnected() {
			return this.connected;
		}
		async exec(args) {
			if (!this.connected || !this.connId) throw new Error("未连接到 Redis（tcp 模式请先「连接」）");
			const head = (args[0] || "").toUpperCase();
			let action = "exec";
			let payload = { args };
			if (head === "RCANALYZE") {
				action = "analyze_memory";
				payload = {};
			} else if (head === "RCSLOW") {
				action = "slowlog";
				payload = { count: parseInt(args[1] || "20", 10) };
			} else if (head === "RCCLUSTER") {
				action = "cluster";
				payload = {};
			} else if (head === "RCREPL") {
				action = "replication";
				payload = {};
			} else if (head === "RCSENTINEL") {
				action = "sentinel";
				payload = {
					sub: args[1] || "MASTERS",
					name: args[2] || ""
				};
			}
			return await this.call(action, payload);
		}
	};
	var ProxyRedisTransport = class extends RedisTransport {
		constructor(..._args3) {
			super(..._args3);
			this.connected = false;
		}
		async connect(_cfg) {
			throw reserved("ProxyRedisTransport.connect", "需 wasm 后端 + net:<proxy-host> 权限，访问本地 RESP→HTTP 代理");
		}
		disconnect() {
			this.connected = false;
		}
		isConnected() {
			return this.connected;
		}
		exec() {
			throw reserved("ProxyRedisTransport.exec", "见 ProxyRedisTransport.connect 注释");
		}
	};
	var transports = {
		mock: () => new MockRedisTransport(),
		tcp: () => new WasmRedisTransport(),
		proxy: () => new ProxyRedisTransport()
	};
	//#endregion
	//#region src/store.ts
	var client = null;
	var connSeq = 0;
	var store = /* @__PURE__ */ reactive({
		connections: [],
		activeConnId: null,
		connected: false,
		currentDb: 0,
		dbCounts: [],
		search: "",
		typeFilter: "",
		groupBy: true,
		keys: [],
		scanCursor: "0",
		hasMoreKeys: false,
		loadingKeys: false,
		selectedKey: null,
		keyInfo: null,
		keyLoadedAt: 0,
		keyLoadError: null,
		selectedKeys: [],
		terminalLines: [],
		commandHistory: [],
		historyIndex: -1,
		monitorText: "",
		monitorLoading: false,
		toast: null,
		dialog: null,
		nowTick: 0,
		mockMode: true,
		audit: []
	});
	function saveConnections() {
		persistence.set("rc.connections", JSON.stringify(store.connections));
	}
	function loadConnections() {
		const raw = persistence.get("rc.connections");
		if (raw) try {
			store.connections = JSON.parse(raw);
		} catch {
			store.connections = [];
		}
		const active = persistence.get("rc.active");
		if (active && store.connections.some((c) => c.id === active)) store.activeConnId = active;
		if (!store.connections.length) seedDefaultConnection();
	}
	function seedDefaultConnection() {
		const conn = {
			id: "local-mock",
			name: "本地 Mock",
			host: "127.0.0.1",
			port: 6379,
			db: 0,
			mode: "mock",
			seed: true
		};
		store.connections = [conn];
		store.activeConnId = conn.id;
		saveConnections();
	}
	function ensureInit() {
		if (store.connections.length || persistence.get("rc.connections")) loadConnections();
		else loadConnections();
		loadAudit();
		setInterval(() => {
			store.nowTick++;
		}, 1e3);
	}
	function loadAudit() {
		const raw = persistence.get("rc.audit");
		if (raw) try {
			store.audit = JSON.parse(raw);
		} catch {
			store.audit = [];
		}
	}
	function recordAudit(op, detail, target = "-", status = "ok") {
		const entry = {
			ts: Date.now(),
			op,
			detail,
			target,
			status
		};
		store.audit.unshift(entry);
		if (store.audit.length > 500) store.audit.length = 500;
		persistence.set("rc.audit", JSON.stringify(store.audit));
	}
	function clearAudit() {
		store.audit = [];
		persistence.set("rc.audit", JSON.stringify(store.audit));
		toast("审计日志已清空", "ok");
	}
	function exportAudit(fmt = "json") {
		if (fmt === "csv") return "ts,op,target,status,detail\n" + store.audit.map((e) => {
			const t = new Date(e.ts).toISOString();
			const d = e.detail.replace(/"/g, "\"\"").replace(/\n/g, " ");
			return `"${t}",${e.op},"${e.target}",${e.status},"${d}"`;
		}).join("\n");
		return JSON.stringify(store.audit, null, 2);
	}
	async function connectActive() {
		if (!store.activeConnId) return;
		await connectById(store.activeConnId);
	}
	async function connectById(id) {
		const cfg = store.connections.find((c) => c.id === id);
		if (!cfg) return;
		if (client) client.disconnect();
		client = new RedisClient(transports[cfg.mode]());
		store.mockMode = cfg.mode === "mock";
		await client.connect(cfg);
		store.connected = client.isConnected();
		store.activeConnId = id;
		store.currentDb = cfg.db || 0;
		persistence.set("rc.active", id);
		await afterConnect();
	}
	async function afterConnect() {
		store.selectedKey = null;
		store.keyInfo = null;
		store.keys = [];
		store.selectedKeys = [];
		await Promise.all([loadDbCounts(), loadKeys(true)]);
	}
	function addConnection(cfg) {
		store.connections.push(cfg);
		saveConnections();
	}
	function updateConnection(cfg) {
		const i = store.connections.findIndex((c) => c.id === cfg.id);
		if (i >= 0) store.connections[i] = cfg;
		saveConnections();
	}
	function deleteConnection(id) {
		store.connections = store.connections.filter((c) => c.id !== id);
		if (store.activeConnId === id) {
			store.activeConnId = store.connections[0]?.id || null;
			store.connected = false;
		}
		saveConnections();
	}
	function newConnId() {
		return "conn-" + Date.now().toString(36) + "-" + connSeq++;
	}
	async function loadDbCounts() {
		if (!client) return;
		const counts = [];
		for (let db = 0; db < 16; db++) {
			await client.select(db);
			counts.push(await client.dbsize());
		}
		await client.select(store.currentDb);
		store.dbCounts = counts;
	}
	async function selectDb(db) {
		if (!client) return;
		store.currentDb = db;
		await client.select(db);
		store.keys = [];
		store.scanCursor = "0";
		await loadKeys(true);
	}
	async function loadKeys(reset) {
		if (!client) return;
		store.loadingKeys = true;
		try {
			if (reset) {
				store.keys = [];
				store.scanCursor = "0";
			}
			const res = await client.scanKeysWithMeta({
				cursor: store.scanCursor,
				match: store.search || "*",
				count: 200
			});
			let merged = reset ? res.keys : store.keys.concat(res.keys);
			if (store.typeFilter) merged = merged.filter((k) => k.type === store.typeFilter);
			store.keys = merged;
			store.scanCursor = res.cursor;
			store.hasMoreKeys = res.cursor !== "0";
		} finally {
			store.loadingKeys = false;
		}
	}
	async function loadMoreKeys() {
		if (store.loadingKeys || !store.hasMoreKeys) return;
		await loadKeys(false);
	}
	async function refreshKeys() {
		await loadDbCounts();
		await loadKeys(true);
	}
	function setSearch(v) {
		store.search = v;
		loadKeys(true);
	}
	function setTypeFilter(v) {
		store.typeFilter = v;
		loadKeys(true);
	}
	function setGroupBy(v) {
		store.groupBy = v;
	}
	function selectKey(key) {
		store.selectedKey = key;
		reloadKeyInfo();
	}
	async function reloadKeyInfo() {
		if (!client || !store.selectedKey) return;
		const k = store.selectedKey;
		store.keyLoadError = null;
		try {
			const [type, ttl, size] = await Promise.all([
				client.type(k),
				client.ttl(k),
				client.keyMemory(k)
			]);
			if (store.selectedKey !== k) return;
			store.keyInfo = {
				type,
				ttl,
				size
			};
			store.keyLoadedAt = Date.now();
		} catch (e) {
			if (store.selectedKey === k) {
				store.keyInfo = null;
				store.keyLoadError = e.message;
			}
		}
	}
	function toggleSelect(key) {
		const i = store.selectedKeys.indexOf(key);
		if (i >= 0) store.selectedKeys.splice(i, 1);
		else store.selectedKeys.push(key);
	}
	function clearSelection() {
		store.selectedKeys = [];
	}
	async function runCommand(line) {
		if (!client || !store.connected) {
			pushTerminal("err", "(未连接) 请先连接 Redis");
			return;
		}
		pushTerminal("cmd", "redis> " + line);
		const r = await client.executeLine(line);
		pushTerminal(r.kind, r.text);
		recordAudit("CMD", maskSensitive(line), deriveTarget(line), r.kind === "err" ? "err" : "ok");
		const lc = line.trim().toUpperCase().split(/\s+/)[0];
		if ([
			"SET",
			"DEL",
			"HSET",
			"LPUSH",
			"RPUSH",
			"SADD",
			"ZADD",
			"XADD",
			"RENAME",
			"COPY",
			"EXPIRE",
			"PERSIST",
			"FLUSHDB",
			"FLUSHALL",
			"HDEL",
			"LREM",
			"SREM",
			"ZREM",
			"XDEL"
		].includes(lc)) refreshKeys();
	}
	function deriveTarget(line) {
		const parts = line.trim().split(/\s+/);
		const cmd = parts[0].toUpperCase();
		if (cmd === "FLUSHDB" || cmd === "FLUSHALL") return "(all)";
		if (parts[1]) return parts[1];
		return "-";
	}
	function pushTerminal(kind, text) {
		store.terminalLines.push({
			kind,
			text
		});
		if (store.terminalLines.length > 500) store.terminalLines.shift();
	}
	function pushHistory(line) {
		if (!line.trim()) return;
		store.commandHistory = store.commandHistory.filter((h) => h !== line);
		store.commandHistory.unshift(line);
		if (store.commandHistory.length > 100) store.commandHistory.pop();
		store.historyIndex = -1;
	}
	function historyPrev() {
		if (!store.commandHistory.length) return null;
		store.historyIndex = Math.min(store.historyIndex + 1, store.commandHistory.length - 1);
		return store.commandHistory[store.historyIndex];
	}
	function historyNext() {
		if (!store.commandHistory.length) return null;
		store.historyIndex = Math.max(store.historyIndex - 1, -1);
		return store.historyIndex >= 0 ? store.commandHistory[store.historyIndex] : null;
	}
	function toast(msg, type = "info") {
		store.toast = {
			msg,
			type
		};
		setTimeout(() => {
			if (store.toast && store.toast.msg === msg) store.toast = null;
		}, 2600);
	}
	function getClient() {
		return client;
	}
	function openDialog(type, props) {
		store.dialog = {
			type,
			props
		};
	}
	function closeDialog() {
		store.dialog = null;
	}
	var groupedKeys = computed(() => {
		if (!store.groupBy) return null;
		const map = {};
		for (const k of store.keys) {
			const prefix = k.name.includes(":") ? k.name.split(":")[0] : "(root)";
			(map[prefix] || (map[prefix] = [])).push(k);
		}
		return Object.keys(map).sort().map((g) => ({
			group: g,
			items: map[g]
		}));
	});
	//#endregion
	//#region src/components/ConnBar.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$18 = { style: {
		"display": "flex",
		"align-items": "center",
		"gap": "6px"
	} };
	var _hoisted_2$18 = ["value"];
	var _hoisted_3$18 = ["value"];
	//#endregion
	//#region src/components/ConnBar.vue
	var ConnBar_default = /* @__PURE__ */ defineComponent({
		__name: "ConnBar",
		setup(__props) {
			function onSelect(e) {
				connectById(e.target.value);
			}
			function openNew() {
				openDialog("conn", {});
			}
			function openEdit() {
				const c = store.connections.find((c) => c.id === store.activeConnId);
				if (c) openDialog("conn", { conn: c });
			}
			function onDelete() {
				if (!store.activeConnId) return;
				if (store.connections.length <= 1) {
					toast("至少保留一个连接", "err");
					return;
				}
				deleteConnection(store.activeConnId);
				toast("已删除连接");
			}
			async function onTest() {
				if (!store.activeConnId) return;
				await connectById(store.activeConnId);
				toast(store.connected ? "连接成功" : "连接失败", store.connected ? "ok" : "err");
			}
			return (_ctx, _cache) => {
				return openBlock(), createElementBlock("div", _hoisted_1$18, [
					createBaseVNode("select", {
						class: "select",
						style: {
							"width": "auto",
							"max-width": "170px"
						},
						value: unref(store).activeConnId,
						onChange: onSelect
					}, [(openBlock(true), createElementBlock(Fragment, null, renderList(unref(store).connections, (c) => {
						return openBlock(), createElementBlock("option", {
							key: c.id,
							value: c.id
						}, toDisplayString(c.name) + " (" + toDisplayString(c.mode) + ")", 9, _hoisted_3$18);
					}), 128))], 40, _hoisted_2$18),
					createBaseVNode("button", {
						class: "icon-btn",
						type: "button",
						title: "新增连接",
						onClick: openNew
					}, "＋"),
					createBaseVNode("button", {
						class: "icon-btn",
						type: "button",
						title: "编辑连接",
						onClick: openEdit
					}, "✎"),
					createBaseVNode("button", {
						class: "icon-btn",
						type: "button",
						title: "删除连接",
						onClick: onDelete
					}, "🗑"),
					createBaseVNode("button", {
						class: "icon-btn",
						type: "button",
						title: "测试连接",
						onClick: onTest
					}, "⇆")
				]);
			};
		}
	});
	//#endregion
	//#region src/types.ts
	var TYPE_COLORS = {
		string: "#2f6feb",
		hash: "#1a7f37",
		list: "#b8860b",
		set: "#8957e5",
		zset: "#c0392b",
		stream: "#0d9488",
		none: "#9aa0a6"
	};
	//#endregion
	//#region src/components/KeyList.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$17 = { class: "key-list" };
	var _hoisted_2$17 = { class: "key-group-title" };
	var _hoisted_3$17 = ["onClick"];
	var _hoisted_4$13 = ["checked", "onChange"];
	var _hoisted_5$13 = { class: "kn" };
	var _hoisted_6$13 = {
		key: 0,
		class: "expiry"
	};
	var _hoisted_7$13 = ["onClick"];
	var _hoisted_8$11 = ["checked", "onChange"];
	var _hoisted_9$10 = { class: "kn" };
	var _hoisted_10$9 = {
		key: 0,
		class: "expiry"
	};
	var _hoisted_11$7 = {
		key: 2,
		class: "key-empty"
	};
	var _hoisted_12$7 = {
		key: 3,
		class: "key-empty"
	};
	//#endregion
	//#region src/components/KeyList.vue
	var KeyList_default = /* @__PURE__ */ defineComponent({
		__name: "KeyList",
		setup(__props) {
			const grouped = computed(() => groupedKeys.value);
			function color(t) {
				return TYPE_COLORS[t] || "#9aa0a6";
			}
			function isSoon(k) {
				return k.ttl > 0 && k.ttl < 60;
			}
			return (_ctx, _cache) => {
				return openBlock(), createElementBlock("ul", _hoisted_1$17, [
					unref(store).groupBy && grouped.value ? (openBlock(true), createElementBlock(Fragment, { key: 0 }, renderList(grouped.value, (g) => {
						return openBlock(), createElementBlock(Fragment, { key: g.group }, [createBaseVNode("li", _hoisted_2$17, toDisplayString(g.group) + "（" + toDisplayString(g.items.length) + "）", 1), (openBlock(true), createElementBlock(Fragment, null, renderList(g.items, (k) => {
							return openBlock(), createElementBlock("li", {
								key: k.name,
								class: normalizeClass(["key-item", {
									active: k.name === unref(store).selectedKey,
									soon: isSoon(k)
								}]),
								onClick: ($event) => unref(selectKey)(k.name)
							}, [
								createBaseVNode("input", {
									type: "checkbox",
									checked: unref(store).selectedKeys.includes(k.name),
									onClick: _cache[0] || (_cache[0] = withModifiers(() => {}, ["stop"])),
									onChange: ($event) => unref(toggleSelect)(k.name)
								}, null, 40, _hoisted_4$13),
								createBaseVNode("span", {
									class: "kt",
									style: normalizeStyle({ background: color(k.type) })
								}, toDisplayString(k.type), 5),
								createBaseVNode("span", _hoisted_5$13, toDisplayString(k.name), 1),
								k.ttl > 0 && k.ttl < 60 ? (openBlock(), createElementBlock("span", _hoisted_6$13, "⏱" + toDisplayString(k.ttl) + "s", 1)) : createCommentVNode("", true)
							], 10, _hoisted_3$17);
						}), 128))], 64);
					}), 128)) : (openBlock(true), createElementBlock(Fragment, { key: 1 }, renderList(unref(store).keys, (k) => {
						return openBlock(), createElementBlock("li", {
							key: k.name,
							class: normalizeClass(["key-item", {
								active: k.name === unref(store).selectedKey,
								soon: isSoon(k)
							}]),
							onClick: ($event) => unref(selectKey)(k.name)
						}, [
							createBaseVNode("input", {
								type: "checkbox",
								checked: unref(store).selectedKeys.includes(k.name),
								onClick: _cache[1] || (_cache[1] = withModifiers(() => {}, ["stop"])),
								onChange: ($event) => unref(toggleSelect)(k.name)
							}, null, 40, _hoisted_8$11),
							createBaseVNode("span", {
								class: "kt",
								style: normalizeStyle({ background: color(k.type) })
							}, toDisplayString(k.type), 5),
							createBaseVNode("span", _hoisted_9$10, toDisplayString(k.name), 1),
							k.ttl > 0 && k.ttl < 60 ? (openBlock(), createElementBlock("span", _hoisted_10$9, "⏱" + toDisplayString(k.ttl) + "s", 1)) : createCommentVNode("", true)
						], 10, _hoisted_7$13);
					}), 128)),
					!unref(store).keys.length && !unref(store).loadingKeys ? (openBlock(), createElementBlock("li", _hoisted_11$7, "没有匹配的 key")) : createCommentVNode("", true),
					unref(store).loadingKeys ? (openBlock(), createElementBlock("li", _hoisted_12$7, "加载中…")) : createCommentVNode("", true),
					unref(store).hasMoreKeys ? (openBlock(), createElementBlock("li", {
						key: 4,
						class: "key-empty",
						style: { "cursor": "pointer" },
						onClick: _cache[2] || (_cache[2] = (...args) => unref(loadMoreKeys) && unref(loadMoreKeys)(...args))
					}, " 加载更多… ")) : createCommentVNode("", true)
				]);
			};
		}
	});
	//#endregion
	//#region src/components/Modal.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$16 = { class: "dialog" };
	var _hoisted_2$16 = { class: "d-body" };
	var _hoisted_3$16 = {
		key: 0,
		class: "d-foot"
	};
	//#endregion
	//#region src/components/Modal.vue
	var Modal_default = /* @__PURE__ */ defineComponent({
		__name: "Modal",
		props: { title: {} },
		emits: ["close"],
		setup(__props) {
			return (_ctx, _cache) => {
				return openBlock(), createElementBlock("div", {
					class: "dialog-mask",
					onClick: _cache[0] || (_cache[0] = withModifiers(($event) => _ctx.$emit("close"), ["self"]))
				}, [createBaseVNode("div", _hoisted_1$16, [
					createBaseVNode("h3", null, toDisplayString(__props.title), 1),
					createBaseVNode("div", _hoisted_2$16, [renderSlot(_ctx.$slots, "default")]),
					_ctx.$slots.footer ? (openBlock(), createElementBlock("div", _hoisted_3$16, [renderSlot(_ctx.$slots, "footer")])) : createCommentVNode("", true)
				])]);
			};
		}
	});
	//#endregion
	//#region src/components/BatchToolbar.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$15 = {
		key: 0,
		class: "batch-bar"
	};
	var _hoisted_2$15 = { class: "count" };
	var _hoisted_3$15 = { class: "field" };
	//#endregion
	//#region src/components/BatchToolbar.vue
	var BatchToolbar_default = /* @__PURE__ */ defineComponent({
		__name: "BatchToolbar",
		setup(__props) {
			const showDel = /* @__PURE__ */ ref(false);
			const showExpire = /* @__PURE__ */ ref(false);
			const expireSec = /* @__PURE__ */ ref("");
			async function batchDelete() {
				const c = getClient();
				if (!c) return;
				const keys = [...store.selectedKeys];
				const n = await c.batchDelete(keys);
				recordAudit("BATCH_DELETE", `批量删除 ${n} 个 key`, keys.join(","));
				toast(`已删除 ${n} 个 key`, "ok");
				showDel.value = false;
				clearSelection();
				refreshKeys();
			}
			async function doExpire() {
				const sec = parseInt(expireSec.value, 10);
				if (isNaN(sec) || sec <= 0) {
					toast("请输入有效秒数", "err");
					return;
				}
				const c = getClient();
				if (!c) return;
				const keys = [...store.selectedKeys];
				const n = await c.batchExpire(keys, sec);
				recordAudit("BATCH_EXPIRE", `批量设置过期 ${sec}s × ${n} 个 key`, keys.join(","));
				toast(`已为 ${n} 个 key 设置过期`, "ok");
				showExpire.value = false;
				clearSelection();
				refreshKeys();
			}
			async function batchPersist() {
				const c = getClient();
				if (!c) return;
				const keys = [...store.selectedKeys];
				const n = await c.batchPersist(keys);
				recordAudit("BATCH_PERSIST", `批量转永久 × ${n} 个 key`, keys.join(","));
				toast(`已为 ${n} 个 key 转为永久`, "ok");
				clearSelection();
				refreshKeys();
			}
			function openExport() {
				openDialog("importExport", {
					mode: "export",
					scope: "selected"
				});
			}
			return (_ctx, _cache) => {
				return unref(store).selectedKeys.length ? (openBlock(), createElementBlock("div", _hoisted_1$15, [
					createBaseVNode("span", _hoisted_2$15, "已选 " + toDisplayString(unref(store).selectedKeys.length), 1),
					createBaseVNode("button", {
						class: "mini-btn danger",
						type: "button",
						onClick: _cache[0] || (_cache[0] = ($event) => showDel.value = true)
					}, "批量删除"),
					createBaseVNode("button", {
						class: "mini-btn",
						type: "button",
						onClick: _cache[1] || (_cache[1] = ($event) => showExpire.value = true)
					}, "批量设过期"),
					createBaseVNode("button", {
						class: "mini-btn",
						type: "button",
						onClick: batchPersist
					}, "批量永久"),
					createBaseVNode("button", {
						class: "mini-btn",
						type: "button",
						onClick: openExport
					}, "导出选中"),
					_cache[9] || (_cache[9] = createBaseVNode("span", { class: "spacer" }, null, -1)),
					createBaseVNode("button", {
						class: "mini-btn",
						type: "button",
						onClick: _cache[2] || (_cache[2] = (...args) => unref(clearSelection) && unref(clearSelection)(...args))
					}, "清空选择"),
					showDel.value ? (openBlock(), createBlock(Modal_default, {
						key: 0,
						title: "确认批量删除",
						onClose: _cache[4] || (_cache[4] = ($event) => showDel.value = false)
					}, {
						footer: withCtx(() => [createBaseVNode("button", {
							class: "btn",
							type: "button",
							onClick: _cache[3] || (_cache[3] = ($event) => showDel.value = false)
						}, "取消"), createBaseVNode("button", {
							class: "btn danger",
							type: "button",
							onClick: batchDelete
						}, "删除")]),
						default: withCtx(() => [createBaseVNode("div", null, "确定删除选中的 " + toDisplayString(unref(store).selectedKeys.length) + " 个 key？此操作不可恢复。", 1)]),
						_: 1
					})) : createCommentVNode("", true),
					showExpire.value ? (openBlock(), createBlock(Modal_default, {
						key: 1,
						title: "批量设置过期",
						onClose: _cache[7] || (_cache[7] = ($event) => showExpire.value = false)
					}, {
						footer: withCtx(() => [createBaseVNode("button", {
							class: "btn",
							type: "button",
							onClick: _cache[6] || (_cache[6] = ($event) => showExpire.value = false)
						}, "取消"), createBaseVNode("button", {
							class: "btn primary",
							type: "button",
							onClick: doExpire
						}, "确定")]),
						default: withCtx(() => [createBaseVNode("div", _hoisted_3$15, [_cache[8] || (_cache[8] = createBaseVNode("label", null, "过期秒数", -1)), withDirectives(createBaseVNode("input", {
							class: "input",
							"onUpdate:modelValue": _cache[5] || (_cache[5] = ($event) => expireSec.value = $event),
							placeholder: "例如 3600"
						}, null, 512), [[vModelText, expireSec.value]])])]),
						_: 1
					})) : createCommentVNode("", true)
				])) : createCommentVNode("", true);
			};
		}
	});
	//#endregion
	//#region src/components/DbSidebar.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$14 = { class: "db-picker" };
	var _hoisted_2$14 = { class: "db-grid" };
	var _hoisted_3$14 = ["onClick"];
	var _hoisted_4$12 = { class: "n" };
	var _hoisted_5$12 = { class: "c" };
	var _hoisted_6$12 = { class: "key-tools" };
	var _hoisted_7$12 = ["value"];
	var _hoisted_8$10 = { class: "row" };
	var _hoisted_9$9 = ["value"];
	var _hoisted_10$8 = { class: "chk" };
	var _hoisted_11$6 = ["checked"];
	var _hoisted_12$6 = { class: "key-meta" };
	var _hoisted_13$6 = { class: "row" };
	//#endregion
	//#region src/components/DbSidebar.vue
	var DbSidebar_default = /* @__PURE__ */ defineComponent({
		__name: "DbSidebar",
		setup(__props) {
			let timer = null;
			function onSearch(e) {
				const v = e.target.value;
				clearTimeout(timer);
				timer = setTimeout(() => setSearch(v), 250);
			}
			function onType(e) {
				setTypeFilter(e.target.value);
			}
			function onGroup(e) {
				setGroupBy(e.target.checked);
			}
			return (_ctx, _cache) => {
				return openBlock(), createElementBlock(Fragment, null, [
					createBaseVNode("section", _hoisted_1$14, [_cache[2] || (_cache[2] = createBaseVNode("div", { class: "section-label" }, "数据库（key 数）", -1)), createBaseVNode("div", _hoisted_2$14, [(openBlock(), createElementBlock(Fragment, null, renderList(16, (db) => {
						return createBaseVNode("div", {
							key: db,
							class: normalizeClass(["db-cell", { active: unref(store).currentDb === db - 1 }]),
							onClick: ($event) => unref(selectDb)(db - 1)
						}, [createBaseVNode("span", _hoisted_4$12, toDisplayString(db - 1), 1), createBaseVNode("span", _hoisted_5$12, toDisplayString(unref(store).dbCounts[db - 1] ?? 0), 1)], 10, _hoisted_3$14);
					}), 64))])]),
					createBaseVNode("section", _hoisted_6$12, [
						createBaseVNode("input", {
							class: "input",
							value: unref(store).search,
							onInput: onSearch,
							placeholder: "搜索 key（支持 * 通配）"
						}, null, 40, _hoisted_7$12),
						createBaseVNode("div", _hoisted_8$10, [createBaseVNode("select", {
							class: "select small",
							value: unref(store).typeFilter,
							onChange: onType
						}, [..._cache[3] || (_cache[3] = [createStaticVNode("<option value=\"\">全部类型</option><option value=\"string\">string</option><option value=\"hash\">hash</option><option value=\"list\">list</option><option value=\"set\">set</option><option value=\"zset\">zset</option><option value=\"stream\">stream</option>", 7)])], 40, _hoisted_9$9), createBaseVNode("label", _hoisted_10$8, [createBaseVNode("input", {
							type: "checkbox",
							checked: unref(store).groupBy,
							onChange: onGroup
						}, null, 40, _hoisted_11$6), _cache[4] || (_cache[4] = createTextVNode(" 前缀分组 ", -1))])]),
						createBaseVNode("div", _hoisted_12$6, toDisplayString(unref(store).keys.length) + " 个 key" + toDisplayString(unref(store).hasMoreKeys ? "（还有更多，可加载）" : ""), 1),
						createBaseVNode("div", _hoisted_13$6, [createBaseVNode("button", {
							class: "mini-btn",
							type: "button",
							onClick: _cache[0] || (_cache[0] = ($event) => unref(openDialog)("importExport", { mode: "import" }))
						}, "导入"), createBaseVNode("button", {
							class: "mini-btn",
							type: "button",
							onClick: _cache[1] || (_cache[1] = ($event) => unref(openDialog)("importExport", {
								mode: "export",
								scope: "all"
							}))
						}, "导出全部")])
					]),
					createVNode(KeyList_default),
					createVNode(BatchToolbar_default)
				], 64);
			};
		}
	});
	//#endregion
	//#region src/components/KeyInfoBar.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$13 = {
		key: 0,
		class: "key-info"
	};
	var _hoisted_2$13 = { class: "kname" };
	var _hoisted_3$13 = { class: "ki" };
	var _hoisted_4$11 = { class: "ki" };
	var _hoisted_5$11 = { class: "ki" };
	var _hoisted_6$11 = { class: "field" };
	var _hoisted_7$11 = ["placeholder"];
	var _hoisted_8$9 = { class: "field" };
	var _hoisted_9$8 = ["placeholder"];
	var _hoisted_10$7 = {
		key: 1,
		class: "key-info"
	};
	//#endregion
	//#region src/components/KeyInfoBar.vue
	var KeyInfoBar_default = /* @__PURE__ */ defineComponent({
		__name: "KeyInfoBar",
		setup(__props) {
			const info = computed(() => store.keyInfo);
			const sizeText = computed(() => info.value?.size != null ? formatBytes(info.value.size) : "—");
			const remaining = computed(() => {
				if (!info.value) return -2;
				const ttl = info.value.ttl;
				if (ttl < 0) return ttl;
				const elapsed = Math.floor((Date.now() - store.keyLoadedAt) / 1e3);
				return Math.max(0, ttl - elapsed);
			});
			const progressPct = computed(() => {
				if (!info.value || info.value.ttl <= 0) return "0%";
				const p = remaining.value / info.value.ttl;
				return Math.max(0, Math.min(1, p)) * 100 + "%";
			});
			const isWarn = computed(() => remaining.value > 0 && remaining.value < 60);
			const ttlInput = /* @__PURE__ */ ref("");
			const showRename = /* @__PURE__ */ ref(false);
			const showCopy = /* @__PURE__ */ ref(false);
			const renameTo = /* @__PURE__ */ ref("");
			const copyTo = /* @__PURE__ */ ref("");
			async function setTtl() {
				const sec = parseInt(ttlInput.value, 10);
				if (isNaN(sec) || sec <= 0) {
					toast("请输入有效秒数", "err");
					return;
				}
				const c = getClient();
				if (!c || !store.selectedKey) return;
				if (await c.expire(store.selectedKey, sec)) {
					recordAudit("EXPIRE", `EXPIRE ${store.selectedKey} ${sec}`, store.selectedKey);
					toast("已设置过期时间", "ok");
					reloadKeyInfo();
					refreshKeys();
				}
			}
			async function persist() {
				const c = getClient();
				if (!c || !store.selectedKey) return;
				if (await c.persist(store.selectedKey)) {
					recordAudit("PERSIST", `PERSIST ${store.selectedKey}`, store.selectedKey);
					toast("已转为永久", "ok");
					reloadKeyInfo();
					refreshKeys();
				}
			}
			async function delKey() {
				const c = getClient();
				if (!c || !store.selectedKey) return;
				const k = store.selectedKey;
				await c.del(k);
				recordAudit("DEL", `DEL ${k}`, k);
				toast("已删除 " + k, "ok");
				store.selectedKey = null;
				store.keyInfo = null;
				refreshKeys();
			}
			async function doRename() {
				const c = getClient();
				if (!c || !store.selectedKey) return;
				const oldK = store.selectedKey;
				const newK = renameTo.value.trim();
				if (!newK || newK === oldK) {
					showRename.value = false;
					return;
				}
				await c.rename(oldK, newK);
				recordAudit("RENAME", `RENAME ${oldK} ${newK}`, oldK);
				toast("已重命名为 " + newK, "ok");
				showRename.value = false;
				selectKey(newK);
				refreshKeys();
			}
			async function doCopy() {
				const c = getClient();
				if (!c || !store.selectedKey) return;
				const oldK = store.selectedKey;
				const newK = copyTo.value.trim() || oldK + "_copy";
				if (await c.copy(oldK, newK, true)) {
					recordAudit("COPY", `COPY ${oldK} ${newK}`, newK);
					toast("已复制到 " + newK, "ok");
				} else toast("复制失败（目标已存在且未覆盖）", "err");
				showCopy.value = false;
				refreshKeys();
			}
			return (_ctx, _cache) => {
				return unref(store).selectedKey ? (openBlock(), createElementBlock("div", _hoisted_1$13, [
					createBaseVNode("span", _hoisted_2$13, toDisplayString(unref(store).selectedKey), 1),
					createBaseVNode("span", _hoisted_3$13, [_cache[10] || (_cache[10] = createBaseVNode("b", null, "类型", -1)), createTextVNode(toDisplayString(info.value?.type || "—"), 1)]),
					createBaseVNode("span", _hoisted_4$11, [_cache[11] || (_cache[11] = createBaseVNode("b", null, "内存", -1)), createTextVNode(toDisplayString(sizeText.value), 1)]),
					createBaseVNode("span", _hoisted_5$11, [_cache[12] || (_cache[12] = createBaseVNode("b", null, "TTL", -1)), info.value && info.value.ttl === -1 ? (openBlock(), createElementBlock(Fragment, { key: 0 }, [createTextVNode("永久")], 64)) : info.value && info.value.ttl === -2 ? (openBlock(), createElementBlock(Fragment, { key: 1 }, [createTextVNode("不存在")], 64)) : (openBlock(), createElementBlock(Fragment, { key: 2 }, [createTextVNode(toDisplayString(remaining.value) + "s ", 1), createBaseVNode("span", { class: normalizeClass(["ttl-bar", { warn: isWarn.value }]) }, [createBaseVNode("i", { style: normalizeStyle({ width: progressPct.value }) }, null, 4)], 2)], 64))]),
					_cache[15] || (_cache[15] = createBaseVNode("span", { class: "spacer" }, null, -1)),
					info.value && info.value.type === "string" ? (openBlock(), createElementBlock(Fragment, { key: 0 }, [
						withDirectives(createBaseVNode("input", {
							class: "input",
							style: { "width": "90px" },
							"onUpdate:modelValue": _cache[0] || (_cache[0] = ($event) => ttlInput.value = $event),
							placeholder: "过期秒数"
						}, null, 512), [[vModelText, ttlInput.value]]),
						createBaseVNode("button", {
							class: "mini-btn",
							type: "button",
							onClick: setTtl
						}, "设过期"),
						createBaseVNode("button", {
							class: "mini-btn",
							type: "button",
							onClick: persist
						}, "永久")
					], 64)) : createCommentVNode("", true),
					createBaseVNode("button", {
						class: "mini-btn",
						type: "button",
						onClick: _cache[1] || (_cache[1] = ($event) => showRename.value = true)
					}, "重命名"),
					createBaseVNode("button", {
						class: "mini-btn",
						type: "button",
						onClick: _cache[2] || (_cache[2] = ($event) => showCopy.value = true)
					}, "复制"),
					createBaseVNode("button", {
						class: "mini-btn",
						type: "button",
						onClick: _cache[3] || (_cache[3] = ($event) => unref(openDialog)("compare", { keyA: unref(store).selectedKey }))
					}, "对比"),
					createBaseVNode("button", {
						class: "mini-btn danger",
						type: "button",
						onClick: delKey
					}, "删除"),
					showRename.value ? (openBlock(), createBlock(Modal_default, {
						key: 1,
						title: "重命名 Key",
						onClose: _cache[6] || (_cache[6] = ($event) => showRename.value = false)
					}, {
						footer: withCtx(() => [createBaseVNode("button", {
							class: "btn",
							type: "button",
							onClick: _cache[5] || (_cache[5] = ($event) => showRename.value = false)
						}, "取消"), createBaseVNode("button", {
							class: "btn primary",
							type: "button",
							onClick: doRename
						}, "确定")]),
						default: withCtx(() => [createBaseVNode("div", _hoisted_6$11, [_cache[13] || (_cache[13] = createBaseVNode("label", null, "新名称", -1)), withDirectives(createBaseVNode("input", {
							class: "input",
							"onUpdate:modelValue": _cache[4] || (_cache[4] = ($event) => renameTo.value = $event),
							placeholder: unref(store).selectedKey || ""
						}, null, 8, _hoisted_7$11), [[vModelText, renameTo.value]])])]),
						_: 1
					})) : createCommentVNode("", true),
					showCopy.value ? (openBlock(), createBlock(Modal_default, {
						key: 2,
						title: "复制 Key",
						onClose: _cache[9] || (_cache[9] = ($event) => showCopy.value = false)
					}, {
						footer: withCtx(() => [createBaseVNode("button", {
							class: "btn",
							type: "button",
							onClick: _cache[8] || (_cache[8] = ($event) => showCopy.value = false)
						}, "取消"), createBaseVNode("button", {
							class: "btn primary",
							type: "button",
							onClick: doCopy
						}, "复制")]),
						default: withCtx(() => [createBaseVNode("div", _hoisted_8$9, [_cache[14] || (_cache[14] = createBaseVNode("label", null, "目标名称（同实例）", -1)), withDirectives(createBaseVNode("input", {
							class: "input",
							"onUpdate:modelValue": _cache[7] || (_cache[7] = ($event) => copyTo.value = $event),
							placeholder: (unref(store).selectedKey || "") + "_copy"
						}, null, 8, _hoisted_9$8), [[vModelText, copyTo.value]])])]),
						_: 1
					})) : createCommentVNode("", true)
				])) : (openBlock(), createElementBlock("div", _hoisted_10$7, [..._cache[16] || (_cache[16] = [createBaseVNode("span", {
					class: "kname",
					style: { "color": "var(--text-2)" }
				}, "未选择 key", -1)])]));
			};
		}
	});
	//#endregion
	//#region src/components/JsonNode.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$12 = { class: "jt-node" };
	var _hoisted_2$12 = { class: "jt-row" };
	var _hoisted_3$12 = ["aria-expanded"];
	var _hoisted_4$10 = {
		key: 1,
		class: "jt-toggle"
	};
	var _hoisted_5$10 = {
		key: 2,
		class: "jt-key"
	};
	var _hoisted_6$10 = {
		key: 3,
		class: "jt-punc"
	};
	var _hoisted_7$10 = { class: "jt-row jt-close" };
	var _hoisted_8$8 = { class: "jt-punc" };
	//#endregion
	//#region src/components/JsonNode.vue
	var JsonNode_default = /* @__PURE__ */ defineComponent({
		__name: "JsonNode",
		props: {
			keyName: {},
			value: {},
			depth: {}
		},
		setup(__props) {
			const props = __props;
			const open = /* @__PURE__ */ ref(props.depth < 2);
			const isContainer = computed(() => props.value !== null && typeof props.value === "object");
			const isArray = computed(() => Array.isArray(props.value));
			const childEntries = computed(() => {
				if (!isContainer.value) return [];
				return props.value;
			});
			const collapsedHint = computed(() => {
				const n = isArray.value ? props.value.length : Object.keys(props.value).length;
				return isArray.value ? `[…(${n})]` : `{…(${n})}}`;
			});
			const valueClass = computed(() => {
				const v = props.value;
				if (v === null) return "jt-null";
				switch (typeof v) {
					case "string": return "jt-str";
					case "number": return "jt-num";
					case "boolean": return "jt-bool";
					default: return "jt-null";
				}
			});
			const displayValue = computed(() => {
				const v = props.value;
				if (v === null) return "null";
				if (typeof v === "string") return JSON.stringify(v);
				return String(v);
			});
			return (_ctx, _cache) => {
				return openBlock(), createElementBlock("div", _hoisted_1$12, [createBaseVNode("div", _hoisted_2$12, [
					isContainer.value ? (openBlock(), createElementBlock("span", {
						key: 0,
						class: "jt-toggle",
						type: "button",
						role: "button",
						"aria-expanded": open.value,
						onClick: _cache[0] || (_cache[0] = ($event) => open.value = !open.value)
					}, toDisplayString(open.value ? "▾" : "▸"), 9, _hoisted_3$12)) : (openBlock(), createElementBlock("span", _hoisted_4$10)),
					__props.keyName !== null ? (openBlock(), createElementBlock("span", _hoisted_5$10, toDisplayString(__props.keyName), 1)) : createCommentVNode("", true),
					isContainer.value ? (openBlock(), createElementBlock("span", _hoisted_6$10, toDisplayString(open.value ? isArray.value ? "[" : "{" : collapsedHint.value), 1)) : (openBlock(), createElementBlock("span", {
						key: 4,
						class: normalizeClass(valueClass.value)
					}, toDisplayString(displayValue.value), 3))
				]), isContainer.value && open.value ? (openBlock(), createElementBlock(Fragment, { key: 0 }, [(openBlock(true), createElementBlock(Fragment, null, renderList(childEntries.value, (item, k) => {
					return openBlock(), createBlock(JsonNode_default, {
						key: String(k),
						"key-name": isArray.value ? null : String(k),
						value: item,
						depth: __props.depth + 1
					}, null, 8, [
						"key-name",
						"value",
						"depth"
					]);
				}), 128)), createBaseVNode("div", _hoisted_7$10, [_cache[1] || (_cache[1] = createBaseVNode("span", { class: "jt-toggle" }, null, -1)), createBaseVNode("span", _hoisted_8$8, toDisplayString(isArray.value ? "]" : "}"), 1)])], 64)) : createCommentVNode("", true)]);
			};
		}
	});
	//#endregion
	//#region src/components/ValueEditor.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$11 = {
		key: 0,
		class: "ve"
	};
	var _hoisted_2$11 = {
		key: 0,
		class: "ve-str"
	};
	var _hoisted_3$11 = { class: "str-toolbar" };
	var _hoisted_4$9 = { class: "fmt-tabs" };
	var _hoisted_5$9 = ["onClick"];
	var _hoisted_6$9 = { class: "meta" };
	var _hoisted_7$9 = ["disabled"];
	var _hoisted_8$7 = {
		key: 0,
		class: "json-err"
	};
	var _hoisted_9$7 = {
		key: 1,
		class: "hex-view"
	};
	var _hoisted_10$6 = { class: "hex-off" };
	var _hoisted_11$5 = { class: "hex-bytes" };
	var _hoisted_12$5 = { class: "hex-ascii" };
	var _hoisted_13$5 = {
		key: 0,
		class: "pager"
	};
	var _hoisted_14$5 = {
		key: 2,
		class: "jt"
	};
	var _hoisted_15$5 = {
		key: 0,
		class: "pager"
	};
	var _hoisted_16$4 = {
		key: 1,
		class: "ve-table"
	};
	var _hoisted_17$4 = { class: "editor-toolbar" };
	var _hoisted_18$4 = { class: "meta" };
	var _hoisted_19$4 = { class: "vtable" };
	var _hoisted_20$4 = { class: "mono" };
	var _hoisted_21$4 = ["onUpdate:modelValue", "onBlur"];
	var _hoisted_22$4 = { class: "row-actions" };
	var _hoisted_23$4 = ["onClick"];
	var _hoisted_24$4 = ["onClick"];
	var _hoisted_25$4 = { class: "add-row" };
	var _hoisted_26$4 = {
		key: 2,
		class: "ve-table"
	};
	var _hoisted_27$4 = { class: "editor-toolbar" };
	var _hoisted_28$3 = { class: "meta" };
	var _hoisted_29$3 = { class: "vtable" };
	var _hoisted_30$2 = { class: "idx" };
	var _hoisted_31$2 = ["onUpdate:modelValue", "onBlur"];
	var _hoisted_32$2 = { class: "row-actions" };
	var _hoisted_33$2 = ["onClick"];
	var _hoisted_34$1 = ["onClick"];
	var _hoisted_35$1 = { class: "add-row" };
	var _hoisted_36$1 = {
		key: 3,
		class: "ve-table"
	};
	var _hoisted_37$1 = { class: "editor-toolbar" };
	var _hoisted_38$1 = { class: "meta" };
	var _hoisted_39$1 = { class: "vtable" };
	var _hoisted_40$1 = ["onUpdate:modelValue", "onBlur"];
	var _hoisted_41$1 = { class: "row-actions" };
	var _hoisted_42 = ["onClick"];
	var _hoisted_43 = ["onClick"];
	var _hoisted_44 = { class: "add-row" };
	var _hoisted_45 = {
		key: 4,
		class: "ve-table"
	};
	var _hoisted_46 = { class: "editor-toolbar" };
	var _hoisted_47 = { class: "meta" };
	var _hoisted_48 = { class: "vtable" };
	var _hoisted_49 = ["onUpdate:modelValue", "onBlur"];
	var _hoisted_50 = ["onUpdate:modelValue", "onBlur"];
	var _hoisted_51 = { class: "row-actions" };
	var _hoisted_52 = ["onClick"];
	var _hoisted_53 = ["onClick"];
	var _hoisted_54 = { class: "add-row" };
	var _hoisted_55 = {
		key: 5,
		class: "ve-table"
	};
	var _hoisted_56 = { class: "editor-toolbar" };
	var _hoisted_57 = { class: "meta" };
	var _hoisted_58 = { class: "vtable" };
	var _hoisted_59 = { class: "mono" };
	var _hoisted_60 = ["onClick"];
	var _hoisted_61 = { class: "add-row" };
	var _hoisted_62 = {
		key: 6,
		class: "placeholder"
	};
	var linePage = 200;
	var hexPage = 1024;
	var ValueEditor_vue_vue_type_script_setup_true_lang_default = /*@__PURE__*/ defineComponent({
		__name: "ValueEditor",
		setup(__props) {
			const key = computed(() => store.selectedKey);
			const type = computed(() => store.keyInfo?.type || "none");
			const formats = [
				{
					v: "text",
					label: "Text"
				},
				{
					v: "json",
					label: "JSON"
				},
				{
					v: "xml",
					label: "XML"
				},
				{
					v: "yaml",
					label: "YAML"
				},
				{
					v: "hex",
					label: "Hex"
				}
			];
			const fmt = /* @__PURE__ */ ref("text");
			const jsonTree = /* @__PURE__ */ ref(false);
			const saving = /* @__PURE__ */ ref(false);
			const editText = /* @__PURE__ */ ref("");
			const jsonError = /* @__PURE__ */ ref("");
			const parsedJson = /* @__PURE__ */ ref(null);
			const shownLines = /* @__PURE__ */ ref(linePage);
			const totalLines = computed(() => editText.value ? editText.value.split("\n").length : 0);
			const hasMoreLines = computed(() => shownLines.value < totalLines.value);
			const linesLabel = computed(() => {
				const byteCount = toUtf8Bytes(editText.value).length;
				return `${totalLines.value} 行 · ${formatBytes(byteCount)}`;
			});
			const setFmt = (f) => {
				fmt.value = f;
				if (f === "json") validateJson();
				if (f !== "hex") shownLines.value = linePage;
			};
			function validateJson() {
				jsonError.value = "";
				parsedJson.value = null;
				if (fmt.value !== "json") return;
				const t = editText.value.trim();
				if (!t) return;
				try {
					parsedJson.value = JSON.parse(t);
				} catch (e) {
					jsonError.value = e.message;
				}
			}
			const hexAll = computed(() => toUtf8Bytes(editText.value));
			const shownHex = /* @__PURE__ */ ref(hexPage);
			const hasMoreHex = computed(() => shownHex.value < hexAll.value.length);
			function toPrintable(b) {
				return b >= 32 && b < 127 ? String.fromCharCode(b) : ".";
			}
			const hexRows = computed(() => {
				const bytes = hexAll.value.slice(0, shownHex.value);
				const rows = [];
				for (let i = 0; i < bytes.length; i += 16) {
					const slice = bytes.slice(i, i + 16);
					const hex = Array.from(slice).map((b) => b.toString(16).padStart(2, "0")).join(" ");
					const ascii = Array.from(slice).map((b) => toPrintable(b)).join("");
					rows.push({
						offset: i.toString(16).padStart(8, "0"),
						bytes: hex,
						ascii
					});
				}
				return rows;
			});
			async function saveString() {
				const c = getClient();
				if (!c || !key.value) return;
				if (fmt.value === "json") {
					validateJson();
					if (jsonError.value) {
						toast("JSON 格式错误，未保存", "err");
						return;
					}
				}
				if (fmt.value === "hex") {
					toast("Hex 视图为只读，请切回文本/JSON 编辑", "err");
					return;
				}
				saving.value = true;
				try {
					await c.set(key.value, editText.value);
					toast("已保存", "ok");
					reloadKeyInfo();
					refreshKeys();
				} finally {
					saving.value = false;
				}
			}
			const hashRows = /* @__PURE__ */ ref([]);
			const newHashField = /* @__PURE__ */ ref("");
			const newHashValue = /* @__PURE__ */ ref("");
			async function saveHash(r) {
				const c = getClient();
				if (!c || !key.value) return;
				await c.hset(key.value, r.field, r.draftValue);
				r.value = r.draftValue;
				toast("已更新字段", "ok");
				reloadKeyInfo();
			}
			async function delHash(field) {
				const c = getClient();
				if (!c || !key.value) return;
				await c.hdel(key.value, field);
				hashRows.value = hashRows.value.filter((r) => r.field !== field);
				toast("已删除字段", "ok");
				reloadKeyInfo();
				refreshKeys();
			}
			async function addHash() {
				const c = getClient();
				if (!c || !key.value || !newHashField.value) return;
				await c.hset(key.value, newHashField.value, newHashValue.value);
				hashRows.value.push({
					field: newHashField.value,
					value: newHashValue.value,
					draftValue: newHashValue.value
				});
				newHashField.value = "";
				newHashValue.value = "";
				reloadKeyInfo();
			}
			const listItems = /* @__PURE__ */ ref([]);
			const newListItem = /* @__PURE__ */ ref("");
			async function saveList(i) {
				const c = getClient();
				if (!c || !key.value) return;
				await c.lset(key.value, i, listItems.value[i].draft);
				listItems.value[i].value = listItems.value[i].draft;
				toast("已更新元素", "ok");
				reloadKeyInfo();
			}
			async function delList(value) {
				const c = getClient();
				if (!c || !key.value) return;
				await c.lrem(key.value, value);
				listItems.value = listItems.value.filter((it) => it.value !== value);
				toast("已删除元素", "ok");
				reloadKeyInfo();
				refreshKeys();
			}
			async function addList() {
				const c = getClient();
				if (!c || !key.value || !newListItem.value) return;
				await c.rpush(key.value, newListItem.value);
				listItems.value.push({
					value: newListItem.value,
					draft: newListItem.value
				});
				newListItem.value = "";
				reloadKeyInfo();
			}
			const setMembers = /* @__PURE__ */ ref([]);
			const newSetMember = /* @__PURE__ */ ref("");
			async function saveSet(m) {
				const c = getClient();
				if (!c || !key.value) return;
				if (m.draft === m.value) return;
				await c.srem(key.value, m.value);
				await c.sadd(key.value, m.draft);
				m.value = m.draft;
				toast("已更新成员", "ok");
				reloadKeyInfo();
			}
			async function delSet(value) {
				const c = getClient();
				if (!c || !key.value) return;
				await c.srem(key.value, value);
				setMembers.value = setMembers.value.filter((m) => m.value !== value);
				toast("已删除成员", "ok");
				reloadKeyInfo();
				refreshKeys();
			}
			async function addSet() {
				const c = getClient();
				if (!c || !key.value || !newSetMember.value) return;
				await c.sadd(key.value, newSetMember.value);
				setMembers.value.push({
					value: newSetMember.value,
					draft: newSetMember.value
				});
				newSetMember.value = "";
				reloadKeyInfo();
			}
			const zsetMembers = /* @__PURE__ */ ref([]);
			const newZMember = /* @__PURE__ */ ref("");
			const newZScore = /* @__PURE__ */ ref("");
			async function saveZSet(m) {
				const c = getClient();
				if (!c || !key.value) return;
				const score = parseFloat(m.draftScore);
				if (isNaN(score)) {
					toast("分值必须是数字", "err");
					return;
				}
				if (m.draftMember !== m.member) await c.zrem(key.value, m.member);
				await c.zadd(key.value, m.draftMember, score);
				m.member = m.draftMember;
				m.score = score;
				toast("已更新成员", "ok");
				reloadKeyInfo();
			}
			async function delZSet(member) {
				const c = getClient();
				if (!c || !key.value) return;
				await c.zrem(key.value, member);
				zsetMembers.value = zsetMembers.value.filter((m) => m.member !== member);
				toast("已删除成员", "ok");
				reloadKeyInfo();
				refreshKeys();
			}
			async function addZSet() {
				const c = getClient();
				if (!c || !key.value || !newZMember.value) return;
				const score = parseFloat(newZScore.value);
				if (isNaN(score)) {
					toast("分值必须是数字", "err");
					return;
				}
				await c.zadd(key.value, newZMember.value, score);
				zsetMembers.value.push({
					member: newZMember.value,
					score,
					draftMember: newZMember.value,
					draftScore: String(score)
				});
				newZMember.value = "";
				newZScore.value = "";
				reloadKeyInfo();
			}
			const streamEntries = /* @__PURE__ */ ref([]);
			const newStreamField = /* @__PURE__ */ ref("");
			const newStreamValue = /* @__PURE__ */ ref("");
			async function delStream(id) {
				const c = getClient();
				if (!c || !key.value) return;
				await c.xdel(key.value, id);
				streamEntries.value = streamEntries.value.filter((e) => e.id !== id);
				toast("已删除条目", "ok");
				reloadKeyInfo();
				refreshKeys();
			}
			async function addStream() {
				const c = getClient();
				if (!c || !key.value || !newStreamField.value) return;
				await c.xadd(key.value, { [newStreamField.value]: newStreamValue.value });
				newStreamField.value = "";
				newStreamValue.value = "";
				await reloadActual();
				reloadKeyInfo();
			}
			async function reloadActual() {
				const c = getClient();
				if (!c || !key.value) return;
				const t = type.value;
				if (t === "string") {
					const raw = await c.get(key.value);
					editText.value = raw == null ? "" : raw;
					shownLines.value = linePage;
					validateJson();
				} else if (t === "hash") {
					const o = await c.hgetall(key.value);
					hashRows.value = Object.keys(o).map((f) => ({
						field: f,
						value: o[f],
						draftValue: o[f]
					}));
				} else if (t === "list") {
					const arr = await c.lrange(key.value, 0, -1);
					listItems.value = arr.map((v) => ({
						value: v,
						draft: v
					}));
				} else if (t === "set") {
					const arr = await c.smembers(key.value);
					setMembers.value = arr.map((v) => ({
						value: v,
						draft: v
					}));
				} else if (t === "zset") {
					const arr = await c.zrangeAll(key.value);
					zsetMembers.value = arr.map((m) => ({
						member: m.member,
						score: m.score,
						draftMember: m.member,
						draftScore: String(m.score)
					}));
				} else if (t === "stream") streamEntries.value = await c.xrange(key.value, "-", "+");
			}
			function reload() {
				reloadActual();
			}
			watch(() => [store.selectedKey, store.keyInfo?.type], () => {
				fmt.value = "text";
				jsonTree.value = false;
				if (key.value) reloadActual();
			}, { immediate: true });
			return (_ctx, _cache) => {
				return key.value ? (openBlock(), createElementBlock("div", _hoisted_1$11, [type.value === "string" ? (openBlock(), createElementBlock("div", _hoisted_2$11, [
					createBaseVNode("div", _hoisted_3$11, [
						createBaseVNode("div", _hoisted_4$9, [(openBlock(), createElementBlock(Fragment, null, renderList(formats, (f) => {
							return createBaseVNode("button", {
								key: f.v,
								type: "button",
								class: normalizeClass({ active: fmt.value === f.v }),
								onClick: ($event) => setFmt(f.v)
							}, toDisplayString(f.label), 11, _hoisted_5$9);
						}), 64))]),
						fmt.value === "json" ? (openBlock(), createElementBlock("button", {
							key: 0,
							class: "mini-btn",
							type: "button",
							onClick: _cache[0] || (_cache[0] = ($event) => jsonTree.value = !jsonTree.value)
						}, toDisplayString(jsonTree.value ? "文本视图" : "JSON 树"), 1)) : createCommentVNode("", true),
						_cache[12] || (_cache[12] = createBaseVNode("span", { class: "spacer" }, null, -1)),
						createBaseVNode("span", _hoisted_6$9, toDisplayString(linesLabel.value), 1),
						createBaseVNode("button", {
							class: "btn sm primary",
							type: "button",
							onClick: saveString,
							disabled: saving.value
						}, "保存", 8, _hoisted_7$9)
					]),
					jsonError.value ? (openBlock(), createElementBlock("div", _hoisted_8$7, "⚠ " + toDisplayString(jsonError.value), 1)) : createCommentVNode("", true),
					fmt.value === "hex" ? (openBlock(), createElementBlock("div", _hoisted_9$7, [(openBlock(true), createElementBlock(Fragment, null, renderList(hexRows.value, (row, i) => {
						return openBlock(), createElementBlock("div", {
							key: i,
							class: "hex-row"
						}, [
							createBaseVNode("span", _hoisted_10$6, toDisplayString(row.offset), 1),
							createBaseVNode("span", _hoisted_11$5, toDisplayString(row.bytes), 1),
							createBaseVNode("span", _hoisted_12$5, toDisplayString(row.ascii), 1)
						]);
					}), 128)), hasMoreHex.value ? (openBlock(), createElementBlock("div", _hoisted_13$5, [
						createBaseVNode("span", null, "已显示 " + toDisplayString(shownHex.value) + " / " + toDisplayString(hexAll.value.length) + " 字节", 1),
						_cache[13] || (_cache[13] = createBaseVNode("span", { class: "spacer" }, null, -1)),
						createBaseVNode("button", {
							class: "mini-btn",
							type: "button",
							onClick: _cache[1] || (_cache[1] = ($event) => shownHex.value += hexPage)
						}, "加载更多")
					])) : createCommentVNode("", true)])) : fmt.value === "json" && jsonTree.value && !jsonError.value ? (openBlock(), createElementBlock("div", _hoisted_14$5, [createVNode(JsonNode_default, {
						"key-name": null,
						value: parsedJson.value,
						depth: 0
					}, null, 8, ["value"])])) : (openBlock(), createElementBlock(Fragment, { key: 3 }, [withDirectives(createBaseVNode("textarea", {
						class: "str-area",
						"onUpdate:modelValue": _cache[2] || (_cache[2] = ($event) => editText.value = $event),
						spellcheck: "false"
					}, null, 512), [[vModelText, editText.value]]), hasMoreLines.value ? (openBlock(), createElementBlock("div", _hoisted_15$5, [
						createBaseVNode("span", null, "已显示 " + toDisplayString(shownLines.value) + " / " + toDisplayString(totalLines.value) + " 行", 1),
						_cache[14] || (_cache[14] = createBaseVNode("span", { class: "spacer" }, null, -1)),
						createBaseVNode("button", {
							class: "mini-btn",
							type: "button",
							onClick: _cache[3] || (_cache[3] = ($event) => shownLines.value += linePage)
						}, "加载更多")
					])) : createCommentVNode("", true)], 64))
				])) : type.value === "hash" ? (openBlock(), createElementBlock("div", _hoisted_16$4, [
					createBaseVNode("div", _hoisted_17$4, [
						createBaseVNode("span", _hoisted_18$4, toDisplayString(hashRows.value.length) + " 个字段", 1),
						_cache[15] || (_cache[15] = createBaseVNode("span", { class: "spacer" }, null, -1)),
						createBaseVNode("button", {
							class: "btn sm",
							type: "button",
							onClick: reload
						}, "刷新")
					]),
					createBaseVNode("table", _hoisted_19$4, [_cache[16] || (_cache[16] = createBaseVNode("thead", null, [createBaseVNode("tr", null, [
						createBaseVNode("th", null, "字段"),
						createBaseVNode("th", null, "值"),
						createBaseVNode("th", { style: { "width": "120px" } }, "操作")
					])], -1)), createBaseVNode("tbody", null, [(openBlock(true), createElementBlock(Fragment, null, renderList(hashRows.value, (r) => {
						return openBlock(), createElementBlock("tr", { key: r.field }, [
							createBaseVNode("td", _hoisted_20$4, toDisplayString(r.field), 1),
							createBaseVNode("td", null, [withDirectives(createBaseVNode("input", {
								class: "cell-input",
								"onUpdate:modelValue": ($event) => r.draftValue = $event,
								onBlur: ($event) => saveHash(r)
							}, null, 40, _hoisted_21$4), [[vModelText, r.draftValue]])]),
							createBaseVNode("td", null, [createBaseVNode("div", _hoisted_22$4, [createBaseVNode("button", {
								class: "mini-btn",
								type: "button",
								onClick: ($event) => saveHash(r)
							}, "存", 8, _hoisted_23$4), createBaseVNode("button", {
								class: "mini-btn danger",
								type: "button",
								onClick: ($event) => delHash(r.field)
							}, "删", 8, _hoisted_24$4)])])
						]);
					}), 128))])]),
					createBaseVNode("div", _hoisted_25$4, [
						withDirectives(createBaseVNode("input", {
							class: "input",
							"onUpdate:modelValue": _cache[4] || (_cache[4] = ($event) => newHashField.value = $event),
							placeholder: "字段名"
						}, null, 512), [[vModelText, newHashField.value]]),
						withDirectives(createBaseVNode("input", {
							class: "input",
							"onUpdate:modelValue": _cache[5] || (_cache[5] = ($event) => newHashValue.value = $event),
							placeholder: "值",
							onKeyup: withKeys(addHash, ["enter"])
						}, null, 544), [[vModelText, newHashValue.value]]),
						createBaseVNode("button", {
							class: "btn",
							type: "button",
							onClick: addHash
						}, "添加")
					])
				])) : type.value === "list" ? (openBlock(), createElementBlock("div", _hoisted_26$4, [
					createBaseVNode("div", _hoisted_27$4, [
						createBaseVNode("span", _hoisted_28$3, toDisplayString(listItems.value.length) + " 个元素", 1),
						_cache[17] || (_cache[17] = createBaseVNode("span", { class: "spacer" }, null, -1)),
						createBaseVNode("button", {
							class: "btn sm",
							type: "button",
							onClick: reload
						}, "刷新")
					]),
					createBaseVNode("table", _hoisted_29$3, [_cache[18] || (_cache[18] = createBaseVNode("thead", null, [createBaseVNode("tr", null, [
						createBaseVNode("th", { style: { "width": "60px" } }, "#"),
						createBaseVNode("th", null, "值"),
						createBaseVNode("th", { style: { "width": "120px" } }, "操作")
					])], -1)), createBaseVNode("tbody", null, [(openBlock(true), createElementBlock(Fragment, null, renderList(listItems.value, (it, i) => {
						return openBlock(), createElementBlock("tr", { key: i }, [
							createBaseVNode("td", _hoisted_30$2, toDisplayString(i), 1),
							createBaseVNode("td", null, [withDirectives(createBaseVNode("input", {
								class: "cell-input",
								"onUpdate:modelValue": ($event) => it.draft = $event,
								onBlur: ($event) => saveList(i)
							}, null, 40, _hoisted_31$2), [[vModelText, it.draft]])]),
							createBaseVNode("td", null, [createBaseVNode("div", _hoisted_32$2, [createBaseVNode("button", {
								class: "mini-btn",
								type: "button",
								onClick: ($event) => saveList(i)
							}, "存", 8, _hoisted_33$2), createBaseVNode("button", {
								class: "mini-btn danger",
								type: "button",
								onClick: ($event) => delList(it.value)
							}, "删", 8, _hoisted_34$1)])])
						]);
					}), 128))])]),
					createBaseVNode("div", _hoisted_35$1, [withDirectives(createBaseVNode("input", {
						class: "input",
						"onUpdate:modelValue": _cache[6] || (_cache[6] = ($event) => newListItem.value = $event),
						placeholder: "新元素值",
						onKeyup: withKeys(addList, ["enter"])
					}, null, 544), [[vModelText, newListItem.value]]), createBaseVNode("button", {
						class: "btn",
						type: "button",
						onClick: addList
					}, "右侧推入")])
				])) : type.value === "set" ? (openBlock(), createElementBlock("div", _hoisted_36$1, [
					createBaseVNode("div", _hoisted_37$1, [
						createBaseVNode("span", _hoisted_38$1, toDisplayString(setMembers.value.length) + " 个成员", 1),
						_cache[19] || (_cache[19] = createBaseVNode("span", { class: "spacer" }, null, -1)),
						createBaseVNode("button", {
							class: "btn sm",
							type: "button",
							onClick: reload
						}, "刷新")
					]),
					createBaseVNode("table", _hoisted_39$1, [_cache[20] || (_cache[20] = createBaseVNode("thead", null, [createBaseVNode("tr", null, [createBaseVNode("th", null, "成员"), createBaseVNode("th", { style: { "width": "120px" } }, "操作")])], -1)), createBaseVNode("tbody", null, [(openBlock(true), createElementBlock(Fragment, null, renderList(setMembers.value, (m) => {
						return openBlock(), createElementBlock("tr", { key: m.value }, [createBaseVNode("td", null, [withDirectives(createBaseVNode("input", {
							class: "cell-input",
							"onUpdate:modelValue": ($event) => m.draft = $event,
							onBlur: ($event) => saveSet(m)
						}, null, 40, _hoisted_40$1), [[vModelText, m.draft]])]), createBaseVNode("td", null, [createBaseVNode("div", _hoisted_41$1, [createBaseVNode("button", {
							class: "mini-btn",
							type: "button",
							onClick: ($event) => saveSet(m)
						}, "存", 8, _hoisted_42), createBaseVNode("button", {
							class: "mini-btn danger",
							type: "button",
							onClick: ($event) => delSet(m.value)
						}, "删", 8, _hoisted_43)])])]);
					}), 128))])]),
					createBaseVNode("div", _hoisted_44, [withDirectives(createBaseVNode("input", {
						class: "input",
						"onUpdate:modelValue": _cache[7] || (_cache[7] = ($event) => newSetMember.value = $event),
						placeholder: "新成员",
						onKeyup: withKeys(addSet, ["enter"])
					}, null, 544), [[vModelText, newSetMember.value]]), createBaseVNode("button", {
						class: "btn",
						type: "button",
						onClick: addSet
					}, "添加")])
				])) : type.value === "zset" ? (openBlock(), createElementBlock("div", _hoisted_45, [
					createBaseVNode("div", _hoisted_46, [
						createBaseVNode("span", _hoisted_47, toDisplayString(zsetMembers.value.length) + " 个成员", 1),
						_cache[21] || (_cache[21] = createBaseVNode("span", { class: "spacer" }, null, -1)),
						createBaseVNode("button", {
							class: "btn sm",
							type: "button",
							onClick: reload
						}, "刷新")
					]),
					createBaseVNode("table", _hoisted_48, [_cache[22] || (_cache[22] = createBaseVNode("thead", null, [createBaseVNode("tr", null, [
						createBaseVNode("th", null, "成员"),
						createBaseVNode("th", { style: { "width": "120px" } }, "分值"),
						createBaseVNode("th", { style: { "width": "120px" } }, "操作")
					])], -1)), createBaseVNode("tbody", null, [(openBlock(true), createElementBlock(Fragment, null, renderList(zsetMembers.value, (m) => {
						return openBlock(), createElementBlock("tr", { key: m.member }, [
							createBaseVNode("td", null, [withDirectives(createBaseVNode("input", {
								class: "cell-input",
								"onUpdate:modelValue": ($event) => m.draftMember = $event,
								onBlur: ($event) => saveZSet(m)
							}, null, 40, _hoisted_49), [[vModelText, m.draftMember]])]),
							createBaseVNode("td", null, [withDirectives(createBaseVNode("input", {
								class: "cell-input",
								"onUpdate:modelValue": ($event) => m.draftScore = $event,
								onBlur: ($event) => saveZSet(m)
							}, null, 40, _hoisted_50), [[vModelText, m.draftScore]])]),
							createBaseVNode("td", null, [createBaseVNode("div", _hoisted_51, [createBaseVNode("button", {
								class: "mini-btn",
								type: "button",
								onClick: ($event) => saveZSet(m)
							}, "存", 8, _hoisted_52), createBaseVNode("button", {
								class: "mini-btn danger",
								type: "button",
								onClick: ($event) => delZSet(m.member)
							}, "删", 8, _hoisted_53)])])
						]);
					}), 128))])]),
					createBaseVNode("div", _hoisted_54, [
						withDirectives(createBaseVNode("input", {
							class: "input",
							style: { "flex": ".4" },
							"onUpdate:modelValue": _cache[8] || (_cache[8] = ($event) => newZMember.value = $event),
							placeholder: "成员"
						}, null, 512), [[vModelText, newZMember.value]]),
						withDirectives(createBaseVNode("input", {
							class: "input",
							style: { "flex": ".3" },
							"onUpdate:modelValue": _cache[9] || (_cache[9] = ($event) => newZScore.value = $event),
							placeholder: "分值",
							onKeyup: withKeys(addZSet, ["enter"])
						}, null, 544), [[vModelText, newZScore.value]]),
						createBaseVNode("button", {
							class: "btn",
							type: "button",
							onClick: addZSet
						}, "添加")
					])
				])) : type.value === "stream" ? (openBlock(), createElementBlock("div", _hoisted_55, [
					createBaseVNode("div", _hoisted_56, [
						createBaseVNode("span", _hoisted_57, toDisplayString(streamEntries.value.length) + " 条条目", 1),
						_cache[23] || (_cache[23] = createBaseVNode("span", { class: "spacer" }, null, -1)),
						createBaseVNode("button", {
							class: "btn sm",
							type: "button",
							onClick: reload
						}, "刷新")
					]),
					createBaseVNode("table", _hoisted_58, [_cache[24] || (_cache[24] = createBaseVNode("thead", null, [createBaseVNode("tr", null, [
						createBaseVNode("th", { style: { "width": "200px" } }, "ID"),
						createBaseVNode("th", null, "字段"),
						createBaseVNode("th", { style: { "width": "90px" } }, "操作")
					])], -1)), createBaseVNode("tbody", null, [(openBlock(true), createElementBlock(Fragment, null, renderList(streamEntries.value, (e) => {
						return openBlock(), createElementBlock("tr", { key: e.id }, [
							createBaseVNode("td", _hoisted_59, toDisplayString(e.id), 1),
							createBaseVNode("td", null, [(openBlock(true), createElementBlock(Fragment, null, renderList(e.fields, (v, f) => {
								return openBlock(), createElementBlock("span", {
									key: f,
									class: "kv-pill"
								}, [createBaseVNode("b", null, toDisplayString(f), 1), createTextVNode("=" + toDisplayString(v), 1)]);
							}), 128))]),
							createBaseVNode("td", null, [createBaseVNode("button", {
								class: "mini-btn danger",
								type: "button",
								onClick: ($event) => delStream(e.id)
							}, "删", 8, _hoisted_60)])
						]);
					}), 128))])]),
					createBaseVNode("div", _hoisted_61, [
						withDirectives(createBaseVNode("input", {
							class: "input",
							style: { "flex": ".4" },
							"onUpdate:modelValue": _cache[10] || (_cache[10] = ($event) => newStreamField.value = $event),
							placeholder: "字段名"
						}, null, 512), [[vModelText, newStreamField.value]]),
						withDirectives(createBaseVNode("input", {
							class: "input",
							style: { "flex": ".4" },
							"onUpdate:modelValue": _cache[11] || (_cache[11] = ($event) => newStreamValue.value = $event),
							placeholder: "字段值"
						}, null, 512), [[vModelText, newStreamValue.value]]),
						createBaseVNode("button", {
							class: "btn",
							type: "button",
							onClick: addStream
						}, "追加条目")
					])
				])) : (openBlock(), createElementBlock("div", _hoisted_62, "未知类型：" + toDisplayString(type.value), 1))])) : createCommentVNode("", true);
			};
		}
	});
	//#endregion
	//#region \0plugin-vue:export-helper
	var _plugin_vue_export_helper_default = (sfc, props) => {
		const target = sfc.__vccOpts || sfc;
		for (const [key, val] of props) target[key] = val;
		return target;
	};
	//#endregion
	//#region src/components/ValueEditor.vue
	var ValueEditor_default = /*#__PURE__*/ _plugin_vue_export_helper_default(ValueEditor_vue_vue_type_script_setup_true_lang_default, [["__scopeId", "data-v-4ba484fd"]]);
	//#endregion
	//#region src/components/Terminal.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$10 = { class: "term" };
	var _hoisted_2$10 = {
		key: 0,
		class: "term-line-info"
	};
	var _hoisted_3$10 = { class: "term-input-row" };
	//#endregion
	//#region src/components/Terminal.vue
	var Terminal_default = /* @__PURE__ */ defineComponent({
		__name: "Terminal",
		setup(__props) {
			const line = /* @__PURE__ */ ref("");
			const out = /* @__PURE__ */ ref(null);
			function scrollBottom() {
				nextTick(() => {
					if (out.value) out.value.scrollTop = out.value.scrollHeight;
				});
			}
			watch(() => store.terminalLines.length, scrollBottom);
			function submit() {
				const text = line.value;
				if (!text.trim()) return;
				runCommand(text);
				pushHistory(text);
				line.value = "";
			}
			function onKey(e) {
				if (e.key === "Enter") {
					e.preventDefault();
					submit();
				} else if (e.key === "ArrowUp") {
					e.preventDefault();
					const h = historyPrev();
					if (h != null) line.value = h;
				} else if (e.key === "ArrowDown") {
					e.preventDefault();
					const h = historyNext();
					if (h != null) line.value = h;
				} else if (e.key === "Tab") {
					e.preventDefault();
					complete();
				}
			}
			function complete() {
				const parts = line.value.split(/\s+/);
				const matches = completeCommand(parts[parts.length - 1] || "");
				if (matches.length === 1) {
					parts[parts.length - 1] = matches[0];
					line.value = parts.join(" ");
				} else if (matches.length > 1) {
					store.terminalLines.push({
						kind: "info",
						text: matches.join("   ")
					});
					scrollBottom();
				}
			}
			return (_ctx, _cache) => {
				return openBlock(), createElementBlock("div", _hoisted_1$10, [createBaseVNode("div", {
					class: "term-output",
					ref_key: "out",
					ref: out,
					"aria-live": "polite"
				}, [(openBlock(true), createElementBlock(Fragment, null, renderList(unref(store).terminalLines, (l, i) => {
					return openBlock(), createElementBlock("div", {
						key: i,
						class: normalizeClass("term-line-" + l.kind)
					}, toDisplayString(l.text), 3);
				}), 128)), !unref(store).terminalLines.length ? (openBlock(), createElementBlock("div", _hoisted_2$10, [..._cache[1] || (_cache[1] = [
					createTextVNode(" 命令行已就绪。输入 Redis 命令，如 ", -1),
					createBaseVNode("code", null, "GET foo", -1),
					createTextVNode(" / ", -1),
					createBaseVNode("code", null, "HGETALL user:1", -1),
					createTextVNode(" / ", -1),
					createBaseVNode("code", null, "SCAN 0 MATCH user:*", -1),
					createTextVNode("。 ↑/↓ 浏览历史，Tab 自动补全。 ", -1)
				])])) : createCommentVNode("", true)], 512), createBaseVNode("div", _hoisted_3$10, [_cache[2] || (_cache[2] = createBaseVNode("span", { class: "prompt" }, "redis>", -1)), withDirectives(createBaseVNode("input", {
					class: "input term-input",
					"onUpdate:modelValue": _cache[0] || (_cache[0] = ($event) => line.value = $event),
					type: "text",
					autocomplete: "off",
					spellcheck: "false",
					placeholder: "输入 Redis 命令",
					"aria-label": "命令输入",
					onKeydown: onKey
				}, null, 544), [[vModelText, line.value]])])]);
			};
		}
	});
	//#endregion
	//#region src/monitor.ts
	function parseInfo(text) {
		const sections = [];
		let current = null;
		text.split("\n").forEach((line) => {
			const t = line.trim();
			if (!t) return;
			if (t.startsWith("#")) {
				current = {
					name: t.slice(1).trim(),
					fields: []
				};
				sections.push(current);
				return;
			}
			const i = t.indexOf(":");
			if (i >= 0 && current) current.fields.push({
				k: t.slice(0, i),
				v: t.slice(i + 1)
			});
		});
		return sections;
	}
	function computeMemoryUsage(text) {
		const mem = parseInfo(text).find((s) => s.name === "Memory");
		let usedBytes = 0;
		let maxBytes = 0;
		if (mem) {
			const used = mem.fields.find((f) => f.k === "used_memory");
			const max = mem.fields.find((f) => f.k === "maxmemory");
			if (used) usedBytes = parseInt(used.v, 10) || 0;
			if (max && parseInt(max.v, 10) > 0) maxBytes = parseInt(max.v, 10);
		}
		const ratio = maxBytes > 0 ? usedBytes / maxBytes : 0;
		return {
			usedBytes,
			maxBytes,
			ratio
		};
	}
	function infoField(text, section, key) {
		const s = parseInfo(text).find((x) => x.name === section);
		if (!s) return null;
		const f = s.fields.find((x) => x.k === key);
		return f ? f.v : null;
	}
	//#endregion
	//#region src/components/MonitorPanel.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$9 = { class: "monitor" };
	var _hoisted_2$9 = { class: "monitor-tabs" };
	var _hoisted_3$9 = ["disabled"];
	var _hoisted_4$8 = { class: "monitor-body" };
	var _hoisted_5$8 = {
		key: 0,
		class: "pane-inner"
	};
	var _hoisted_6$8 = { class: "monitor-grid" };
	var _hoisted_7$8 = { class: "metric-card" };
	var _hoisted_8$6 = { class: "value" };
	var _hoisted_9$6 = { class: "mem-bar" };
	var _hoisted_10$5 = { class: "sub" };
	var _hoisted_11$4 = { class: "metric-card" };
	var _hoisted_12$4 = { class: "value" };
	var _hoisted_13$4 = { class: "metric-card" };
	var _hoisted_14$4 = { class: "value" };
	var _hoisted_15$4 = { class: "metric-card" };
	var _hoisted_16$3 = { class: "value" };
	var _hoisted_17$3 = { class: "metric-card" };
	var _hoisted_18$3 = { class: "value" };
	var _hoisted_19$3 = { class: "sub" };
	var _hoisted_20$3 = { class: "metric-card" };
	var _hoisted_21$3 = {
		class: "value",
		style: { "font-size": "16px" }
	};
	var _hoisted_22$3 = { class: "info-section" };
	var _hoisted_23$3 = { class: "kv-table" };
	var _hoisted_24$3 = { key: 0 };
	var _hoisted_25$3 = {
		key: 1,
		class: "pane-inner"
	};
	var _hoisted_26$3 = { class: "kv-table" };
	var _hoisted_27$3 = {
		key: 0,
		class: "key-empty"
	};
	var _hoisted_28$2 = {
		key: 2,
		class: "pane-inner"
	};
	var _hoisted_29$2 = { class: "vtable" };
	var _hoisted_30$1 = { class: "mono" };
	var _hoisted_31$1 = { class: "mono" };
	var _hoisted_32$1 = { class: "mono" };
	var _hoisted_33$1 = { key: 0 };
	var _hoisted_34 = {
		key: 3,
		class: "pane-inner"
	};
	var _hoisted_35 = { class: "vtable" };
	var _hoisted_36 = { class: "mono" };
	var _hoisted_37 = { class: "mono" };
	var _hoisted_38 = { class: "mono" };
	var _hoisted_39 = { class: "mono" };
	var _hoisted_40 = { class: "mono" };
	var _hoisted_41 = { key: 0 };
	//#endregion
	//#region src/components/MonitorPanel.vue
	var MonitorPanel_default = /*#__PURE__*/ _plugin_vue_export_helper_default(/* @__PURE__ */ defineComponent({
		__name: "MonitorPanel",
		setup(__props) {
			const tab = /* @__PURE__ */ ref("overview");
			const loading = /* @__PURE__ */ ref(false);
			const infoText = /* @__PURE__ */ ref("");
			const sections = /* @__PURE__ */ ref([]);
			const slowlog = /* @__PURE__ */ ref([]);
			const clients = /* @__PURE__ */ ref([]);
			const mem = computed(() => {
				const m = computeMemoryUsage(infoText.value);
				return {
					usedText: formatBytes(m.usedBytes),
					maxText: m.maxBytes > 0 ? formatBytes(m.maxBytes) : "无限制",
					pct: Math.round(m.ratio * 100)
				};
			});
			const uptime = computed(() => {
				const v = infoField(infoText.value, "Server", "uptime_in_seconds");
				return v != null ? formatUptime(parseInt(v, 10)) : "—";
			});
			const version = computed(() => infoField(infoText.value, "Server", "redis_version") || "—");
			const connectedClients = computed(() => infoField(infoText.value, "Clients", "connected_clients") || "—");
			const ops = computed(() => infoField(infoText.value, "Stats", "instantaneous_ops_per_sec") || "—");
			const dbKeys = computed(() => {
				const k = infoField(infoText.value, "Keyspace", "db" + store.currentDb);
				if (!k) return 0;
				const m = /keys=(\d+)/.exec(k);
				return m ? m[1] : "—";
			});
			const expires = computed(() => {
				const k = infoField(infoText.value, "Keyspace", "db" + store.currentDb);
				if (!k) return 0;
				const m = /expires=(\d+)/.exec(k);
				return m ? m[1] : "0";
			});
			const keyspaceRows = computed(() => {
				const sec = sections.value.find((s) => s.name === "Keyspace");
				return sec ? sec.fields : [];
			});
			function fmtTime(ts) {
				return (/* @__PURE__ */ new Date(ts * 1e3)).toLocaleString();
			}
			function fmtDur(sec) {
				if (sec < 60) return sec + "s";
				if (sec < 3600) return Math.floor(sec / 60) + "m";
				return Math.floor(sec / 3600) + "h";
			}
			async function load() {
				const c = getClient();
				if (!c) {
					toast("未连接，无法加载监控", "err");
					return;
				}
				loading.value = true;
				try {
					const [info, sl, cl] = await Promise.all([
						c.info(),
						c.slowlog(20),
						c.clientList()
					]);
					infoText.value = info;
					sections.value = parseInfo(info);
					slowlog.value = sl;
					clients.value = cl;
				} catch (e) {
					toast("监控加载失败: " + (e.message || e), "err");
				} finally {
					loading.value = false;
				}
			}
			onMounted(() => {
				if (store.connected) load();
			});
			return (_ctx, _cache) => {
				return openBlock(), createElementBlock("div", _hoisted_1$9, [createBaseVNode("div", _hoisted_2$9, [
					createBaseVNode("button", {
						class: normalizeClass(["tab", { active: tab.value === "overview" }]),
						type: "button",
						onClick: _cache[0] || (_cache[0] = ($event) => tab.value = "overview")
					}, "概览", 2),
					createBaseVNode("button", {
						class: normalizeClass(["tab", { active: tab.value === "info" }]),
						type: "button",
						onClick: _cache[1] || (_cache[1] = ($event) => tab.value = "info")
					}, "INFO", 2),
					createBaseVNode("button", {
						class: normalizeClass(["tab", { active: tab.value === "slowlog" }]),
						type: "button",
						onClick: _cache[2] || (_cache[2] = ($event) => tab.value = "slowlog")
					}, "慢日志", 2),
					createBaseVNode("button", {
						class: normalizeClass(["tab", { active: tab.value === "clients" }]),
						type: "button",
						onClick: _cache[3] || (_cache[3] = ($event) => tab.value = "clients")
					}, "客户端", 2),
					_cache[4] || (_cache[4] = createBaseVNode("span", { class: "spacer" }, null, -1)),
					createBaseVNode("button", {
						class: "btn sm",
						type: "button",
						onClick: load,
						disabled: loading.value
					}, "刷新", 8, _hoisted_3$9)
				]), createBaseVNode("div", _hoisted_4$8, [tab.value === "overview" ? (openBlock(), createElementBlock("div", _hoisted_5$8, [createBaseVNode("div", _hoisted_6$8, [
					createBaseVNode("div", _hoisted_7$8, [
						_cache[5] || (_cache[5] = createBaseVNode("div", { class: "label" }, "内存占用", -1)),
						createBaseVNode("div", _hoisted_8$6, toDisplayString(mem.value.usedText), 1),
						createBaseVNode("div", _hoisted_9$6, [createBaseVNode("i", { style: normalizeStyle({ width: mem.value.pct + "%" }) }, null, 4)]),
						createBaseVNode("div", _hoisted_10$5, "上限 " + toDisplayString(mem.value.maxText) + " · " + toDisplayString(mem.value.pct) + "%", 1)
					]),
					createBaseVNode("div", _hoisted_11$4, [_cache[6] || (_cache[6] = createBaseVNode("div", { class: "label" }, "运行时长", -1)), createBaseVNode("div", _hoisted_12$4, toDisplayString(uptime.value), 1)]),
					createBaseVNode("div", _hoisted_13$4, [_cache[7] || (_cache[7] = createBaseVNode("div", { class: "label" }, "客户端连接", -1)), createBaseVNode("div", _hoisted_14$4, toDisplayString(connectedClients.value), 1)]),
					createBaseVNode("div", _hoisted_15$4, [_cache[8] || (_cache[8] = createBaseVNode("div", { class: "label" }, "实时 QPS", -1)), createBaseVNode("div", _hoisted_16$3, toDisplayString(ops.value), 1)]),
					createBaseVNode("div", _hoisted_17$3, [
						_cache[9] || (_cache[9] = createBaseVNode("div", { class: "label" }, "当前库 key 数", -1)),
						createBaseVNode("div", _hoisted_18$3, toDisplayString(dbKeys.value), 1),
						createBaseVNode("div", _hoisted_19$3, "过期 " + toDisplayString(expires.value), 1)
					]),
					createBaseVNode("div", _hoisted_20$3, [_cache[10] || (_cache[10] = createBaseVNode("div", { class: "label" }, "Redis 版本", -1)), createBaseVNode("div", _hoisted_21$3, toDisplayString(version.value), 1)])
				]), createBaseVNode("div", _hoisted_22$3, [_cache[12] || (_cache[12] = createBaseVNode("h4", null, "Keyspace", -1)), createBaseVNode("table", _hoisted_23$3, [(openBlock(true), createElementBlock(Fragment, null, renderList(keyspaceRows.value, (row, i) => {
					return openBlock(), createElementBlock("tr", { key: i }, [createBaseVNode("td", null, toDisplayString(row.k), 1), createBaseVNode("td", null, toDisplayString(row.v), 1)]);
				}), 128)), !keyspaceRows.value.length ? (openBlock(), createElementBlock("tr", _hoisted_24$3, [..._cache[11] || (_cache[11] = [createBaseVNode("td", { colspan: "2" }, "无数据", -1)])])) : createCommentVNode("", true)])])])) : tab.value === "info" ? (openBlock(), createElementBlock("div", _hoisted_25$3, [(openBlock(true), createElementBlock(Fragment, null, renderList(sections.value, (sec) => {
					return openBlock(), createElementBlock("div", {
						key: sec.name,
						class: "info-section"
					}, [createBaseVNode("h4", null, toDisplayString(sec.name), 1), createBaseVNode("table", _hoisted_26$3, [(openBlock(true), createElementBlock(Fragment, null, renderList(sec.fields, (f, i) => {
						return openBlock(), createElementBlock("tr", { key: i }, [createBaseVNode("td", null, toDisplayString(f.k), 1), createBaseVNode("td", null, toDisplayString(f.v), 1)]);
					}), 128))])]);
				}), 128)), !sections.value.length ? (openBlock(), createElementBlock("div", _hoisted_27$3, "暂无数据，点击刷新")) : createCommentVNode("", true)])) : tab.value === "slowlog" ? (openBlock(), createElementBlock("div", _hoisted_28$2, [createBaseVNode("table", _hoisted_29$2, [_cache[14] || (_cache[14] = createBaseVNode("thead", null, [createBaseVNode("tr", null, [
					createBaseVNode("th", null, "#"),
					createBaseVNode("th", null, "时间"),
					createBaseVNode("th", null, "耗时(μs)"),
					createBaseVNode("th", null, "命令")
				])], -1)), createBaseVNode("tbody", null, [(openBlock(true), createElementBlock(Fragment, null, renderList(slowlog.value, (s) => {
					return openBlock(), createElementBlock("tr", { key: s.id }, [
						createBaseVNode("td", null, toDisplayString(s.id), 1),
						createBaseVNode("td", _hoisted_30$1, toDisplayString(fmtTime(s.timestamp)), 1),
						createBaseVNode("td", _hoisted_31$1, toDisplayString(s.durationUs), 1),
						createBaseVNode("td", _hoisted_32$1, toDisplayString(s.command), 1)
					]);
				}), 128)), !slowlog.value.length ? (openBlock(), createElementBlock("tr", _hoisted_33$1, [..._cache[13] || (_cache[13] = [createBaseVNode("td", {
					colspan: "4",
					class: "key-empty"
				}, "无慢日志", -1)])])) : createCommentVNode("", true)])])])) : tab.value === "clients" ? (openBlock(), createElementBlock("div", _hoisted_34, [createBaseVNode("table", _hoisted_35, [_cache[16] || (_cache[16] = createBaseVNode("thead", null, [createBaseVNode("tr", null, [
					createBaseVNode("th", null, "ID"),
					createBaseVNode("th", null, "地址"),
					createBaseVNode("th", null, "名称"),
					createBaseVNode("th", null, "库"),
					createBaseVNode("th", null, "时长"),
					createBaseVNode("th", null, "空闲"),
					createBaseVNode("th", null, "标志")
				])], -1)), createBaseVNode("tbody", null, [(openBlock(true), createElementBlock(Fragment, null, renderList(clients.value, (c) => {
					return openBlock(), createElementBlock("tr", { key: c.id }, [
						createBaseVNode("td", _hoisted_36, toDisplayString(c.id), 1),
						createBaseVNode("td", _hoisted_37, toDisplayString(c.addr), 1),
						createBaseVNode("td", null, toDisplayString(c.name || "—"), 1),
						createBaseVNode("td", null, toDisplayString(c.db), 1),
						createBaseVNode("td", _hoisted_38, toDisplayString(fmtDur(c.age)), 1),
						createBaseVNode("td", _hoisted_39, toDisplayString(fmtDur(c.idle)), 1),
						createBaseVNode("td", _hoisted_40, toDisplayString(c.flags), 1)
					]);
				}), 128)), !clients.value.length ? (openBlock(), createElementBlock("tr", _hoisted_41, [..._cache[15] || (_cache[15] = [createBaseVNode("td", {
					colspan: "7",
					class: "key-empty"
				}, "无客户端", -1)])])) : createCommentVNode("", true)])])])) : createCommentVNode("", true)])]);
			};
		}
	}), [["__scopeId", "data-v-1e457256"]]);
	//#endregion
	//#region src/components/MemoryAnalysisPanel.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$8 = { class: "pane-inner" };
	var _hoisted_2$8 = { class: "ops-head" };
	var _hoisted_3$8 = ["disabled"];
	var _hoisted_4$7 = { class: "muted" };
	var _hoisted_5$7 = { class: "info-section" };
	var _hoisted_6$7 = {
		key: 0,
		class: "vtable"
	};
	var _hoisted_7$7 = { class: "mono" };
	var _hoisted_8$5 = { class: "mono kname" };
	var _hoisted_9$5 = { class: "mono" };
	var _hoisted_10$4 = { class: "mono" };
	var _hoisted_11$3 = { class: "bar" };
	var _hoisted_12$3 = {
		key: 1,
		class: "key-empty"
	};
	var _hoisted_13$3 = { class: "two-col" };
	var _hoisted_14$3 = { class: "info-section" };
	var _hoisted_15$3 = {
		key: 0,
		class: "vtable"
	};
	var _hoisted_16$2 = { class: "mono" };
	var _hoisted_17$2 = { class: "mono" };
	var _hoisted_18$2 = { class: "mono" };
	var _hoisted_19$2 = { class: "mono" };
	var _hoisted_20$2 = {
		key: 1,
		class: "key-empty"
	};
	var _hoisted_21$2 = { class: "info-section" };
	var _hoisted_22$2 = {
		key: 0,
		class: "donut-wrap"
	};
	var _hoisted_23$2 = {
		viewBox: "0 0 120 120",
		class: "donut"
	};
	var _hoisted_24$2 = [
		"stroke",
		"stroke-dasharray",
		"stroke-dashoffset"
	];
	var _hoisted_25$2 = { class: "legend" };
	var _hoisted_26$2 = { class: "mono" };
	var _hoisted_27$2 = { class: "mono muted" };
	var _hoisted_28$1 = { class: "mono muted" };
	var _hoisted_29$1 = {
		key: 1,
		class: "key-empty"
	};
	var R = 48;
	//#endregion
	//#region src/components/MemoryAnalysisPanel.vue
	var MemoryAnalysisPanel_default = /*#__PURE__*/ _plugin_vue_export_helper_default(/* @__PURE__ */ defineComponent({
		__name: "MemoryAnalysisPanel",
		setup(__props) {
			const loading = /* @__PURE__ */ ref(false);
			const analysis = /* @__PURE__ */ ref(null);
			const bigKeys = computed(() => analysis.value?.bigKeys ?? []);
			const prefixes = computed(() => analysis.value?.prefixes ?? []);
			const types = computed(() => analysis.value?.types ?? []);
			const C = 2 * Math.PI * R;
			const donut = computed(() => {
				let acc = 0;
				return types.value.map((t) => {
					const start = acc;
					acc += t.pct;
					return {
						type: t.type,
						dash: t.pct / 100 * C,
						offset: -(start / 100) * C
					};
				});
			});
			function fmtBytes(n) {
				return formatBytes(n);
			}
			function color(t) {
				return TYPE_COLORS[t] || "#9aa0a6";
			}
			function pctOf(size) {
				const total = analysis.value?.totalBytes ?? 0;
				if (!total) return 0;
				return Math.max(2, size / total * 100);
			}
			async function load() {
				const c = getClient();
				if (!c) {
					toast("未连接", "err");
					return;
				}
				loading.value = true;
				try {
					analysis.value = await c.memoryAnalyze();
				} catch (e) {
					toast("内存分析失败: " + (e.message || e), "err");
				} finally {
					loading.value = false;
				}
			}
			onMounted(() => {
				if (store.connected) load();
			});
			return (_ctx, _cache) => {
				return openBlock(), createElementBlock("div", _hoisted_1$8, [
					createBaseVNode("div", _hoisted_2$8, [createBaseVNode("button", {
						class: "btn sm",
						type: "button",
						onClick: load,
						disabled: loading.value
					}, "刷新", 8, _hoisted_3$8), createBaseVNode("span", _hoisted_4$7, "当前库共 " + toDisplayString(analysis.value?.totalKeys ?? 0) + " 个 key · " + toDisplayString(fmtBytes(analysis.value?.totalBytes ?? 0)), 1)]),
					createBaseVNode("div", _hoisted_5$7, [createBaseVNode("h4", null, "大 Key 排名（Top " + toDisplayString(bigKeys.value.length) + "）", 1), bigKeys.value.length ? (openBlock(), createElementBlock("table", _hoisted_6$7, [_cache[0] || (_cache[0] = createBaseVNode("thead", null, [createBaseVNode("tr", null, [
						createBaseVNode("th", null, "#"),
						createBaseVNode("th", null, "Key"),
						createBaseVNode("th", null, "类型"),
						createBaseVNode("th", null, "内存"),
						createBaseVNode("th", null, "TTL"),
						createBaseVNode("th", { style: { "width": "30%" } }, "占比")
					])], -1)), createBaseVNode("tbody", null, [(openBlock(true), createElementBlock(Fragment, null, renderList(bigKeys.value, (k, i) => {
						return openBlock(), createElementBlock("tr", { key: k.name }, [
							createBaseVNode("td", _hoisted_7$7, toDisplayString(i + 1), 1),
							createBaseVNode("td", _hoisted_8$5, toDisplayString(k.name), 1),
							createBaseVNode("td", null, [createBaseVNode("span", {
								class: "type-badge",
								style: normalizeStyle({ background: color(k.type) })
							}, toDisplayString(k.type), 5)]),
							createBaseVNode("td", _hoisted_9$5, toDisplayString(fmtBytes(k.size)), 1),
							createBaseVNode("td", _hoisted_10$4, toDisplayString(k.ttl === -1 ? "永久" : k.ttl + "s"), 1),
							createBaseVNode("td", null, [createBaseVNode("div", _hoisted_11$3, [createBaseVNode("i", { style: normalizeStyle({
								width: pctOf(k.size) + "%",
								background: color(k.type)
							}) }, null, 4)])])
						]);
					}), 128))])])) : (openBlock(), createElementBlock("div", _hoisted_12$3, "暂无数据"))]),
					createBaseVNode("div", _hoisted_13$3, [createBaseVNode("div", _hoisted_14$3, [_cache[2] || (_cache[2] = createBaseVNode("h4", null, "按前缀内存占比", -1)), prefixes.value.length ? (openBlock(), createElementBlock("table", _hoisted_15$3, [_cache[1] || (_cache[1] = createBaseVNode("thead", null, [createBaseVNode("tr", null, [
						createBaseVNode("th", null, "前缀"),
						createBaseVNode("th", null, "数量"),
						createBaseVNode("th", null, "内存"),
						createBaseVNode("th", null, "占比")
					])], -1)), createBaseVNode("tbody", null, [(openBlock(true), createElementBlock(Fragment, null, renderList(prefixes.value, (p) => {
						return openBlock(), createElementBlock("tr", { key: p.prefix }, [
							createBaseVNode("td", _hoisted_16$2, toDisplayString(p.prefix), 1),
							createBaseVNode("td", _hoisted_17$2, toDisplayString(p.count), 1),
							createBaseVNode("td", _hoisted_18$2, toDisplayString(fmtBytes(p.size)), 1),
							createBaseVNode("td", _hoisted_19$2, toDisplayString(p.pct) + "%", 1)
						]);
					}), 128))])])) : (openBlock(), createElementBlock("div", _hoisted_20$2, "暂无数据"))]), createBaseVNode("div", _hoisted_21$2, [_cache[4] || (_cache[4] = createBaseVNode("h4", null, "按类型内存分布", -1)), types.value.length ? (openBlock(), createElementBlock("div", _hoisted_22$2, [(openBlock(), createElementBlock("svg", _hoisted_23$2, [_cache[3] || (_cache[3] = createBaseVNode("circle", {
						cx: "60",
						cy: "60",
						r: "48",
						fill: "none",
						stroke: "var(--border)",
						"stroke-width": "14"
					}, null, -1)), (openBlock(true), createElementBlock(Fragment, null, renderList(donut.value, (s, i) => {
						return openBlock(), createElementBlock("circle", {
							key: i,
							cx: "60",
							cy: "60",
							r: "48",
							fill: "none",
							stroke: color(s.type),
							"stroke-width": "14",
							"stroke-dasharray": `${s.dash} ${C}`,
							"stroke-dashoffset": s.offset,
							transform: "rotate(-90 60 60)"
						}, null, 8, _hoisted_24$2);
					}), 128))])), createBaseVNode("div", _hoisted_25$2, [(openBlock(true), createElementBlock(Fragment, null, renderList(types.value, (t) => {
						return openBlock(), createElementBlock("div", {
							key: t.type,
							class: "legend-row"
						}, [
							createBaseVNode("i", {
								class: "dot",
								style: normalizeStyle({ background: color(t.type) })
							}, null, 4),
							createBaseVNode("span", _hoisted_26$2, toDisplayString(t.type), 1),
							createBaseVNode("span", _hoisted_27$2, toDisplayString(t.pct) + "%", 1),
							createBaseVNode("span", _hoisted_28$1, toDisplayString(fmtBytes(t.size)), 1)
						]);
					}), 128))])])) : (openBlock(), createElementBlock("div", _hoisted_29$1, "暂无数据"))])])
				]);
			};
		}
	}), [["__scopeId", "data-v-4f79d5a3"]]);
	//#endregion
	//#region src/components/SlowQueryPanel.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$7 = { class: "pane-inner" };
	var _hoisted_2$7 = { class: "ops-head" };
	var _hoisted_3$7 = ["disabled"];
	var _hoisted_4$6 = { class: "muted" };
	var _hoisted_5$6 = { class: "monitor-grid" };
	var _hoisted_6$6 = { class: "metric-card" };
	var _hoisted_7$6 = { class: "value" };
	var _hoisted_8$4 = { class: "metric-card" };
	var _hoisted_9$4 = { class: "value danger" };
	var _hoisted_10$3 = { class: "metric-card" };
	var _hoisted_11$2 = { class: "value" };
	var _hoisted_12$2 = { class: "metric-card" };
	var _hoisted_13$2 = { class: "value warn" };
	var _hoisted_14$2 = { class: "info-section" };
	var _hoisted_15$2 = {
		key: 0,
		class: "vtable"
	};
	var _hoisted_16$1 = { class: "mono" };
	var _hoisted_17$1 = { class: "mono" };
	var _hoisted_18$1 = { class: "mono" };
	var _hoisted_19$1 = { class: "mono" };
	var _hoisted_20$1 = { class: "bar" };
	var _hoisted_21$1 = { class: "info-section" };
	var _hoisted_22$1 = {
		key: 0,
		class: "vtable"
	};
	var _hoisted_23$1 = { class: "mono" };
	var _hoisted_24$1 = { class: "mono" };
	var _hoisted_25$1 = { class: "mono" };
	var _hoisted_26$1 = { class: "mono muted" };
	var _hoisted_27$1 = {
		key: 1,
		class: "key-empty"
	};
	//#endregion
	//#region src/components/SlowQueryPanel.vue
	var SlowQueryPanel_default = /*#__PURE__*/ _plugin_vue_export_helper_default(/* @__PURE__ */ defineComponent({
		__name: "SlowQueryPanel",
		setup(__props) {
			const loading = /* @__PURE__ */ ref(false);
			const analysis = /* @__PURE__ */ ref(null);
			const items = computed(() => analysis.value?.items ?? []);
			const byCommand = computed(() => analysis.value?.byCommand ?? []);
			function pctOfTotal(us) {
				const total = byCommand.value.reduce((s, g) => s + g.totalUs, 0);
				if (!total) return 0;
				return Math.max(2, us / total * 100);
			}
			function fmtTime(ts) {
				return (/* @__PURE__ */ new Date(ts * 1e3)).toLocaleTimeString();
			}
			async function load() {
				const c = getClient();
				if (!c) {
					toast("未连接", "err");
					return;
				}
				loading.value = true;
				try {
					analysis.value = await c.slowQueryAnalysis(20);
				} catch (e) {
					toast("慢查询分析失败: " + (e.message || e), "err");
				} finally {
					loading.value = false;
				}
			}
			onMounted(() => {
				if (store.connected) load();
			});
			return (_ctx, _cache) => {
				return openBlock(), createElementBlock("div", _hoisted_1$7, [
					createBaseVNode("div", _hoisted_2$7, [createBaseVNode("button", {
						class: "btn sm",
						type: "button",
						onClick: load,
						disabled: loading.value
					}, "刷新", 8, _hoisted_3$7), createBaseVNode("span", _hoisted_4$6, "采样 " + toDisplayString(analysis.value?.count ?? 0) + " 条慢查询（μs = 微秒）", 1)]),
					createBaseVNode("div", _hoisted_5$6, [
						createBaseVNode("div", _hoisted_6$6, [_cache[0] || (_cache[0] = createBaseVNode("div", { class: "label" }, "慢查询数", -1)), createBaseVNode("div", _hoisted_7$6, toDisplayString(analysis.value?.count ?? 0), 1)]),
						createBaseVNode("div", _hoisted_8$4, [_cache[2] || (_cache[2] = createBaseVNode("div", { class: "label" }, "最大耗时", -1)), createBaseVNode("div", _hoisted_9$4, [createTextVNode(toDisplayString(analysis.value?.maxUs ?? 0), 1), _cache[1] || (_cache[1] = createBaseVNode("small", null, "μs", -1))])]),
						createBaseVNode("div", _hoisted_10$3, [_cache[4] || (_cache[4] = createBaseVNode("div", { class: "label" }, "平均耗时", -1)), createBaseVNode("div", _hoisted_11$2, [createTextVNode(toDisplayString(analysis.value?.avgUs ?? 0), 1), _cache[3] || (_cache[3] = createBaseVNode("small", null, "μs", -1))])]),
						createBaseVNode("div", _hoisted_12$2, [_cache[6] || (_cache[6] = createBaseVNode("div", { class: "label" }, "P95 耗时", -1)), createBaseVNode("div", _hoisted_13$2, [createTextVNode(toDisplayString(analysis.value?.p95Us ?? 0), 1), _cache[5] || (_cache[5] = createBaseVNode("small", null, "μs", -1))])])
					]),
					createBaseVNode("div", _hoisted_14$2, [_cache[8] || (_cache[8] = createBaseVNode("h4", null, "命令热点（按累计耗时）", -1)), byCommand.value.length ? (openBlock(), createElementBlock("table", _hoisted_15$2, [_cache[7] || (_cache[7] = createBaseVNode("thead", null, [createBaseVNode("tr", null, [
						createBaseVNode("th", null, "命令模式"),
						createBaseVNode("th", null, "次数"),
						createBaseVNode("th", null, "最大μs"),
						createBaseVNode("th", null, "累计μs"),
						createBaseVNode("th", { style: { "width": "34%" } }, "占比")
					])], -1)), createBaseVNode("tbody", null, [(openBlock(true), createElementBlock(Fragment, null, renderList(byCommand.value, (g) => {
						return openBlock(), createElementBlock("tr", { key: g.pattern }, [
							createBaseVNode("td", _hoisted_16$1, toDisplayString(g.pattern), 1),
							createBaseVNode("td", _hoisted_17$1, toDisplayString(g.count), 1),
							createBaseVNode("td", _hoisted_18$1, toDisplayString(g.maxUs), 1),
							createBaseVNode("td", _hoisted_19$1, toDisplayString(g.totalUs), 1),
							createBaseVNode("td", null, [createBaseVNode("div", _hoisted_20$1, [createBaseVNode("i", { style: normalizeStyle({ width: pctOfTotal(g.totalUs) + "%" }) }, null, 4)])])
						]);
					}), 128))])])) : createCommentVNode("", true)]),
					createBaseVNode("div", _hoisted_21$1, [_cache[10] || (_cache[10] = createBaseVNode("h4", null, "慢查询明细", -1)), items.value.length ? (openBlock(), createElementBlock("table", _hoisted_22$1, [_cache[9] || (_cache[9] = createBaseVNode("thead", null, [createBaseVNode("tr", null, [
						createBaseVNode("th", null, "#"),
						createBaseVNode("th", null, "时间"),
						createBaseVNode("th", null, "耗时μs"),
						createBaseVNode("th", null, "原始命令"),
						createBaseVNode("th", null, "归一化模式")
					])], -1)), createBaseVNode("tbody", null, [(openBlock(true), createElementBlock(Fragment, null, renderList(items.value, (s) => {
						return openBlock(), createElementBlock("tr", {
							key: s.id,
							class: normalizeClass({ hot: s.durationUs >= 1e4 })
						}, [
							createBaseVNode("td", _hoisted_23$1, toDisplayString(s.id), 1),
							createBaseVNode("td", _hoisted_24$1, toDisplayString(fmtTime(s.timestamp)), 1),
							createBaseVNode("td", { class: normalizeClass(["mono", s.durationUs >= 1e4 ? "danger" : ""]) }, toDisplayString(s.durationUs), 3),
							createBaseVNode("td", _hoisted_25$1, toDisplayString(s.command), 1),
							createBaseVNode("td", _hoisted_26$1, toDisplayString(s.pattern), 1)
						], 2);
					}), 128))])])) : (openBlock(), createElementBlock("div", _hoisted_27$1, "暂无慢查询"))])
				]);
			};
		}
	}), [["__scopeId", "data-v-e0286a89"]]);
	//#endregion
	//#region src/components/ClusterPanel.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$6 = { class: "pane-inner" };
	var _hoisted_2$6 = { class: "ops-head" };
	var _hoisted_3$6 = ["disabled"];
	var _hoisted_4$5 = { class: "info-section" };
	var _hoisted_5$5 = {
		key: 0,
		class: "kv-grid"
	};
	var _hoisted_6$5 = { class: "kv" };
	var _hoisted_7$5 = { class: "kv" };
	var _hoisted_8$3 = { class: "kv" };
	var _hoisted_9$3 = { class: "mono" };
	var _hoisted_10$2 = {
		key: 1,
		class: "vtable"
	};
	var _hoisted_11$1 = { class: "mono" };
	var _hoisted_12$1 = { class: "mono" };
	var _hoisted_13$1 = { class: "mono" };
	var _hoisted_14$1 = { class: "mono" };
	var _hoisted_15$1 = { class: "ok-badge" };
	var _hoisted_16 = { class: "info-section" };
	var _hoisted_17 = {
		key: 0,
		class: "vtable"
	};
	var _hoisted_18 = { class: "mono" };
	var _hoisted_19 = { class: "mono" };
	var _hoisted_20 = { class: "mono" };
	var _hoisted_21 = { class: "mono" };
	var _hoisted_22 = { class: "ok-badge" };
	var _hoisted_23 = {
		key: 1,
		class: "slot-bar"
	};
	var _hoisted_24 = ["title"];
	var _hoisted_25 = {
		key: 2,
		class: "legend"
	};
	var _hoisted_26 = { class: "info-section" };
	var _hoisted_27 = {
		key: 0,
		class: "vtable"
	};
	var _hoisted_28 = { class: "mono" };
	var _hoisted_29 = { class: "mono" };
	var _hoisted_30 = { class: "mono" };
	var _hoisted_31 = { class: "mono" };
	var _hoisted_32 = { class: "mono" };
	var _hoisted_33 = {
		key: 1,
		class: "key-empty"
	};
	//#endregion
	//#region src/components/ClusterPanel.vue
	var ClusterPanel_default = /*#__PURE__*/ _plugin_vue_export_helper_default(/* @__PURE__ */ defineComponent({
		__name: "ClusterPanel",
		setup(__props) {
			const loading = /* @__PURE__ */ ref(false);
			const topo = /* @__PURE__ */ ref(null);
			const repl = /* @__PURE__ */ ref(null);
			const sentinels = /* @__PURE__ */ ref([]);
			const modeLabel = computed(() => {
				const m = topo.value?.mode;
				return m === "cluster" ? "cluster 模式" : m === "sentinel" ? "sentinel 模式" : "standalone";
			});
			const masterNodes = computed(() => (topo.value?.nodes ?? []).filter((n) => n.role !== "slave"));
			function short(id) {
				return id.slice(0, 8);
			}
			function slotWidth(range) {
				if (range === "-" || !range.includes("-")) return 1;
				const [a, b] = range.split("-").map((x) => parseInt(x, 10));
				return b - a + 1;
			}
			const SLOT_COLORS = [
				"#2f6feb",
				"#1a7f37",
				"#b8860b",
				"#8957e5",
				"#c0392b",
				"#0d9488"
			];
			function slotColor(i) {
				return SLOT_COLORS[i % SLOT_COLORS.length];
			}
			async function load() {
				const c = getClient();
				if (!c) {
					toast("未连接", "err");
					return;
				}
				loading.value = true;
				try {
					const [t, r, s] = await Promise.all([
						c.clusterTopology(),
						c.replicationInfo(),
						c.sentinelMasters()
					]);
					topo.value = t;
					repl.value = r;
					sentinels.value = s;
				} catch (e) {
					toast("集群信息加载失败: " + (e.message || e), "err");
				} finally {
					loading.value = false;
				}
			}
			onMounted(() => {
				if (store.connected) load();
			});
			return (_ctx, _cache) => {
				return openBlock(), createElementBlock("div", _hoisted_1$6, [
					createBaseVNode("div", _hoisted_2$6, [
						createBaseVNode("button", {
							class: "btn sm",
							type: "button",
							onClick: load,
							disabled: loading.value
						}, "刷新", 8, _hoisted_3$6),
						createBaseVNode("span", { class: normalizeClass(["mode-badge", topo.value?.mode]) }, toDisplayString(modeLabel.value), 3),
						_cache[0] || (_cache[0] = createBaseVNode("span", { class: "muted" }, "以上为 Mock 合成的集群/复制演示数据", -1))
					]),
					createBaseVNode("div", _hoisted_4$5, [
						_cache[5] || (_cache[5] = createBaseVNode("h4", null, "主从复制", -1)),
						repl.value ? (openBlock(), createElementBlock("div", _hoisted_5$5, [
							createBaseVNode("div", _hoisted_6$5, [_cache[1] || (_cache[1] = createBaseVNode("span", null, "角色", -1)), createBaseVNode("b", null, toDisplayString(repl.value.role), 1)]),
							createBaseVNode("div", _hoisted_7$5, [_cache[2] || (_cache[2] = createBaseVNode("span", null, "已连接从节点", -1)), createBaseVNode("b", null, toDisplayString(repl.value.connectedSlaves), 1)]),
							createBaseVNode("div", _hoisted_8$3, [_cache[3] || (_cache[3] = createBaseVNode("span", null, "复制偏移", -1)), createBaseVNode("b", _hoisted_9$3, toDisplayString(repl.value.masterReplOffset), 1)])
						])) : createCommentVNode("", true),
						repl.value && repl.value.slaves.length ? (openBlock(), createElementBlock("table", _hoisted_10$2, [_cache[4] || (_cache[4] = createBaseVNode("thead", null, [createBaseVNode("tr", null, [
							createBaseVNode("th", null, "ID"),
							createBaseVNode("th", null, "地址"),
							createBaseVNode("th", null, "偏移"),
							createBaseVNode("th", null, "延迟(s)"),
							createBaseVNode("th", null, "状态")
						])], -1)), createBaseVNode("tbody", null, [(openBlock(true), createElementBlock(Fragment, null, renderList(repl.value.slaves, (s) => {
							return openBlock(), createElementBlock("tr", { key: s.id }, [
								createBaseVNode("td", _hoisted_11$1, toDisplayString(s.id), 1),
								createBaseVNode("td", _hoisted_12$1, toDisplayString(s.addr), 1),
								createBaseVNode("td", _hoisted_13$1, toDisplayString(s.offset), 1),
								createBaseVNode("td", _hoisted_14$1, toDisplayString(s.lag), 1),
								createBaseVNode("td", null, [createBaseVNode("span", _hoisted_15$1, toDisplayString(s.state), 1)])
							]);
						}), 128))])])) : createCommentVNode("", true)
					]),
					createBaseVNode("div", _hoisted_16, [
						createBaseVNode("h4", null, "集群拓扑（" + toDisplayString(topo.value?.nodeCount ?? 0) + " 节点 · " + toDisplayString(topo.value?.slotsCovered ?? 0) + " 槽位）", 1),
						topo.value ? (openBlock(), createElementBlock("table", _hoisted_17, [_cache[6] || (_cache[6] = createBaseVNode("thead", null, [createBaseVNode("tr", null, [
							createBaseVNode("th", null, "节点"),
							createBaseVNode("th", null, "角色"),
							createBaseVNode("th", null, "地址"),
							createBaseVNode("th", null, "槽位"),
							createBaseVNode("th", null, "主节点"),
							createBaseVNode("th", null, "状态")
						])], -1)), createBaseVNode("tbody", null, [(openBlock(true), createElementBlock(Fragment, null, renderList(topo.value.nodes, (n) => {
							return openBlock(), createElementBlock("tr", { key: n.id }, [
								createBaseVNode("td", _hoisted_18, toDisplayString(short(n.id)) + toDisplayString(n.role === "myself" ? " ★" : ""), 1),
								createBaseVNode("td", null, [createBaseVNode("span", { class: normalizeClass(["role-badge", n.role]) }, toDisplayString(n.role), 3)]),
								createBaseVNode("td", _hoisted_19, toDisplayString(n.addr), 1),
								createBaseVNode("td", _hoisted_20, toDisplayString(n.slots), 1),
								createBaseVNode("td", _hoisted_21, toDisplayString(n.masterId ? short(n.masterId) : "—"), 1),
								createBaseVNode("td", null, [createBaseVNode("span", _hoisted_22, toDisplayString(n.linkState), 1)])
							]);
						}), 128))])])) : createCommentVNode("", true),
						topo.value ? (openBlock(), createElementBlock("div", _hoisted_23, [(openBlock(true), createElementBlock(Fragment, null, renderList(masterNodes.value, (n, i) => {
							return openBlock(), createElementBlock("div", {
								key: n.id,
								class: "slot-seg",
								style: normalizeStyle({
									flex: slotWidth(n.slots),
									background: slotColor(i)
								}),
								title: n.addr + " " + n.slots
							}, null, 12, _hoisted_24);
						}), 128))])) : createCommentVNode("", true),
						topo.value ? (openBlock(), createElementBlock("div", _hoisted_25, [(openBlock(true), createElementBlock(Fragment, null, renderList(masterNodes.value, (n, i) => {
							return openBlock(), createElementBlock("span", {
								key: n.id,
								class: "legend-row"
							}, [createBaseVNode("i", {
								class: "dot",
								style: normalizeStyle({ background: slotColor(i) })
							}, null, 4), createTextVNode(toDisplayString(n.addr) + " (" + toDisplayString(n.slots) + ") ", 1)]);
						}), 128))])) : createCommentVNode("", true)
					]),
					createBaseVNode("div", _hoisted_26, [_cache[8] || (_cache[8] = createBaseVNode("h4", null, "Sentinel 监控的主节点", -1)), sentinels.value.length ? (openBlock(), createElementBlock("table", _hoisted_27, [_cache[7] || (_cache[7] = createBaseVNode("thead", null, [createBaseVNode("tr", null, [
						createBaseVNode("th", null, "名称"),
						createBaseVNode("th", null, "地址"),
						createBaseVNode("th", null, "从节点"),
						createBaseVNode("th", null, "其他 Sentinel"),
						createBaseVNode("th", null, "Quorum")
					])], -1)), createBaseVNode("tbody", null, [(openBlock(true), createElementBlock(Fragment, null, renderList(sentinels.value, (m) => {
						return openBlock(), createElementBlock("tr", { key: m.name }, [
							createBaseVNode("td", _hoisted_28, toDisplayString(m.name), 1),
							createBaseVNode("td", _hoisted_29, toDisplayString(m.ip) + ":" + toDisplayString(m.port), 1),
							createBaseVNode("td", _hoisted_30, toDisplayString(m.numSlaves), 1),
							createBaseVNode("td", _hoisted_31, toDisplayString(m.numOtherSentinels), 1),
							createBaseVNode("td", _hoisted_32, toDisplayString(m.quorum), 1)
						]);
					}), 128))])])) : (openBlock(), createElementBlock("div", _hoisted_33, "无 Sentinel 数据"))])
				]);
			};
		}
	}), [["__scopeId", "data-v-d7fe50c2"]]);
	//#endregion
	//#region src/components/AuditPanel.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$5 = { class: "pane-inner" };
	var _hoisted_2$5 = { class: "ops-head" };
	var _hoisted_3$5 = { class: "muted" };
	var _hoisted_4$4 = ["value"];
	var _hoisted_5$4 = {
		key: 0,
		class: "audit-list"
	};
	var _hoisted_6$4 = { class: "ts mono" };
	var _hoisted_7$4 = {
		key: 0,
		class: "target mono"
	};
	var _hoisted_8$2 = { class: "detail" };
	var _hoisted_9$2 = {
		key: 1,
		class: "key-empty"
	};
	//#endregion
	//#region src/components/AuditPanel.vue
	var AuditPanel_default = /*#__PURE__*/ _plugin_vue_export_helper_default(/* @__PURE__ */ defineComponent({
		__name: "AuditPanel",
		setup(__props) {
			const filter = /* @__PURE__ */ ref("");
			const expFmt = /* @__PURE__ */ ref("json");
			const ops = [
				"CMD",
				"DEL",
				"BATCH_DELETE",
				"BATCH_EXPIRE",
				"BATCH_PERSIST",
				"RENAME",
				"COPY",
				"EXPIRE",
				"PERSIST",
				"IMPORT",
				"EXPORT"
			];
			const filtered = computed(() => filter.value ? store.audit.filter((e) => e.op === filter.value) : store.audit);
			function fmtTime(ts) {
				const d = new Date(ts);
				const p = (n) => String(n).padStart(2, "0");
				return `${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
			}
			function doExport() {
				try {
					const text = exportAudit(expFmt.value);
					const blob = new Blob([text], { type: "text/plain;charset=utf-8" });
					const url = URL.createObjectURL(blob);
					const a = document.createElement("a");
					a.href = url;
					a.download = `redis-audit.${expFmt.value}`;
					document.body.appendChild(a);
					a.click();
					document.body.removeChild(a);
					setTimeout(() => URL.revokeObjectURL(url), 1e3);
					toast("审计日志已导出", "ok");
				} catch (e) {
					toast("导出失败: " + (e.message || e), "err");
				}
			}
			return (_ctx, _cache) => {
				return openBlock(), createElementBlock("div", _hoisted_1$5, [createBaseVNode("div", _hoisted_2$5, [
					createBaseVNode("span", _hoisted_3$5, "共 " + toDisplayString(unref(store).audit.length) + " 条操作记录（按时间倒序）", 1),
					_cache[5] || (_cache[5] = createBaseVNode("span", { class: "spacer" }, null, -1)),
					withDirectives(createBaseVNode("select", {
						class: "select small",
						"onUpdate:modelValue": _cache[0] || (_cache[0] = ($event) => filter.value = $event)
					}, [_cache[3] || (_cache[3] = createBaseVNode("option", { value: "" }, "全部操作", -1)), (openBlock(), createElementBlock(Fragment, null, renderList(ops, (op) => {
						return createBaseVNode("option", {
							key: op,
							value: op
						}, toDisplayString(op), 9, _hoisted_4$4);
					}), 64))], 512), [[vModelSelect, filter.value]]),
					withDirectives(createBaseVNode("select", {
						class: "select small",
						"onUpdate:modelValue": _cache[1] || (_cache[1] = ($event) => expFmt.value = $event)
					}, [..._cache[4] || (_cache[4] = [createBaseVNode("option", { value: "json" }, "JSON", -1), createBaseVNode("option", { value: "csv" }, "CSV", -1)])], 512), [[vModelSelect, expFmt.value]]),
					createBaseVNode("button", {
						class: "btn sm",
						type: "button",
						onClick: doExport
					}, "导出"),
					createBaseVNode("button", {
						class: "btn sm danger",
						type: "button",
						onClick: _cache[2] || (_cache[2] = (...args) => unref(clearAudit) && unref(clearAudit)(...args))
					}, "清空")
				]), filtered.value.length ? (openBlock(), createElementBlock("div", _hoisted_5$4, [(openBlock(true), createElementBlock(Fragment, null, renderList(filtered.value, (e, i) => {
					return openBlock(), createElementBlock("div", {
						key: i,
						class: normalizeClass(["audit-row", e.status])
					}, [
						createBaseVNode("span", _hoisted_6$4, toDisplayString(fmtTime(e.ts)), 1),
						createBaseVNode("span", { class: normalizeClass(["op", "op-" + e.op]) }, toDisplayString(e.op), 3),
						e.target !== "-" ? (openBlock(), createElementBlock("span", _hoisted_7$4, toDisplayString(e.target), 1)) : createCommentVNode("", true),
						createBaseVNode("span", _hoisted_8$2, toDisplayString(e.detail), 1),
						createBaseVNode("span", { class: normalizeClass(["st", e.status]) }, toDisplayString(e.status === "ok" ? "✓" : "✗"), 3)
					], 2);
				}), 128))])) : (openBlock(), createElementBlock("div", _hoisted_9$2, "暂无审计记录，执行任意命令或操作后此处会出现记录"))]);
			};
		}
	}), [["__scopeId", "data-v-38014037"]]);
	//#endregion
	//#region src/components/OpsView.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$4 = { class: "ops" };
	var _hoisted_2$4 = { class: "monitor-tabs" };
	var _hoisted_3$4 = { class: "ops-body" };
	//#endregion
	//#region src/components/OpsView.vue
	var OpsView_default = /*#__PURE__*/ _plugin_vue_export_helper_default(/* @__PURE__ */ defineComponent({
		__name: "OpsView",
		setup(__props) {
			const tab = /* @__PURE__ */ ref("memory");
			return (_ctx, _cache) => {
				return openBlock(), createElementBlock("div", _hoisted_1$4, [createBaseVNode("div", _hoisted_2$4, [
					createBaseVNode("button", {
						class: normalizeClass(["tab", { active: tab.value === "memory" }]),
						type: "button",
						onClick: _cache[0] || (_cache[0] = ($event) => tab.value = "memory")
					}, "内存分析", 2),
					createBaseVNode("button", {
						class: normalizeClass(["tab", { active: tab.value === "slow" }]),
						type: "button",
						onClick: _cache[1] || (_cache[1] = ($event) => tab.value = "slow")
					}, "慢查询分析", 2),
					createBaseVNode("button", {
						class: normalizeClass(["tab", { active: tab.value === "cluster" }]),
						type: "button",
						onClick: _cache[2] || (_cache[2] = ($event) => tab.value = "cluster")
					}, "集群/复制", 2),
					createBaseVNode("button", {
						class: normalizeClass(["tab", { active: tab.value === "audit" }]),
						type: "button",
						onClick: _cache[3] || (_cache[3] = ($event) => tab.value = "audit")
					}, "操作审计", 2),
					_cache[4] || (_cache[4] = createBaseVNode("span", { class: "spacer" }, null, -1)),
					_cache[5] || (_cache[5] = createBaseVNode("span", { class: "mock-badge" }, "P2 · Mock 演示", -1))
				]), createBaseVNode("div", _hoisted_3$4, [tab.value === "memory" ? (openBlock(), createBlock(MemoryAnalysisPanel_default, { key: 0 })) : tab.value === "slow" ? (openBlock(), createBlock(SlowQueryPanel_default, { key: 1 })) : tab.value === "cluster" ? (openBlock(), createBlock(ClusterPanel_default, { key: 2 })) : tab.value === "audit" ? (openBlock(), createBlock(AuditPanel_default, { key: 3 })) : createCommentVNode("", true)])]);
			};
		}
	}), [["__scopeId", "data-v-2de46524"]]);
	//#endregion
	//#region src/components/ConnDialog.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$3 = { class: "field" };
	var _hoisted_2$3 = { class: "field-row" };
	var _hoisted_3$3 = { class: "field" };
	var _hoisted_4$3 = {
		class: "field",
		style: { "max-width": "110px" }
	};
	var _hoisted_5$3 = { class: "field" };
	var _hoisted_6$3 = { class: "field-row" };
	var _hoisted_7$3 = {
		class: "field",
		style: { "max-width": "110px" }
	};
	var _hoisted_8$1 = { class: "field" };
	var _hoisted_9$1 = {
		key: 0,
		class: "field"
	};
	var _hoisted_10$1 = { class: "chk" };
	var _hoisted_11 = { class: "field reserved" };
	var _hoisted_12 = { class: "chk" };
	var _hoisted_13 = { class: "field reserved" };
	var _hoisted_14 = { class: "field reserved" };
	var _hoisted_15 = { class: "field-row" };
	//#endregion
	//#region src/components/ConnDialog.vue
	var ConnDialog_default = /* @__PURE__ */ defineComponent({
		__name: "ConnDialog",
		props: { conn: {} },
		emits: ["close"],
		setup(__props, { emit: __emit }) {
			const props = __props;
			const emit = __emit;
			const form = /* @__PURE__ */ reactive({
				id: "",
				name: "",
				host: "127.0.0.1",
				port: 6379,
				password: "",
				db: 0,
				mode: "mock",
				seed: true,
				...props.conn || {}
			});
			const ssh = /* @__PURE__ */ reactive({
				host: "",
				user: ""
			});
			async function save() {
				if (!form.name.trim()) {
					toast("请填写名称", "err");
					return;
				}
				const isEdit = !!props.conn;
				if (!isEdit) form.id = newConnId();
				if (isEdit) updateConnection({ ...form });
				else addConnection({ ...form });
				await connectById(form.id);
				toast("已保存并连接", "ok");
				emit("close");
			}
			return (_ctx, _cache) => {
				return openBlock(), createBlock(Modal_default, {
					title: "连接配置",
					onClose: _cache[12] || (_cache[12] = ($event) => _ctx.$emit("close"))
				}, {
					footer: withCtx(() => [createBaseVNode("button", {
						class: "btn",
						type: "button",
						onClick: _cache[11] || (_cache[11] = ($event) => _ctx.$emit("close"))
					}, "取消"), createBaseVNode("button", {
						class: "btn primary",
						type: "button",
						onClick: save
					}, "保存")]),
					default: withCtx(() => [
						createBaseVNode("div", _hoisted_1$3, [_cache[13] || (_cache[13] = createBaseVNode("label", null, "名称", -1)), withDirectives(createBaseVNode("input", {
							class: "input",
							"onUpdate:modelValue": _cache[0] || (_cache[0] = ($event) => form.name = $event),
							placeholder: "我的 Redis"
						}, null, 512), [[vModelText, form.name]])]),
						createBaseVNode("div", _hoisted_2$3, [createBaseVNode("div", _hoisted_3$3, [_cache[14] || (_cache[14] = createBaseVNode("label", null, "主机", -1)), withDirectives(createBaseVNode("input", {
							class: "input",
							"onUpdate:modelValue": _cache[1] || (_cache[1] = ($event) => form.host = $event),
							placeholder: "127.0.0.1"
						}, null, 512), [[vModelText, form.host]])]), createBaseVNode("div", _hoisted_4$3, [_cache[15] || (_cache[15] = createBaseVNode("label", null, "端口", -1)), withDirectives(createBaseVNode("input", {
							class: "input",
							"onUpdate:modelValue": _cache[2] || (_cache[2] = ($event) => form.port = $event),
							type: "number"
						}, null, 512), [[
							vModelText,
							form.port,
							void 0,
							{ number: true }
						]])])]),
						createBaseVNode("div", _hoisted_5$3, [_cache[16] || (_cache[16] = createBaseVNode("label", null, "密码（可选）", -1)), withDirectives(createBaseVNode("input", {
							class: "input",
							"onUpdate:modelValue": _cache[3] || (_cache[3] = ($event) => form.password = $event),
							type: "password",
							placeholder: "留空表示无密码"
						}, null, 512), [[vModelText, form.password]])]),
						createBaseVNode("div", _hoisted_6$3, [createBaseVNode("div", _hoisted_7$3, [_cache[17] || (_cache[17] = createBaseVNode("label", null, "默认库", -1)), withDirectives(createBaseVNode("input", {
							class: "input",
							"onUpdate:modelValue": _cache[4] || (_cache[4] = ($event) => form.db = $event),
							type: "number",
							min: "0",
							max: "15"
						}, null, 512), [[
							vModelText,
							form.db,
							void 0,
							{ number: true }
						]])]), createBaseVNode("div", _hoisted_8$1, [_cache[19] || (_cache[19] = createBaseVNode("label", null, "连接方式", -1)), withDirectives(createBaseVNode("select", {
							class: "select",
							"onUpdate:modelValue": _cache[5] || (_cache[5] = ($event) => form.mode = $event)
						}, [..._cache[18] || (_cache[18] = [
							createBaseVNode("option", { value: "mock" }, "本地 Mock（内置引擎，无需真实服务）", -1),
							createBaseVNode("option", { value: "tcp" }, "直连 TCP（真实后端）", -1),
							createBaseVNode("option", { value: "proxy" }, "伴随代理（预留）", -1)
						])], 512), [[vModelSelect, form.mode]])])]),
						form.mode === "mock" ? (openBlock(), createElementBlock("div", _hoisted_9$1, [createBaseVNode("label", _hoisted_10$1, [withDirectives(createBaseVNode("input", {
							type: "checkbox",
							"onUpdate:modelValue": _cache[6] || (_cache[6] = ($event) => form.seed = $event)
						}, null, 512), [[vModelCheckbox, form.seed]]), _cache[20] || (_cache[20] = createTextVNode(" 注入演示数据", -1))])])) : createCommentVNode("", true),
						createBaseVNode("div", _hoisted_11, [createBaseVNode("label", _hoisted_12, [
							withDirectives(createBaseVNode("input", {
								type: "checkbox",
								"onUpdate:modelValue": _cache[7] || (_cache[7] = ($event) => form.tls = $event)
							}, null, 512), [[vModelCheckbox, form.tls]]),
							_cache[21] || (_cache[21] = createTextVNode(" 启用 SSL/TLS ", -1)),
							_cache[22] || (_cache[22] = createBaseVNode("span", { class: "tag-reserved" }, "预留", -1))
						])]),
						createBaseVNode("div", _hoisted_13, [_cache[23] || (_cache[23] = createBaseVNode("label", null, [createTextVNode("Unix Socket 路径"), createBaseVNode("span", { class: "tag-reserved" }, "预留")], -1)), withDirectives(createBaseVNode("input", {
							class: "input",
							"onUpdate:modelValue": _cache[8] || (_cache[8] = ($event) => form.unixSocket = $event),
							placeholder: "/tmp/redis.sock",
							disabled: ""
						}, null, 512), [[vModelText, form.unixSocket]])]),
						createBaseVNode("div", _hoisted_14, [_cache[24] || (_cache[24] = createBaseVNode("label", null, [createTextVNode("SSH 隧道"), createBaseVNode("span", { class: "tag-reserved" }, "预留")], -1)), createBaseVNode("div", _hoisted_15, [withDirectives(createBaseVNode("input", {
							class: "input",
							"onUpdate:modelValue": _cache[9] || (_cache[9] = ($event) => ssh.host = $event),
							placeholder: "跳板机 host",
							disabled: ""
						}, null, 512), [[vModelText, ssh.host]]), withDirectives(createBaseVNode("input", {
							class: "input",
							"onUpdate:modelValue": _cache[10] || (_cache[10] = ($event) => ssh.user = $event),
							placeholder: "user",
							disabled: ""
						}, null, 512), [[vModelText, ssh.user]])])])
					]),
					_: 1
				});
			};
		}
	});
	//#endregion
	//#region src/components/CompareDialog.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$2 = { class: "cmp-form" };
	var _hoisted_2$2 = { class: "field-row" };
	var _hoisted_3$2 = { class: "field" };
	var _hoisted_4$2 = { class: "field" };
	var _hoisted_5$2 = ["disabled"];
	var _hoisted_6$2 = {
		key: 0,
		class: "cmp-result"
	};
	var _hoisted_7$2 = { class: "cmp-head" };
	var _hoisted_8 = {
		key: 0,
		class: "diff-box"
	};
	var _hoisted_9 = {
		key: 1,
		class: "diff-box"
	};
	var _hoisted_10 = {
		key: 0,
		class: "diff-eq"
	};
	//#endregion
	//#region src/components/CompareDialog.vue
	var CompareDialog_default = /*#__PURE__*/ _plugin_vue_export_helper_default(/* @__PURE__ */ defineComponent({
		__name: "CompareDialog",
		emits: ["close"],
		setup(__props) {
			const keyA = /* @__PURE__ */ ref("");
			const keyB = /* @__PURE__ */ ref("");
			const busy = /* @__PURE__ */ ref(false);
			const result = /* @__PURE__ */ ref(null);
			if (store.dialog?.props?.keyA) keyA.value = store.dialog.props.keyA;
			function jsonVal(v) {
				if (v === void 0) return "";
				if (typeof v === "string") return JSON.stringify(v);
				return JSON.stringify(v);
			}
			async function run() {
				const c = getClient();
				if (!c) {
					toast("未连接", "err");
					return;
				}
				busy.value = true;
				try {
					result.value = await c.diffValues(keyA.value, keyB.value);
				} catch (e) {
					toast("对比失败: " + (e.message || e), "err");
				} finally {
					busy.value = false;
				}
			}
			return (_ctx, _cache) => {
				return openBlock(), createBlock(Modal_default, {
					title: "值对比",
					onClose: _cache[2] || (_cache[2] = ($event) => _ctx.$emit("close"))
				}, {
					default: withCtx(() => [createBaseVNode("div", _hoisted_1$2, [createBaseVNode("div", _hoisted_2$2, [createBaseVNode("div", _hoisted_3$2, [_cache[3] || (_cache[3] = createBaseVNode("label", null, "Key A", -1)), withDirectives(createBaseVNode("input", {
						class: "input",
						"onUpdate:modelValue": _cache[0] || (_cache[0] = ($event) => keyA.value = $event),
						placeholder: "左侧 key"
					}, null, 512), [[vModelText, keyA.value]])]), createBaseVNode("div", _hoisted_4$2, [_cache[4] || (_cache[4] = createBaseVNode("label", null, "Key B", -1)), withDirectives(createBaseVNode("input", {
						class: "input",
						"onUpdate:modelValue": _cache[1] || (_cache[1] = ($event) => keyB.value = $event),
						placeholder: "右侧 key"
					}, null, 512), [[vModelText, keyB.value]])])]), createBaseVNode("button", {
						class: "btn primary",
						type: "button",
						onClick: run,
						disabled: busy.value || !keyA.value || !keyB.value
					}, "开始对比", 8, _hoisted_5$2)]), result.value ? (openBlock(), createElementBlock("div", _hoisted_6$2, [createBaseVNode("div", _hoisted_7$2, toDisplayString(result.value.mode === "json" ? "JSON 结构差异" : "文本行差异"), 1), result.value.mode === "text" ? (openBlock(), createElementBlock("div", _hoisted_8, [(openBlock(true), createElementBlock(Fragment, null, renderList(result.value.lines, (l, i) => {
						return openBlock(), createElementBlock("div", {
							key: i,
							class: normalizeClass("diff-" + l.kind)
						}, toDisplayString(l.kind === "add" ? "+ " : l.kind === "del" ? "- " : "  ") + toDisplayString(l.text), 3);
					}), 128))])) : (openBlock(), createElementBlock("div", _hoisted_9, [(openBlock(true), createElementBlock(Fragment, null, renderList(result.value.changes || [], (c, i) => {
						return openBlock(), createElementBlock("div", {
							key: i,
							class: normalizeClass("diff-" + (c.type === "changed" ? "chg" : c.type === "added" ? "add" : "del"))
						}, [
							createBaseVNode("b", null, toDisplayString(c.path), 1),
							createTextVNode(" [" + toDisplayString(c.type) + "] ", 1),
							c.type === "changed" ? (openBlock(), createElementBlock(Fragment, { key: 0 }, [createTextVNode(toDisplayString(jsonVal(c.from)) + " → " + toDisplayString(jsonVal(c.to)), 1)], 64)) : c.type === "added" ? (openBlock(), createElementBlock(Fragment, { key: 1 }, [createTextVNode(" = " + toDisplayString(jsonVal(c.to)), 1)], 64)) : (openBlock(), createElementBlock(Fragment, { key: 2 }, [createTextVNode(" (已移除) " + toDisplayString(jsonVal(c.from)), 1)], 64))
						], 2);
					}), 128)), !(result.value.changes && result.value.changes.length) ? (openBlock(), createElementBlock("div", _hoisted_10, "两个值完全一致 ✓")) : createCommentVNode("", true)]))])) : createCommentVNode("", true)]),
					_: 1
				});
			};
		}
	}), [["__scopeId", "data-v-f7264279"]]);
	//#endregion
	//#region src/components/ImportExportDialog.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1$1 = { class: "ie-head" };
	var _hoisted_2$1 = { class: "fmt-tabs" };
	var _hoisted_3$1 = ["onClick"];
	var _hoisted_4$1 = { class: "meta" };
	var _hoisted_5$1 = ["disabled"];
	var _hoisted_6$1 = ["disabled"];
	var _hoisted_7$1 = {
		key: 0,
		class: "import-result"
	};
	//#endregion
	//#region src/components/ImportExportDialog.vue
	var ImportExportDialog_default = /*#__PURE__*/ _plugin_vue_export_helper_default(/* @__PURE__ */ defineComponent({
		__name: "ImportExportDialog",
		emits: ["close"],
		setup(__props) {
			const dlg = computed(() => store.dialog?.props || {});
			const mode = computed(() => dlg.value.mode === "import" ? "import" : "export");
			const scopeLabel = computed(() => {
				if (mode.value === "import") return "";
				return dlg.value.scope === "selected" ? `已选 ${store.selectedKeys.length} 个` : "当前库全部";
			});
			const formats = [
				{
					v: "json",
					label: "JSON"
				},
				{
					v: "csv",
					label: "CSV"
				},
				{
					v: "redis",
					label: "Redis 命令"
				}
			];
			const fmt = /* @__PURE__ */ ref("json");
			const output = /* @__PURE__ */ ref("");
			const input = /* @__PURE__ */ ref("");
			const busy = /* @__PURE__ */ ref(false);
			const importResult = /* @__PURE__ */ ref(null);
			async function doExport() {
				const c = getClient();
				if (!c) {
					toast("未连接", "err");
					return;
				}
				const keys = dlg.value.scope === "selected" ? [...store.selectedKeys] : store.keys.map((k) => k.name);
				if (!keys.length) {
					toast("没有可导出的 key", "err");
					return;
				}
				busy.value = true;
				try {
					output.value = await c.exportKeys(keys, fmt.value);
					recordAudit("EXPORT", `导出 ${keys.length} 个 key (${fmt.value})`, keys.join(","));
					toast("已生成导出内容", "ok");
				} catch (e) {
					toast("导出失败: " + (e.message || e), "err");
				} finally {
					busy.value = false;
				}
			}
			async function doImport() {
				const c = getClient();
				if (!c) {
					toast("未连接", "err");
					return;
				}
				busy.value = true;
				try {
					const r = await c.importKeys(input.value, fmt.value);
					importResult.value = r;
					if (r.count) {
						recordAudit("IMPORT", `导入 ${r.count} 个 key (${fmt.value})`, r.errors.length ? `失败 ${r.errors.length}` : "-");
						refreshKeys();
					} else recordAudit("IMPORT", `导入 0 个 key (${fmt.value})`, "无有效条目");
				} catch (e) {
					toast("导入失败: " + (e.message || e), "err");
				} finally {
					busy.value = false;
				}
			}
			function copyOut() {
				if (!output.value) return;
				const ta = document.createElement("textarea");
				ta.value = output.value;
				document.body.appendChild(ta);
				ta.select();
				try {
					document.execCommand("copy");
					toast("已复制到剪贴板", "ok");
				} catch {
					toast("复制失败，请手动选择", "err");
				}
				document.body.removeChild(ta);
			}
			function download() {
				if (!output.value) return;
				try {
					const blob = new Blob([output.value], { type: "text/plain;charset=utf-8" });
					const url = URL.createObjectURL(blob);
					const a = document.createElement("a");
					a.href = url;
					a.download = `redis-export.${fmt.value === "redis" ? "txt" : fmt.value}`;
					document.body.appendChild(a);
					a.click();
					document.body.removeChild(a);
					setTimeout(() => URL.revokeObjectURL(url), 1e3);
				} catch (e) {
					toast("下载失败: " + (e.message || e), "err");
				}
			}
			return (_ctx, _cache) => {
				return openBlock(), createBlock(Modal_default, {
					title: mode.value === "import" ? "导入 Key" : "导出 Key",
					onClose: _cache[2] || (_cache[2] = ($event) => _ctx.$emit("close"))
				}, {
					default: withCtx(() => [createBaseVNode("div", _hoisted_1$1, [
						createBaseVNode("div", _hoisted_2$1, [(openBlock(), createElementBlock(Fragment, null, renderList(formats, (f) => {
							return createBaseVNode("button", {
								key: f.v,
								type: "button",
								class: normalizeClass({ active: fmt.value === f.v }),
								onClick: ($event) => fmt.value = f.v
							}, toDisplayString(f.label), 11, _hoisted_3$1);
						}), 64))]),
						_cache[3] || (_cache[3] = createBaseVNode("span", { class: "spacer" }, null, -1)),
						createBaseVNode("span", _hoisted_4$1, toDisplayString(scopeLabel.value), 1)
					]), mode.value === "export" ? (openBlock(), createElementBlock(Fragment, { key: 0 }, [
						createBaseVNode("button", {
							class: "btn primary",
							type: "button",
							onClick: doExport,
							disabled: busy.value
						}, "生成导出内容", 8, _hoisted_5$1),
						withDirectives(createBaseVNode("textarea", {
							class: "ie-box",
							"onUpdate:modelValue": _cache[0] || (_cache[0] = ($event) => output.value = $event),
							readonly: "",
							spellcheck: "false"
						}, null, 512), [[vModelText, output.value]]),
						createBaseVNode("div", { class: "row-actions" }, [createBaseVNode("button", {
							class: "mini-btn",
							type: "button",
							onClick: copyOut
						}, "复制"), createBaseVNode("button", {
							class: "mini-btn",
							type: "button",
							onClick: download
						}, "下载文件")])
					], 64)) : (openBlock(), createElementBlock(Fragment, { key: 1 }, [
						withDirectives(createBaseVNode("textarea", {
							class: "ie-box",
							"onUpdate:modelValue": _cache[1] || (_cache[1] = ($event) => input.value = $event),
							placeholder: "在此粘贴 json / csv / redis 命令脚本",
							spellcheck: "false"
						}, null, 512), [[vModelText, input.value]]),
						createBaseVNode("button", {
							class: "btn primary",
							type: "button",
							onClick: doImport,
							disabled: busy.value || !input.value.trim()
						}, "开始导入", 8, _hoisted_6$1),
						importResult.value ? (openBlock(), createElementBlock("div", _hoisted_7$1, [createBaseVNode("div", { class: normalizeClass(importResult.value.errors.length ? "diff-chg" : "diff-add") }, " 成功导入 " + toDisplayString(importResult.value.count) + " 个 key ", 3), (openBlock(true), createElementBlock(Fragment, null, renderList(importResult.value.errors, (e, i) => {
							return openBlock(), createElementBlock("div", {
								key: i,
								class: "diff-del"
							}, toDisplayString(e), 1);
						}), 128))])) : createCommentVNode("", true)
					], 64))]),
					_: 1
				}, 8, ["title"]);
			};
		}
	}), [["__scopeId", "data-v-5e54f2d0"]]);
	//#endregion
	//#region src/components/AppRoot.vue?vue&type=script&setup=true&lang.ts
	var _hoisted_1 = { class: "topbar" };
	var _hoisted_2 = { class: "layout" };
	var _hoisted_3 = { class: "sidebar" };
	var _hoisted_4 = { class: "main" };
	var _hoisted_5 = { class: "tabs" };
	var _hoisted_6 = { class: "value-area" };
	var _hoisted_7 = {
		key: 1,
		class: "placeholder"
	};
	//#endregion
	//#region src/components/AppRoot.vue
	var AppRoot_default = /* @__PURE__ */ defineComponent({
		__name: "AppRoot",
		setup(__props) {
			const tab = /* @__PURE__ */ ref("browser");
			onMounted(async () => {
				ensureInit();
				if (store.activeConnId) await connectActive();
			});
			return (_ctx, _cache) => {
				return openBlock(), createElementBlock(Fragment, null, [
					createBaseVNode("div", _hoisted_1, [
						_cache[4] || (_cache[4] = createBaseVNode("div", { class: "brand" }, [createTextVNode("Redis 客户端"), createBaseVNode("span", { class: "ver" }, "P2 体验")], -1)),
						createVNode(ConnBar_default),
						createBaseVNode("div", { class: normalizeClass(["conn-state", { online: unref(store).connected }]) }, toDisplayString(unref(store).connected ? "已连接" : "未连接"), 3)
					]),
					createBaseVNode("div", _hoisted_2, [createBaseVNode("aside", _hoisted_3, [createVNode(DbSidebar_default)]), createBaseVNode("main", _hoisted_4, [
						createBaseVNode("div", _hoisted_5, [
							createBaseVNode("button", {
								class: normalizeClass(["tab", { active: tab.value === "browser" }]),
								type: "button",
								onClick: _cache[0] || (_cache[0] = ($event) => tab.value = "browser")
							}, " 数据浏览 ", 2),
							createBaseVNode("button", {
								class: normalizeClass(["tab", { active: tab.value === "terminal" }]),
								type: "button",
								onClick: _cache[1] || (_cache[1] = ($event) => tab.value = "terminal")
							}, " 命令行 ", 2),
							createBaseVNode("button", {
								class: normalizeClass(["tab", { active: tab.value === "monitor" }]),
								type: "button",
								onClick: _cache[2] || (_cache[2] = ($event) => tab.value = "monitor")
							}, " 监控 ", 2),
							createBaseVNode("button", {
								class: normalizeClass(["tab", { active: tab.value === "ops" }]),
								type: "button",
								onClick: _cache[3] || (_cache[3] = ($event) => tab.value = "ops")
							}, " 运维 ", 2)
						]),
						createBaseVNode("section", { class: normalizeClass(["pane", { hidden: tab.value !== "browser" }]) }, [createVNode(KeyInfoBar_default), createBaseVNode("div", _hoisted_6, [unref(store).selectedKey ? (openBlock(), createBlock(ValueEditor_default, { key: 0 })) : (openBlock(), createElementBlock("div", _hoisted_7, "在左侧选择或搜索一个 key 查看其内容"))])], 2),
						createBaseVNode("section", {
							class: normalizeClass(["pane", { hidden: tab.value !== "terminal" }]),
							id: "pane-terminal"
						}, [createVNode(Terminal_default)], 2),
						createBaseVNode("section", { class: normalizeClass(["pane", { hidden: tab.value !== "monitor" }]) }, [createVNode(MonitorPanel_default)], 2),
						createBaseVNode("section", { class: normalizeClass(["pane", { hidden: tab.value !== "ops" }]) }, [createVNode(OpsView_default)], 2)
					])]),
					unref(store).dialog?.type === "conn" ? (openBlock(), createBlock(ConnDialog_default, {
						key: 0,
						onClose: unref(closeDialog)
					}, null, 8, ["onClose"])) : unref(store).dialog?.type === "compare" ? (openBlock(), createBlock(CompareDialog_default, {
						key: 1,
						onClose: unref(closeDialog)
					}, null, 8, ["onClose"])) : unref(store).dialog?.type === "importExport" ? (openBlock(), createBlock(ImportExportDialog_default, {
						key: 2,
						onClose: unref(closeDialog)
					}, null, 8, ["onClose"])) : createCommentVNode("", true),
					unref(store).toast ? (openBlock(), createElementBlock("div", {
						key: 3,
						class: normalizeClass(["toast", unref(store).toast.type])
					}, toDisplayString(unref(store).toast.msg), 3)) : createCommentVNode("", true)
				], 64);
			};
		}
	});
	//#endregion
	//#region src/main.ts
	window.addEventListener("message", (e) => {
		bridge.handleMessage(e.data);
		const d = e.data;
		if (d && d.v === 1 && d.kind === "theme") document.documentElement.setAttribute("data-theme", d.dark ? "dark" : "light");
	});
	bridge.ready();
	createApp(AppRoot_default).mount("#app");
	//#endregion
})();
