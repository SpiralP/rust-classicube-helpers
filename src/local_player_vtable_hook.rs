#[cfg(test)]
mod tests;

use std::{
    cell::{Cell, RefCell},
    ptr,
};

use classicube_sys::{
    ENTITIES_SELF_ID, Entities, Entity, EntityVTABLE, LocationUpdate, PackedCol, cc_bool,
};

// ---------------------------------------------------------------------------
// Per-field codegen
//
// One invocation over the full field list emits, per field: a public
// fn-pointer type alias (`*Fn`), a public callback type alias (`*Cb`), one
// thread-local callback slot (`CALLBACK_*`), and a trampoline. It also emits,
// once: the `LocalPlayerVTableHooks` struct plus the `bake_all` / `arm_all` /
// `clear_callbacks` helpers that drive the per-field items, so adding or
// removing a hooked field is a single-line change to the table below.
//
// Void and value-returning fields share one arm: every field declares a return
// type, and void fields pass `-> ()`. The original (next-in-chain) function
// pointer for each field is read live from the prior VTABLE saved in
// `ORIGINAL_VTABLE`, so there is no per-field `ORIGINAL_*` cell.
// ---------------------------------------------------------------------------
macro_rules! local_player_vtable_hooks {
    (
        $(
            $snake:ident, $vtable_field:ident, $fn_ty:ident,
            ($($arg_name:ident : $arg_ty:ty),*) -> $ret:ty,
            $cb_sig:ty
        );* $(;)?
    ) => {
        paste::paste! {
            $(
                pub type $fn_ty =
                    unsafe extern "C" fn(e: *mut Entity $(, $arg_name: $arg_ty)*) -> $ret;

                pub type [<$vtable_field Cb>] = $cb_sig;

                thread_local! {
                    static [<CALLBACK_ $vtable_field:upper>]:
                        RefCell<Option<Box<[<$vtable_field Cb>]>>> = const { RefCell::new(None) };
                }

                unsafe extern "C" fn [<$snake _tramp>](
                    e: *mut Entity $(, $arg_name: $arg_ty)*
                ) -> $ret {
                    // The prior VTABLE saved at push time is the next link in the
                    // chain; its same-named field is the original fn for this slot.
                    // ORIGINAL_VTABLE is Some whenever a trampoline can fire: push()
                    // sets it before repointing the slot at our box, and it is
                    // cleared only on head-uninstall, after the slot no longer
                    // points at us.
                    let vt = ORIGINAL_VTABLE
                        .with(Cell::get)
                        .expect("ORIGINAL_VTABLE is set whenever a trampoline fires");
                    // SAFETY: vt is the live prior VTABLE in our chain (the stock
                    // static, or a foreign box kept alive while we remain
                    // reachable). VTABLE structs are swapped wholesale, never
                    // mutated in place, so the field value is stable. The field is
                    // Some: every link copies a fully-populated live VTABLE before
                    // patching.
                    let original = unsafe { (*vt).$vtable_field }.expect(concat!(
                        stringify!($vtable_field),
                        " original fn is populated in the prior VTABLE"
                    ));
                    [<CALLBACK_ $vtable_field:upper>].with(|c| {
                        // borrow_mut is held across the callback: a re-entrant
                        // dispatch of this field from inside the callback panics
                        // with BorrowMutError rather than silently running again.
                        let mut slot = c.borrow_mut();
                        match slot.as_mut() {
                            Some(f) => f(e $(, $arg_name)*, original),
                            None => {
                                // Buried-but-dropped: transparent forward.
                                // SAFETY: original is a valid function pointer from
                                // the chain; the entity pointer is live while the
                                // VTABLE dispatch is in progress.
                                unsafe { original(e $(, $arg_name)*) }
                            }
                        }
                    })
                }
            )*

            /// Optional **around** callback per `EntityVTABLE` field.
            ///
            /// Each field's callback receives the entity pointer, the field's
            /// arguments, and the original function pointer (the next link in the
            /// chain). Calling the original is `unsafe`. Construct with
            /// `..Default::default()` to leave unneeded fields as `None`.
            ///
            /// The hooked field set is baked into the replacement VTABLE box on the
            /// first [`LocalPlayerVTableHook::new`] call and is fixed for the
            /// lifetime of that box. Supply the same fields on every reload (drop
            /// then `new` again): fields absent on reload cannot be baked into the
            /// existing box while it is buried.
            #[derive(Default)]
            pub struct LocalPlayerVTableHooks {
                $( pub $snake: Option<Box<[<$vtable_field Cb>]>>, )*
            }

            /// Patch the requested fields' trampolines into the replacement VTABLE
            /// copy `v`. Reads `hooks.*.is_some()`, so it must run while `hooks` is
            /// still intact (before [`arm_all`] consumes it).
            fn bake_all(v: &mut EntityVTABLE, hooks: &LocalPlayerVTableHooks) {
                $( if hooks.$snake.is_some() { v.$vtable_field = Some([<$snake _tramp>]); } )*
            }

            /// Move each supplied callback into its thread-local slot. Consumes
            /// `hooks`; call after [`bake_all`].
            fn arm_all(hooks: LocalPlayerVTableHooks) {
                $(
                    if let Some(cb) = hooks.$snake {
                        [<CALLBACK_ $vtable_field:upper>].with(|c| *c.borrow_mut() = Some(cb));
                    }
                )*
            }

            /// Clear every callback slot. Used on uninstall (head or buried) so a
            /// later `new()` does not trip the double-install assert and any
            /// surviving trampoline degrades to a transparent forwarder. Uses
            /// `try_with` for safety during TLS teardown.
            fn clear_callbacks() {
                $( let _ = [<CALLBACK_ $vtable_field:upper>].try_with(|c| *c.borrow_mut() = None); )*
            }
        }
    };
}

local_player_vtable_hooks! {
    tick, Tick, TickFn,
        (delta: f32) -> (),
        dyn FnMut(*mut Entity, f32, TickFn);
    despawn, Despawn, DespawnFn,
        () -> (),
        dyn FnMut(*mut Entity, DespawnFn);
    set_location, SetLocation, SetLocationFn,
        (update: *mut LocationUpdate) -> (),
        dyn FnMut(*mut Entity, *mut LocationUpdate, SetLocationFn);
    get_col, GetCol, GetColFn,
        () -> PackedCol,
        dyn FnMut(*mut Entity, GetColFn) -> PackedCol;
    render_model, RenderModel, RenderModelFn,
        (delta: f32, t: f32) -> (),
        dyn FnMut(*mut Entity, f32, f32, RenderModelFn);
    should_render_name, ShouldRenderName, ShouldRenderNameFn,
        () -> cc_bool,
        dyn FnMut(*mut Entity, ShouldRenderNameFn) -> cc_bool;
}

// ---------------------------------------------------------------------------
// Shared chain state (one live instance per plugin binary)
// ---------------------------------------------------------------------------

thread_local! {
    /// Whether a [`LocalPlayerVTableHook`] handle is currently live. Guards
    /// against double-install.
    static INSTALLED: Cell<bool> = const { Cell::new(false) };
    /// Whether our box is believed reachable in the live chain (head or buried).
    /// Stays true after a buried Drop so a reload re-arms in place instead of
    /// re-pushing (which would form a cycle).
    static IN_CHAIN: Cell<bool> = const { Cell::new(false) };
    /// The VTABLE pointer that occupied the entity slot at push time -- the next
    /// link in our chain. Restored on head-uninstall, and read live by every
    /// trampoline as the source of its original (next-in-chain) function pointer
    /// (`(*ORIGINAL_VTABLE).<field>`).
    static ORIGINAL_VTABLE: Cell<Option<*const EntityVTABLE>> = const { Cell::new(None) };
    /// Our box's heap address. "Are we on top?" = `(*lp).VTABLE == OUR_VTABLE`.
    static OUR_VTABLE: Cell<Option<*const EntityVTABLE>> = const { Cell::new(None) };
    /// Keeps the replacement box alive. Must not be dropped while buried: a
    /// hook above us saved our box pointer as its ORIGINAL_VTABLE; freeing it
    /// would dangle that pointer and SIGSEGV on the next VTABLE dispatch.
    static VTABLE_BOX: Cell<Option<Box<EntityVTABLE>>> = const { Cell::new(None) };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read `Entities.List[ENTITIES_SELF_ID]`. Returns null if the slot is empty
/// (before the first map load).
fn lp_entity() -> *mut Entity {
    // SAFETY: ENTITIES_SELF_ID is a valid index into Entities.List[]; the element
    // is a Copy pointer read by value, so no reference to the static mut is formed.
    unsafe { Entities.List[ENTITIES_SELF_ID as usize] }
}

/// Pure chain-install decision. Returns `true` when we need to push (i.e. our
/// box is not already the chain head). Testable without FFI.
#[must_use]
fn should_push(current: *const EntityVTABLE, ours: Option<*const EntityVTABLE>) -> bool {
    ours != Some(current)
}

/// Push our replacement VTABLE box onto the chain head. Must be called while
/// `hooks` is still intact (before callbacks are moved into thread-locals).
fn push(lp: *mut Entity, hooks: &LocalPlayerVTableHooks) {
    // SAFETY: lp is non-null (checked in new() before calling push).
    let current: *const EntityVTABLE = unsafe { (*lp).VTABLE };

    if !should_push(current, OUR_VTABLE.with(Cell::get)) {
        // Already on top -- just mark in-chain and return.
        IN_CHAIN.with(|c| c.set(true));
        return;
    }

    // Copy all six fields (invariant 1: carry forward any foreign field overrides
    // already present in the live VTABLE at push time), then patch the subset the
    // consumer requested.
    // SAFETY: current is the live entity VTABLE, valid while the entity is live.
    let mut v: EntityVTABLE = unsafe { *current };
    bake_all(&mut v, hooks);

    let boxed = Box::new(v);
    // Obtain the heap address via a shared reference before moving the Box into
    // the Cell. The heap allocation does not move when the Box handle moves.
    let ptr: *const EntityVTABLE = ptr::from_ref(boxed.as_ref());

    // ORIGINAL_VTABLE must be set before the slot is repointed: the trampolines
    // read it the instant the engine dispatches through our box.
    ORIGINAL_VTABLE.with(|c| c.set(Some(current)));
    // SAFETY: writing the per-entity VTABLE pointer; lp is valid.
    unsafe { (*lp).VTABLE = ptr };
    OUR_VTABLE.with(|c| c.set(Some(ptr)));
    VTABLE_BOX.with(|c| c.set(Some(boxed)));
    IN_CHAIN.with(|c| c.set(true));
}

fn uninstall_inner() {
    // Clear all callbacks unconditionally, even when buried: prevents the
    // double-install assert from firing on a later new(), and degrades any
    // surviving trampoline to a transparent forwarder.
    //
    // try_with throughout: Drop may fire during TLS teardown (e.g. if the
    // consumer stores the handle in a thread_local!). When siblings are already
    // destroyed, we silently skip the unreachable cleanup.
    let _ = INSTALLED.try_with(|c| c.set(false));
    clear_callbacks();

    let lp = lp_entity();
    if lp.is_null() {
        return;
    }

    // SAFETY: lp is non-null.
    let current: *const EntityVTABLE = unsafe { (*lp).VTABLE };
    let our_ptr = OUR_VTABLE.try_with(Cell::get).ok().flatten();

    if our_ptr == Some(current) {
        // We are the head: restore the prior VTABLE pointer first, then drop
        // our box and clear all state.
        let prior = ORIGINAL_VTABLE.try_with(Cell::take).ok().flatten();
        // SAFETY: writing the per-entity VTABLE pointer; the prior pointer is
        // valid (it was saved from the live slot at push time).
        unsafe { (*lp).VTABLE = prior.unwrap_or(current) };
        let _ = VTABLE_BOX.try_with(|c| c.set(None)); // drops our Box
        let _ = OUR_VTABLE.try_with(|c| c.set(None));
        let _ = IN_CHAIN.try_with(|c| c.set(false));
    }
    // Else (buried): leave VTABLE_BOX, OUR_VTABLE, ORIGINAL_VTABLE, and IN_CHAIN
    // alive. Trampolines are still reachable from above and forward transparently
    // with callbacks cleared. IN_CHAIN stays true so a reload re-arms in place
    // rather than re-pushing into a cycle. ORIGINAL_VTABLE stays set so the buried
    // forwarders can still resolve their original fn pointer.
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// RAII handle for a chain-safe `EntityVTABLE` hook on the local player.
///
/// Install with [`new`](Self::new); dropping uninstalls (if on top) or
/// degrades to a transparent forwarder (if buried). No reinstall method is
/// needed -- the local player VTABLE is never wiped between map loads.
#[must_use = "dropping this handle immediately uninstalls the hook"]
pub struct LocalPlayerVTableHook {
    _private: (),
}

impl LocalPlayerVTableHook {
    /// Install callbacks for the requested fields and return a RAII handle.
    ///
    /// Returns `None` if the local player entity slot is still null (e.g.
    /// called from `Init` before the first map load). Safe to retry; call again
    /// from `on_new_map_loaded`.
    ///
    /// If this plugin's box is already believed live in the chain (e.g. the
    /// previous handle was dropped while buried under a foreign plugin), the
    /// new callbacks are armed in place without re-pushing. Re-pushing while
    /// buried would form a cycle. The hooked field set is frozen from the
    /// original push; supply the same fields on reload.
    ///
    /// # Panics
    ///
    /// Panics if a hook is already installed for this plugin binary. Drop the
    /// existing handle before calling `new` again.
    pub fn new(hooks: LocalPlayerVTableHooks) -> Option<Self> {
        let lp = lp_entity();
        if lp.is_null() {
            return None;
        }

        assert!(
            !INSTALLED.with(Cell::get),
            "LocalPlayerVTableHook already installed; drop the existing handle before calling new \
             again",
        );
        INSTALLED.with(|c| c.set(true));

        // push() reads hooks.*.is_some() to decide which fields to bake, so it
        // must run before arm_all() consumes `hooks`.
        if !IN_CHAIN.with(Cell::get) {
            push(lp, &hooks);
        }
        // Else: buried reload -- box and trampolines are already live; arm_all
        // below just re-arms the callbacks.

        arm_all(hooks);

        Some(Self { _private: () })
    }
}

impl Drop for LocalPlayerVTableHook {
    fn drop(&mut self) {
        uninstall_inner();
    }
}
