//! Wipe-safe per-opcode `Protocol.Handlers` hook.
//!
//! `ClassiCube`'s `Protocol` component wipes `Protocol.Handlers` on every
//! `Game_Reset` (disconnect), restoring the stock handlers. [`ProtocolHook`]
//! re-installs the hook for its opcode after each wipe. Re-install is
//! order-independent and self-healing: it reads the live slot and, unless our
//! trampoline is already on top, chains onto whatever the wipe restored -- so
//! it doesn't matter in which order plugins re-install relative to their
//! original install order. Two guards keep this safe: "already on top" prevents
//! chaining onto ourselves at the head, and an `IN_CHAIN` flag makes a fresh
//! `install()` re-arm in place (rather than re-push) when our trampoline is
//! still buried in the chain after a reload -- re-pushing while buried would
//! close a cycle.
//!
//! Each opcode gets its own independent chain: one `Protocol.Handlers` slot,
//! one const-generic trampoline address (selected from a `HANDLER_COUNT`-entry
//! `TRAMPOLINES` table at install time), and one row in per-opcode
//! `HANDLER_COUNT`-wide thread-local state arrays. Hooks for different opcodes
//! never interfere.
//!
//! Each plugin binary that links classicube-helpers gets its own copy of
//! the internal `trampoline` functions (rlib static linking), so the hook
//! addresses are unique per plugin `.so` even though all share this source.
//!
//! # Main-thread requirement
//!
//! All public methods must be called on `ClassiCube`'s main game-loop thread.
//! The internal thread-local storage (`OLD`, `CALLBACK`) is keyed to that
//! thread; calling from a worker thread would silently create a separate,
//! useless slot.
//!
//! # Unloading and module lifetime
//!
//! Dropping a [`ProtocolHook`] at runtime is always safe, in any order:
//! if our trampoline is the chain head it is spliced out (the saved handler is
//! restored into the slot); if a foreign plugin stacked above us, our
//! trampoline stays in the chain as a transparent forwarder (callback cleared,
//! forwards straight to its saved handler).
//!
//! *Reloading* (drop then re-create the handle, e.g. a plugin manager calling a
//! component's Free then Init) is also handled: a fresh
//! [`install`](ProtocolHook::install) re-arms in place when our trampoline is
//! still buried, instead of re-pushing into a cycle. One accepted edge: if a
//! buried plugin is dropped, then a disconnect wipe happens while it is orphaned
//! (so its `Reset`/[`reinstall`](ProtocolHook::reinstall) never fires), then it
//! is reloaded, the hook stays silently inactive until the next disconnect
//! re-fires [`reinstall`](ProtocolHook::reinstall), which heals it. This is a
//! self-healing lost hook, never a crash.
//!
//! What is **not** safe is unmapping the *code* of a plugin whose trampoline is
//! still live in the chain but is not the head -- i.e. `dlclose`-ing a plugin
//! `.so` that another plugin has chained above. The link above it cached its
//! trampoline address, and because each plugin's saved handler lives in a
//! private per-`.so` thread-local that no other link can read or rewrite, a
//! buried link can never remove itself. Unmapping it leaves a dangling function
//! pointer that crashes on the next packet of that opcode. This crate cannot
//! guard against it -- the chain is not introspectable across plugin boundaries.
//!
//! Stock `ClassiCube` never hits this: it `dlopen`s plugins once at startup and
//! never unloads them. A plugin that loads and unloads *other* plugins at
//! runtime must uphold the invariant itself. Safe options, best first:
//!
//!   * Keep managed plugins resident (do not `dlclose`) -- just drop their hook
//!     and run their teardown. The inert forwarders left behind are harmless,
//!     and the next `Protocol` wipe drops them from the chain entirely. This is
//!     order-independent and mirrors `ClassiCube`'s own load-once model.
//!   * If memory must be reclaimed, only `dlclose` a plugin that is the current
//!     chain head (strict LIFO unload order), or do it in the post-disconnect
//!     window after the wipe has reset the slot and before the unloaded plugin
//!     would re-install.

#[cfg(test)]
mod tests;

use std::{cell::Cell, ptr, slice};

use classicube_sys::{Net_Handler, OPCODE__OPCODE_COUNT, Protocol, Server};

/// Per-opcode state width. `ClassiCube`'s C handler table is `[256]`, but only
/// `OPCODE_COUNT` (60 in 1.3.8) opcodes are ever defined; we size our state to
/// the next multiple of 16 above that so the 16-wide `trampoline_table!` macro
/// fills it exactly. Bump by 16 if `ClassiCube` defines more opcodes (the assert
/// below turns that into a compile error rather than an out-of-bounds index).
const HANDLER_COUNT: usize = 64;

const _: () = assert!(
    OPCODE__OPCODE_COUNT as usize <= HANDLER_COUNT,
    "ClassiCube now defines more opcodes than HANDLER_COUNT; bump it to the next multiple of 16",
);

type Callback = Cell<Option<Box<dyn FnMut(&[u8]) -> bool>>>;

thread_local! {
    static OLD: [Cell<Net_Handler>; HANDLER_COUNT] =
        const { [const { Cell::new(None) }; HANDLER_COUNT] };
    static CALLBACK: [Callback; HANDLER_COUNT] =
        const { [const { Cell::new(None) }; HANDLER_COUNT] };
    /// Per-opcode in-chain belief. `true` when our trampoline is believed
    /// reachable from the live chain (head or buried) for that opcode. Gates
    /// `install()` so a reload-while-buried re-arms in place instead of
    /// re-pushing (re-pushing would duplicate our reference and form a cycle).
    /// Set false only when we splice ourselves out as head, or reset by
    /// `reinstall()` before its forced post-wipe re-push.
    static IN_CHAIN: [Cell<bool>; HANDLER_COUNT] =
        const { [const { Cell::new(false) }; HANDLER_COUNT] };
    /// Re-entrancy backstop guarding the forward-to-`OLD` step only, per opcode.
    /// Breaks any residual chain cycle into a dropped packet instead of a stack
    /// overflow.
    static FORWARDING: [Cell<bool>; HANDLER_COUNT] =
        const { [const { Cell::new(false) }; HANDLER_COUNT] };
}

// Per-OP address in each plugin .so is unique (rlib static linking), making
// it safe to use as a hook identity via `ptr::fn_addr_eq`.
extern "C" fn trampoline<const OP: u8>(data: *mut u8) {
    // SAFETY: ClassiCube dispatches handler(readCur + 1) with readCur +=
    // Sizes[opcode] after; payload is Protocol.Sizes[OP] - 1 bytes at data.
    let payload_len = unsafe { Protocol.Sizes[OP as usize] as usize }.saturating_sub(1);
    let bytes = unsafe { slice::from_raw_parts(data, payload_len) };
    // take/call/put-back: if the callback re-enters the trampoline for this
    // opcode (e.g. via chat_print -> ChatEvents -> Protocol roundtrip), the slot
    // holds None during the call, so re-entry naturally skips the callback --
    // identical suppression semantics without a borrow flag.
    let should_suppress = CALLBACK.with(|a| {
        let slot = &a[OP as usize];
        let mut cb = slot.take();
        let suppress = cb.as_mut().is_some_and(|f| f(bytes));
        slot.set(cb);
        suppress
    });
    if !should_suppress {
        // Re-entrancy backstop: guard only the forward. A residual chain cycle
        // (e.g. A.OLD=B, B.OLD=A) would otherwise recurse here forever; instead
        // we drop the packet.
        if !FORWARDING.with(|a| a[OP as usize].get()) {
            FORWARDING.with(|a| a[OP as usize].set(true));
            OLD.with(|a| {
                if let Some(f) = a[OP as usize].get() {
                    // SAFETY: `f` is the previously-installed Net_Handler, which
                    // ClassiCube guarantees is valid while the Protocol layer is live.
                    unsafe { f(data) }
                }
            });
            FORWARDING.with(|a| a[OP as usize].set(false));
        }
    }
}

macro_rules! trampoline_table {
    ($($hi:literal),* $(,)?) => {
        [$(
            Some(trampoline::<{ $hi * 16 + 0  }> as unsafe extern "C" fn(*mut u8)),
            Some(trampoline::<{ $hi * 16 + 1  }> as unsafe extern "C" fn(*mut u8)),
            Some(trampoline::<{ $hi * 16 + 2  }> as unsafe extern "C" fn(*mut u8)),
            Some(trampoline::<{ $hi * 16 + 3  }> as unsafe extern "C" fn(*mut u8)),
            Some(trampoline::<{ $hi * 16 + 4  }> as unsafe extern "C" fn(*mut u8)),
            Some(trampoline::<{ $hi * 16 + 5  }> as unsafe extern "C" fn(*mut u8)),
            Some(trampoline::<{ $hi * 16 + 6  }> as unsafe extern "C" fn(*mut u8)),
            Some(trampoline::<{ $hi * 16 + 7  }> as unsafe extern "C" fn(*mut u8)),
            Some(trampoline::<{ $hi * 16 + 8  }> as unsafe extern "C" fn(*mut u8)),
            Some(trampoline::<{ $hi * 16 + 9  }> as unsafe extern "C" fn(*mut u8)),
            Some(trampoline::<{ $hi * 16 + 10 }> as unsafe extern "C" fn(*mut u8)),
            Some(trampoline::<{ $hi * 16 + 11 }> as unsafe extern "C" fn(*mut u8)),
            Some(trampoline::<{ $hi * 16 + 12 }> as unsafe extern "C" fn(*mut u8)),
            Some(trampoline::<{ $hi * 16 + 13 }> as unsafe extern "C" fn(*mut u8)),
            Some(trampoline::<{ $hi * 16 + 14 }> as unsafe extern "C" fn(*mut u8)),
            Some(trampoline::<{ $hi * 16 + 15 }> as unsafe extern "C" fn(*mut u8)),
        )*]
    };
}

static TRAMPOLINES: [Net_Handler; HANDLER_COUNT] = trampoline_table!(0, 1, 2, 3);

#[must_use]
fn is_our_handler(opcode: u8, a: Net_Handler) -> bool {
    handlers_eq(a, TRAMPOLINES[opcode as usize])
}

#[must_use]
fn handlers_eq(a: Net_Handler, b: Net_Handler) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => ptr::fn_addr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

/// Pure chain-install decision, factored out so it can be unit-tested without
/// the `Protocol` global or the Windows import library. Given the handler
/// `current` occupying the slot and `ours` (this plugin's trampoline identity),
/// returns:
///   * `None` -- `current` is already us; nothing to do (no-op).
///   * `Some(current)` -- we are not on top; the caller saves this value as the
///     new `OLD` and writes `ours` into the slot.
///
/// It never inspects a cached previous `OLD`, so a stale `OLD` left over from
/// before a `Protocol` wipe cannot make it wrongly refuse to re-install. Its
/// only guard is "already on top", which stops pushing self twice when we are
/// the head. The complementary "buried" case (our trampoline reachable below a
/// foreign head, which this pure step cannot see) is gated separately by the
/// `IN_CHAIN` flag in [`ProtocolHook::install`].
#[must_use]
fn install_step<T: Copy + PartialEq>(current: T, ours: T) -> Option<T> {
    if current == ours { None } else { Some(current) }
}

/// `Net_Handler` wrapped so its `PartialEq` uses `handlers_eq`
/// (`ptr::fn_addr_eq`) rather than the derived `Option`/`fn`-pointer `==`,
/// matching the identity semantics the live hook chain relies on. Only
/// `PartialEq` is implemented (not `Eq`): `ptr::fn_addr_eq` is not a guaranteed
/// equivalence under LLVM function-merging, and `install_step` needs no more.
#[derive(Clone, Copy)]
struct HandlerId(Net_Handler);

impl PartialEq for HandlerId {
    fn eq(&self, other: &Self) -> bool {
        handlers_eq(self.0, other.0)
    }
}

/// Returns `true` when running in singleplayer, where `ClassiCube`'s `Protocol`
/// component is inert (its `OnInit`/`OnReset` early-return) -- there is no
/// handler table to hook or re-hook.
#[must_use]
fn is_single_player() -> bool {
    // SAFETY: Server is valid by the time any plugin component is loaded.
    unsafe { Server.IsSinglePlayer != 0 }
}

/// Push our trampoline for `opcode` onto the chain head, saving whatever
/// currently occupies the slot as `OLD[opcode]`. The push primitive shared by
/// `install()` (when not already in the chain) and `reinstall()` (always,
/// post-wipe). Marks `IN_CHAIN[opcode]` on both outcomes. We deliberately never
/// consult the cached `OLD` to decide whether to re-push: a stale `OLD` from
/// before a wipe must not veto it (that was the multi-plugin reorder bug).
fn install_inner(opcode: u8) {
    // SAFETY: reading from the handler table; Protocol is valid once
    // ClassiCube's component chain has initialised (before any Net handler
    // can be invoked).
    let current: Net_Handler = unsafe { Protocol.Handlers[opcode as usize] };
    let Some(new_old) = install_step(HandlerId(current), HandlerId(TRAMPOLINES[opcode as usize]))
    else {
        IN_CHAIN.with(|a| a[opcode as usize].set(true)); // already on top -- still in chain
        return;
    };
    // SAFETY: writing the same table under the same lifetime constraints.
    unsafe {
        Protocol.Handlers[opcode as usize] = TRAMPOLINES[opcode as usize];
    }
    OLD.with(|a| a[opcode as usize].set(new_old.0));
    IN_CHAIN.with(|a| a[opcode as usize].set(true));
}

fn uninstall_inner(opcode: u8) {
    let current: Net_Handler = unsafe { Protocol.Handlers[opcode as usize] };
    if is_our_handler(opcode, current) {
        // We are the head: splice out by restoring OLD. We are no longer
        // referenced anywhere, so clear IN_CHAIN.
        let prior = OLD.with(|a| a[opcode as usize].take());
        unsafe {
            Protocol.Handlers[opcode as usize] = prior;
        }
        IN_CHAIN.with(|a| a[opcode as usize].set(false));
    }
    // Else: a foreign plugin is on top. Leave the slot and OLD alone. The
    // trampoline is still reachable via their chain; with CALLBACK cleared by
    // Drop it will just forward to OLD without calling plugin-specific code.
    // IN_CHAIN stays true: the link above still references us, so a later
    // reload must re-arm in place rather than re-push (which would cycle).
}

/// RAII handle for a `Protocol.Handlers[opcode]` hook.
///
/// The callback receives the raw payload bytes -- everything after the opcode
/// byte itself, of length `Protocol.Sizes[opcode] - 1` -- and returns `true` to
/// suppress (drop) the packet or `false` to pass it on.
///
/// Call [`reinstall`](Self::reinstall) from your plugin component's `Reset`
/// callback after each reconnect; `ClassiCube`'s `Protocol` component wipes the
/// handler table on every disconnect.
///
/// Dropping uninstalls (if on top) and clears the callback. Any foreign plugin
/// that stacked above will keep calling through the trampoline, but it becomes
/// a transparent forwarder with no plugin-specific processing. Dropping is
/// always safe; *unmapping* the plugin `.so` while it is a buried chain link is
/// not -- see the module-level "Unloading and module lifetime" section.
pub struct ProtocolHook {
    opcode: u8,
}

impl ProtocolHook {
    /// Install the hook for `opcode` and return a handle.
    ///
    /// Returns `None` in singleplayer (no `Protocol` layer; nothing to hook).
    ///
    /// If this plugin's trampoline is already live in the chain for this opcode
    /// -- e.g. it was dropped while buried under another plugin and is now being
    /// re-created without an intervening `Protocol` wipe (a reload) -- the new
    /// callback is armed *in place* and the trampoline is **not** re-pushed.
    /// Re-pushing while buried would make our `OLD` point at a chain that leads
    /// back to us and form a cycle.
    ///
    /// # Panics
    ///
    /// Panics if a hook for this opcode is already installed. There is only one
    /// `Protocol.Handlers` slot per opcode per plugin binary; installing twice
    /// would silently overwrite the callback and corrupt the first handle's
    /// `Drop`. Call `drop` (or let the handle go out of scope) before
    /// installing again.
    #[must_use]
    pub fn install<F>(opcode: u8, callback: F) -> Option<Self>
    where
        F: FnMut(&[u8]) -> bool + 'static,
    {
        if is_single_player() {
            return None;
        }
        assert!(
            (opcode as usize) < HANDLER_COUNT,
            "ProtocolHook opcode {opcode} exceeds supported range 0..{HANDLER_COUNT}",
        );
        assert!(
            CALLBACK.with(|a| {
                let slot = &a[opcode as usize];
                let prev = slot.take();
                let empty = prev.is_none();
                slot.set(prev);
                empty
            }),
            "ProtocolHook already installed for opcode {opcode}; drop the existing handle before \
             calling install again",
        );
        CALLBACK.with(|a| a[opcode as usize].set(Some(Box::new(callback))));
        // If our trampoline is believed already in the chain (buried under a
        // foreign plugin after a reload-while-buried), arming CALLBACK above is
        // enough -- the buried forwarder calls us again. Pushing here would
        // duplicate our reference and form a cycle.
        if !IN_CHAIN.with(|a| a[opcode as usize].get()) {
            install_inner(opcode);
        }
        Some(Self { opcode })
    }

    /// Re-hook after `ClassiCube` wipes `Protocol.Handlers` on disconnect.
    ///
    /// Call from your plugin component's `Reset` callback. Idempotent and
    /// order-independent: if our trampoline is already on top it is a no-op;
    /// otherwise it chains onto whatever the wipe restored, regardless of the
    /// order in which other plugins re-install. No-op in singleplayer, where
    /// there is no `Protocol` layer to hook.
    ///
    /// Unlike [`install`](Self::install), this *always* re-pushes. It is only
    /// ever called post-wipe, when the slot holds the stock handler and our
    /// trampoline is not reachable, so a stale `IN_CHAIN` left over from before
    /// the wipe must not veto the re-push (that would re-introduce the reorder
    /// bug). Force-pushing against the stock head cannot cycle.
    pub fn reinstall(&self) {
        if is_single_player() {
            return;
        }
        // Clear the stale in-chain belief from before the wipe, then force-push.
        IN_CHAIN.with(|a| a[self.opcode as usize].set(false));
        install_inner(self.opcode);
    }
}

impl Drop for ProtocolHook {
    fn drop(&mut self) {
        uninstall_inner(self.opcode);
        // Clear CALLBACK unconditionally: even when uninstall_inner no-ops
        // (foreign plugin on top), we must clear the callback so a later
        // `install` call does not spuriously hit the double-install assert.
        CALLBACK.with(|a| a[self.opcode as usize].set(None));
    }
}
