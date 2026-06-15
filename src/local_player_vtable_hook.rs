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
// once: the `LocalPlayerVTableHooks` struct plus the `wrap_all_fields` /
// `arm_callbacks` / `clear_callbacks` helpers that drive the per-field items,
// so adding or removing a hooked field is a single-line change to the table
// below.
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
                        // dispatch of this field from inside the callback hits the
                        // held borrow and panics with BorrowMutError. The crash is
                        // deliberate -- re-entrancy is a consumer bug to fix, not
                        // something this trampoline silently absorbs.
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
            /// Any field subset may be supplied on any install or reload. The
            /// replacement VTABLE box wraps every field up front, so a later reload
            /// (drop then `install` again) is free to arm or disarm any field --
            /// even one not requested originally -- regardless of whether the box is
            /// buried. Fields with no callback forward transparently.
            #[derive(Default)]
            pub struct LocalPlayerVTableHooks {
                $( pub $snake: Option<Box<[<$vtable_field Cb>]>>, )*
            }

            /// Wrap our trampoline around **every populated field** of the
            /// replacement VTABLE copy `v` -- not just the subset a consumer
            /// requested. A foreign plugin that later pushes on top snapshots our
            /// box wholesale, so a field carries our trampoline into that snapshot
            /// only if we wrapped it here; wrapping every field keeps all of them
            /// reachable through any number of layers, which is what lets a buried
            /// reload arm a field the original install never requested. Un-armed
            /// fields forward transparently (their callback slot is `None`), so
            /// wrapping them is behaviour-neutral aside from one extra indirection
            /// per dispatch. Null stock fields are left null (never wrapped), so the
            /// engine's dispatch decision for them is unchanged.
            fn wrap_all_fields(v: &mut EntityVTABLE) {
                $( if v.$vtable_field.is_some() { v.$vtable_field = Some([<$snake _tramp>]); } )*
            }

            /// Move each supplied callback into its thread-local slot, consuming
            /// `hooks`. A field with no callback is left untouched here; on a reload
            /// that drops a field, the prior callback was already cleared by
            /// [`clear_callbacks`] on uninstall, so the surviving trampoline
            /// degrades to a transparent forwarder.
            fn arm_callbacks(hooks: LocalPlayerVTableHooks) {
                $(
                    if let Some(cb) = hooks.$snake {
                        [<CALLBACK_ $vtable_field:upper>].with(|c| *c.borrow_mut() = Some(cb));
                    }
                )*
            }

            /// Clear every callback slot. Used on uninstall (head or buried) so a
            /// later `install()` does not trip the double-install assert and any
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
    /// Keeps the replacement box alive and doubles as our identity pointer
    /// ("are we on top?" = `(*lp).VTABLE == our_box_ptr`). Must not be dropped
    /// while buried: a hook above us saved our box pointer as its
    /// ORIGINAL_VTABLE; freeing it would dangle that pointer and SIGSEGV on
    /// the next VTABLE dispatch.
    static VTABLE_BOX: RefCell<Option<Box<EntityVTABLE>>> = const { RefCell::new(None) };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read `Entities.List[ENTITIES_SELF_ID]`, the local player entity.
///
/// The engine populates this slot in `Entities_Init`, which runs before any
/// plugin's `Init` (the `Entities` component is added ahead of plugin
/// components, and `Init` runs in add-order), so it is present for the whole
/// time a plugin is loaded. The one window in which it is null is during
/// shutdown, after `Entities_Free` removes it -- a Drop firing then sees an
/// empty slot, which the uninstall path tolerates.
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

/// Push our replacement VTABLE box onto the chain head.
fn push(lp: *mut Entity) {
    // SAFETY: lp is non-null (install() asserts it before calling push).
    let current: *const EntityVTABLE = unsafe { (*lp).VTABLE };

    let our_ptr = VTABLE_BOX.with(|c| c.borrow().as_deref().map(ptr::from_ref));
    if !should_push(current, our_ptr) {
        // Already on top -- just mark in-chain and return.
        IN_CHAIN.with(|c| c.set(true));
        return;
    }

    // Copy all six fields (invariant 1: carry forward any foreign field overrides
    // already present in the live VTABLE at push time), then wrap every field with
    // our trampoline. Foreign overrides stay reachable: our trampoline forwards to
    // ORIGINAL_VTABLE (this copied-from box), so their fn pointers live on as our
    // originals rather than as our active fields.
    // SAFETY: current is the live entity VTABLE, valid while the entity is live.
    let mut v: EntityVTABLE = unsafe { *current };
    wrap_all_fields(&mut v);

    let boxed = Box::new(v);
    // Obtain the heap address via a shared reference before moving the Box into
    // the Cell. The heap allocation does not move when the Box handle moves.
    let ptr: *const EntityVTABLE = ptr::from_ref(boxed.as_ref());

    // ORIGINAL_VTABLE must be set before the slot is repointed: the trampolines
    // read it the instant the engine dispatches through our box.
    ORIGINAL_VTABLE.with(|c| c.set(Some(current)));
    // SAFETY: writing the per-entity VTABLE pointer; lp is valid.
    unsafe { (*lp).VTABLE = ptr };
    VTABLE_BOX.with(|c| *c.borrow_mut() = Some(boxed));
    IN_CHAIN.with(|c| c.set(true));
}

fn uninstall_inner() {
    // Clear all callbacks unconditionally, even when buried: prevents the
    // double-install assert from firing on a later install(), and degrades any
    // surviving trampoline to a transparent forwarder.
    //
    // try_with throughout: Drop may fire during TLS teardown (e.g. if the
    // consumer stores the handle in a thread_local!). When siblings are already
    // destroyed, we silently skip the unreachable cleanup.
    let _ = INSTALLED.try_with(|c| c.set(false));
    clear_callbacks();

    // On shutdown, Entities_Free removes the local player before plugin Free
    // runs, so a Drop fired during teardown legitimately sees an empty slot.
    // There is nothing to restore then -- the entity is gone.
    let lp = lp_entity();
    if lp.is_null() {
        return;
    }

    // SAFETY: lp is non-null.
    let current: *const EntityVTABLE = unsafe { (*lp).VTABLE };
    let our_ptr = VTABLE_BOX
        .try_with(|c| c.borrow().as_deref().map(ptr::from_ref))
        .ok()
        .flatten();

    if our_ptr == Some(current) {
        // We are the head: restore the prior VTABLE pointer first, then drop
        // our box and clear all state.
        let prior = ORIGINAL_VTABLE.try_with(Cell::take).ok().flatten();
        // SAFETY: writing the per-entity VTABLE pointer; the prior pointer is
        // valid (it was saved from the live slot at push time).
        unsafe { (*lp).VTABLE = prior.unwrap_or(current) };
        let _ = VTABLE_BOX.try_with(|c| *c.borrow_mut() = None); // drops our Box
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
/// Install with [`install`](Self::install); dropping uninstalls (if on top) or
/// degrades to a transparent forwarder (if buried). No reinstall method is
/// needed -- the local player VTABLE is never wiped between map loads.
#[must_use = "dropping this handle immediately uninstalls the hook"]
pub struct LocalPlayerVTableHook {
    _private: (),
}

impl LocalPlayerVTableHook {
    /// Install callbacks for the requested fields and return a RAII handle.
    ///
    /// If this plugin's box is already believed live in the chain (e.g. the
    /// previous handle was dropped while buried under a foreign plugin), the
    /// new callbacks are armed in place without re-pushing. Re-pushing while
    /// buried would form a cycle. The box wraps every field up front, so the
    /// reload's field set is free to differ from the original install's.
    ///
    /// # Panics
    ///
    /// Panics if the local player entity slot is unset (it is populated by the
    /// engine before plugin `Init` runs, so this should not happen in practice),
    /// or if a hook is already installed for this plugin binary -- drop the
    /// existing handle before calling `install` again.
    pub fn install(hooks: LocalPlayerVTableHooks) -> Self {
        let lp = lp_entity();
        assert!(
            !lp.is_null(),
            "local player entity (Entities.List[ENTITIES_SELF_ID]) is unset; the Entities \
             component populates it before plugin Init runs",
        );

        assert!(
            !INSTALLED.with(Cell::get),
            "LocalPlayerVTableHook already installed; drop the existing handle before calling \
             install again",
        );
        INSTALLED.with(|c| c.set(true));

        if !IN_CHAIN.with(Cell::get) {
            push(lp);
        }
        // Else: buried reload -- box and trampolines are already live; arm_callbacks
        // below just re-arms the callbacks. The box wraps every field, so a reload
        // may arm fields the original install did not request.

        arm_callbacks(hooks);

        Self { _private: () }
    }
}

impl Drop for LocalPlayerVTableHook {
    fn drop(&mut self) {
        uninstall_inner();
    }
}
