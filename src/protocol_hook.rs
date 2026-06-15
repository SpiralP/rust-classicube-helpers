#[cfg(test)]
mod tests;

use std::{
    cell::{Cell, RefCell},
    ptr, slice,
};

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

type Callback = RefCell<Option<Box<dyn FnMut(&[u8]) -> bool>>>;

thread_local! {
    static OLD: [Cell<Net_Handler>; HANDLER_COUNT] =
        const { [const { Cell::new(None) }; HANDLER_COUNT] };
    static CALLBACK: [Callback; HANDLER_COUNT] =
        const { [const { RefCell::new(None) }; HANDLER_COUNT] };
    /// Per-opcode in-chain belief. `true` when our trampoline is believed
    /// reachable from the live chain (head or buried) for that opcode. Gates
    /// `install()` so a reload-while-buried re-arms in place instead of
    /// re-pushing (re-pushing would duplicate our reference and form a cycle).
    /// Set false only when we splice ourselves out as head, or reset by
    /// `reinstall()` before its forced post-wipe re-push.
    static IN_CHAIN: [Cell<bool>; HANDLER_COUNT] =
        const { [const { Cell::new(false) }; HANDLER_COUNT] };
}

// Per-OP address in each plugin .so is unique (rlib static linking), making
// it safe to use as a hook identity via `ptr::fn_addr_eq`.
extern "C" fn trampoline<const OP: u8>(data: *mut u8) {
    // SAFETY: ClassiCube dispatches handler(readCur + 1) with readCur +=
    // Sizes[opcode] after; payload is Protocol.Sizes[OP] - 1 bytes at data.
    let payload_len = unsafe { Protocol.Sizes[OP as usize] as usize }.saturating_sub(1);
    let bytes = unsafe { slice::from_raw_parts(data, payload_len) };
    // borrow_mut is held across the callback: if it re-enters the trampoline for
    // this opcode (e.g. via chat_print -> ChatEvents -> Protocol roundtrip), the
    // re-entrant borrow_mut panics with BorrowMutError rather than silently
    // running again. The borrow is released before the forward below, so a
    // genuine nested dispatch during the forward still runs the callback.
    let should_suppress = CALLBACK.with(|a| {
        let mut slot = a[OP as usize].borrow_mut();
        slot.as_mut().is_some_and(|f| f(bytes))
    });
    if !should_suppress {
        OLD.with(|a| {
            if let Some(f) = a[OP as usize].get() {
                // SAFETY: `f` is the previously-installed Net_Handler, which
                // ClassiCube guarantees is valid while the Protocol layer is live.
                unsafe { f(data) }
            }
        });
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
///
/// The only guard is "already on top" (`is_our_handler`), which stops pushing
/// self twice when we are the head. The complementary "buried" case (our
/// trampoline reachable below a foreign head, which the slot read cannot see)
/// is gated separately by the `IN_CHAIN` flag in [`ProtocolHook::install`].
fn install_inner(opcode: u8) {
    // SAFETY: reading from the handler table; Protocol is valid once
    // ClassiCube's component chain has initialised (before any Net handler
    // can be invoked).
    let current: Net_Handler = unsafe { Protocol.Handlers[opcode as usize] };
    if is_our_handler(opcode, current) {
        IN_CHAIN.with(|a| a[opcode as usize].set(true)); // already on top -- still in chain
        return;
    }
    // SAFETY: writing the same table under the same lifetime constraints.
    unsafe {
        Protocol.Handlers[opcode as usize] = TRAMPOLINES[opcode as usize];
    }
    OLD.with(|a| a[opcode as usize].set(current));
    IN_CHAIN.with(|a| a[opcode as usize].set(true));
}

fn uninstall_inner(opcode: u8) {
    let current: Net_Handler = unsafe { Protocol.Handlers[opcode as usize] };
    if is_our_handler(opcode, current) {
        // We are the head: splice out by restoring OLD. We are no longer
        // referenced anywhere, so clear IN_CHAIN.
        //
        // Use try_with instead of with: if this Drop is reached during
        // thread-local teardown at process exit (e.g. the consumer stored the
        // hook in a thread_local!), OLD/IN_CHAIN may already be destroyed and
        // with would panic with AccessError. The Protocol layer is going away
        // at that point, so skipping the splice is safe.
        let Ok(prior) = OLD.try_with(|a| a[opcode as usize].take()) else {
            return;
        };
        unsafe {
            Protocol.Handlers[opcode as usize] = prior;
        }
        let _ = IN_CHAIN.try_with(|a| a[opcode as usize].set(false));
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
/// The callback must not re-enter its own opcode: the callback slot is held with
/// a `RefCell` borrow for the duration of the call, so synchronously
/// re-dispatching this opcode from inside the callback (e.g. via a
/// `chat_print` -> `ChatEvents` -> `Protocol` roundtrip) panics with
/// `BorrowMutError` rather than silently running again.
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
            CALLBACK.with(|a| a[opcode as usize].borrow().is_none()),
            "ProtocolHook already installed for opcode {opcode}; drop the existing handle before \
             calling install again",
        );
        CALLBACK.with(|a| *a[opcode as usize].borrow_mut() = Some(Box::new(callback)));
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
        // try_with: CALLBACK may already be destroyed during TLS teardown
        // (see uninstall_inner); nothing to clear if so.
        let _ = CALLBACK.try_with(|a| *a[self.opcode as usize].borrow_mut() = None);
    }
}
