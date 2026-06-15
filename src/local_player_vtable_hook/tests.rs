use super::*;

// ---------------------------------------------------------------------------
// should_push (pure, no FFI)
// ---------------------------------------------------------------------------

#[test]
fn should_push_never_installed_is_true() {
    let addr = 0x1000usize as *const EntityVTABLE;
    assert!(should_push(addr, None));
}

#[test]
fn should_push_already_on_top_is_false() {
    let addr = 0x1000usize as *const EntityVTABLE;
    assert!(!should_push(addr, Some(addr)));
}

#[test]
fn should_push_different_ptr_is_true() {
    let a = 0x1000usize as *const EntityVTABLE;
    let b = 0x2000usize as *const EntityVTABLE;
    assert!(should_push(a, Some(b)));
}

// ---------------------------------------------------------------------------
// Chain model (pure, Linux-CI safe)
//
// Models the swap chain with plain integers: STOCK=0, each plugin has a
// unique non-zero id. A plugin's box is its id; "identity" is an integer
// comparison (mirroring the real box-pointer equality). Covers the same
// cases as protocol_hook/tests.rs but without wipe/reinstall (the VTABLE
// is never wiped).
// ---------------------------------------------------------------------------

const STOCK: u32 = 0;

struct Plugin {
    id: u32,
    /// The slot value at push time (what we restore on head-uninstall).
    original: u32,
    /// Mirrors `IN_CHAIN`: whether our id is believed reachable in the chain.
    in_chain: bool,
}

impl Plugin {
    fn new(id: u32) -> Self {
        Self {
            id,
            original: STOCK,
            in_chain: false,
        }
    }

    /// Push onto the chain head, or no-op if already on top.
    fn push(&mut self, slot: &mut u32) {
        if *slot == self.id {
            self.in_chain = true;
            return;
        }
        self.original = *slot;
        *slot = self.id;
        self.in_chain = true;
    }

    /// Re-arm in place when buried (reload), otherwise push.
    fn install(&mut self, slot: &mut u32) {
        if !self.in_chain {
            self.push(slot);
        }
        // else: buried reload -- re-arm callbacks in place, no re-push
    }

    /// Splice out when head; leave everything when buried (keep box alive).
    fn uninstall(&mut self, slot: &mut u32) {
        if *slot == self.id {
            *slot = self.original;
            self.in_chain = false;
        }
        // else buried: leave slot, original, in_chain intact
    }
}

/// Follow the chain from the head, hopping each plugin's saved `original`,
/// until STOCK. A correct chain visits each plugin exactly once.
fn traverse(slot: u32, plugins: &[Plugin]) -> Vec<u32> {
    let mut visited = Vec::new();
    let mut cur = slot;
    while cur != STOCK {
        assert!(!visited.contains(&cur), "cycle detected at id {cur}");
        visited.push(cur);
        cur = plugins
            .iter()
            .find(|p| p.id == cur)
            .map_or(STOCK, |p| p.original);
    }
    visited
}

#[test]
fn single_install_then_head_uninstall() {
    let mut slot = STOCK;
    let mut a = Plugin::new(1);
    a.install(&mut slot);
    assert_eq!(slot, 1);
    a.uninstall(&mut slot);
    assert_eq!(slot, STOCK);
    assert!(!a.in_chain);
}

#[test]
fn two_plugins_chain_order() {
    let mut slot = STOCK;
    let mut a = Plugin::new(1);
    let mut b = Plugin::new(2);
    a.install(&mut slot);
    b.install(&mut slot);
    assert_eq!(traverse(slot, &[a, b]), vec![2, 1]);
}

#[test]
fn double_install_same_plugin_is_noop() {
    let mut slot = STOCK;
    let mut a = Plugin::new(1);
    a.install(&mut slot);
    a.install(&mut slot); // in_chain -> re-arm only, no push
    assert_eq!(a.original, STOCK, "original must not become self");
    assert_eq!(traverse(slot, &[a]), vec![1]);
}

#[test]
fn reload_while_buried_does_not_cycle() {
    let mut slot = STOCK;
    let mut a = Plugin::new(1);
    let mut b = Plugin::new(2);
    a.install(&mut slot);
    b.install(&mut slot); // A buried under B
    a.uninstall(&mut slot); // buried no-op: slot stays 2, a.in_chain stays true
    a.install(&mut slot); // in_chain -> re-arm only, no push
    assert_eq!(
        a.original, STOCK,
        "A's forward target must stay STOCK, not B"
    );
    assert_eq!(traverse(slot, &[a, b]), vec![2, 1]);
}

#[test]
#[should_panic(expected = "cycle detected")]
fn reload_while_buried_old_behavior_cycles() {
    // Pins the pre-fix bug: unconditional push while buried forms a cycle.
    let mut slot = STOCK;
    let mut a = Plugin::new(1);
    let mut b = Plugin::new(2);
    a.push(&mut slot);
    b.push(&mut slot);
    a.uninstall(&mut slot); // buried no-op
    a.push(&mut slot); // re-push while buried: a.original=2, b.original=1 -> cycle
    let _ = traverse(slot, &[a, b]);
}

#[test]
fn reload_while_head_repushes() {
    let mut slot = STOCK;
    let mut a = Plugin::new(1);
    a.install(&mut slot);
    a.uninstall(&mut slot); // head splice: slot->STOCK, in_chain=false
    assert!(!a.in_chain);
    a.install(&mut slot); // in_chain false -> push
    assert!(a.in_chain);
    assert_eq!(traverse(slot, &[a]), vec![1]);
}

#[test]
fn drop_while_buried_keeps_in_chain() {
    let mut slot = STOCK;
    let mut a = Plugin::new(1);
    let mut b = Plugin::new(2);
    a.install(&mut slot);
    b.install(&mut slot); // A buried
    a.uninstall(&mut slot); // buried no-op
    assert!(a.in_chain, "buried uninstall must not clear in_chain");
}

// ---------------------------------------------------------------------------
// Multi-field-in-one-box composition
//
// Models two plugins A and B each holding one box. A's box covers field
// `tick`, B's box covers field `render`. The VTABLE copy is modeled as a
// tiny struct `Vt { tick: u32, render: u32 }` where 0 = stock trampoline
// and non-zero = a plugin's trampoline id.
//
// This directly exercises invariants 1-3 from the module docs:
//   1. Copy all fields on push (non-hooked fields survive).
//   2. Box-pointer identity: uninstall keyed to box id, not field value.
//   3. Keep box alive while buried (field values inside it survive in the
//      live box above).
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
struct Vt {
    tick: u32,
    render: u32,
}

const STOCK_VT: Vt = Vt { tick: 0, render: 0 };

struct BoxPlugin {
    // The box ID this plugin currently owns (None = not yet pushed). Mirrors
    // OUR_VTABLE: the heap address of our Box<EntityVTABLE>.
    box_id: Option<u32>,
    // The box ID that was in the slot when we pushed. Mirrors ORIGINAL_VTABLE.
    original_box_id: u32,
    field: &'static str, // "tick" or "render"
    tramp_id: u32,       // the non-zero value written into the field
    in_chain: bool,
}

impl BoxPlugin {
    fn new(field: &'static str, tramp_id: u32) -> Self {
        Self {
            box_id: None,
            original_box_id: 0,
            field,
            tramp_id,
            in_chain: false,
        }
    }

    fn push(&mut self, slot: &mut u32, boxes: &mut Vec<Vt>) {
        if self.box_id == Some(*slot) {
            // Already on top (identity is box_id, invariant 2).
            self.in_chain = true;
            return;
        }
        // Invariant 1: copy ALL fields from the live VTABLE before patching.
        let current_vt = if *slot == 0 {
            STOCK_VT
        } else {
            boxes[*slot as usize - 1]
        };
        let mut new_vt = current_vt;
        match self.field {
            "tick" => new_vt.tick = self.tramp_id,
            "render" => new_vt.render = self.tramp_id,
            _ => unreachable!(),
        }
        self.original_box_id = *slot; // save prior slot (mirrors ORIGINAL_VTABLE)
        boxes.push(new_vt);
        #[expect(clippy::cast_possible_truncation)]
        let id = boxes.len() as u32; // box_id = 1-based index (non-zero)
        *slot = id;
        self.box_id = Some(id);
        self.in_chain = true;
    }

    fn uninstall(&mut self, slot: &mut u32) {
        if self.box_id == Some(*slot) {
            // Head uninstall: restore prior slot (invariant 2: identity is box_id).
            *slot = self.original_box_id;
            self.in_chain = false;
        }
        // Buried: leave box alive so the hook above keeps a valid pointer (invariant 3).
    }
}

fn live_vt(slot: u32, boxes: &[Vt]) -> Vt {
    if slot == 0 {
        STOCK_VT
    } else {
        boxes[slot as usize - 1]
    }
}

#[test]
fn one_box_two_fields_head_uninstall_restores_stock() {
    // One plugin hooks BOTH tick and render in a single box (simulated by
    // pushing and then manually patching the second field in the same box).
    let mut slot: u32 = 0;
    let mut boxes: Vec<Vt> = Vec::new();
    let mut a = BoxPlugin::new("tick", 11);
    a.push(&mut slot, &mut boxes); // box 1: { tick:11, render:0 }
    boxes[slot as usize - 1].render = 22; // patch render in same box
    assert_eq!(
        live_vt(slot, &boxes),
        Vt {
            tick: 11,
            render: 22
        },
        "both fields in one box"
    );
    a.uninstall(&mut slot);
    assert_eq!(slot, 0, "slot restored to stock");
}

#[test]
fn two_plugins_different_fields_compose() {
    // Plugin T hooks tick, Plugin R hooks render.
    // T installs, R installs on top (copying T's box -- invariant 1 carries tick).
    // R head-uninstall restores T's box: tick survives, render reverts to 0.
    let mut slot: u32 = 0;
    let mut boxes: Vec<Vt> = Vec::new();

    let mut t = BoxPlugin::new("tick", 11);
    let mut r = BoxPlugin::new("render", 22);

    t.push(&mut slot, &mut boxes); // box 1: { tick:11, render:0 }
    assert_eq!(
        boxes[0],
        Vt {
            tick: 11,
            render: 0
        }
    );

    r.push(&mut slot, &mut boxes); // box 2: copy of box1, render patched
    assert_eq!(
        boxes[1],
        Vt {
            tick: 11,
            render: 22
        },
        "invariant 1: tick survives in R's box"
    );
    assert_eq!(
        live_vt(slot, &boxes),
        Vt {
            tick: 11,
            render: 22
        }
    );

    r.uninstall(&mut slot); // head: restore T's box
    assert_eq!(slot, t.box_id.unwrap(), "slot restored to T's box id");
    assert_eq!(
        live_vt(slot, &boxes),
        Vt {
            tick: 11,
            render: 0
        },
        "after R head-uninstall: tick survives, render reverts"
    );
}

#[test]
fn drop_buried_plugin_leaves_tramp_in_live_box() {
    // Plugin R installs, T installs on top. R is dropped while buried.
    // T's live box still holds R's render trampoline (invariant 3: R's box
    // pointer is inside T's original_vt; it stays alive).
    let mut slot: u32 = 0;
    let mut boxes: Vec<Vt> = Vec::new();

    let mut r = BoxPlugin::new("render", 22);
    let mut t = BoxPlugin::new("tick", 11);

    r.push(&mut slot, &mut boxes); // box 1: { tick:0, render:22 }
    t.push(&mut slot, &mut boxes); // box 2: copy of box1, tick patched
    assert_eq!(
        boxes[1],
        Vt {
            tick: 11,
            render: 22
        }
    );

    r.uninstall(&mut slot); // R buried: no-op, box alive (invariant 3)
    assert!(r.in_chain, "buried uninstall keeps in_chain");
    // T's live box still holds R's render trampoline value.
    assert_eq!(
        live_vt(slot, &boxes),
        Vt {
            tick: 11,
            render: 22
        },
        "render trampoline survives in T's live box (invariant 3)"
    );
}

// ---------------------------------------------------------------------------
// FFI-touching tests (Windows + non-CI only)
// ---------------------------------------------------------------------------

#[cfg(all(windows, not(feature = "ci")))]
mod windows_ffi {
    use super::*;

    fn read_vtable() -> *const EntityVTABLE {
        unsafe {
            let lp = lp_entity();
            assert!(!lp.is_null(), "local player must be present");
            (*lp).VTABLE
        }
    }

    #[test]
    #[ignore]
    fn new_puts_box_on_top() {
        let before = read_vtable();
        let hook = LocalPlayerVTableHook::install(LocalPlayerVTableHooks {
            render_model: Some(Box::new(|e, d, t, orig| unsafe { orig(e, d, t) })),
            ..Default::default()
        });
        assert_ne!(
            read_vtable(),
            before,
            "VTABLE pointer should change after install"
        );
        drop(hook);
    }

    #[test]
    #[ignore]
    fn drop_restores_prior_vtable() {
        let before = read_vtable();
        let hook = LocalPlayerVTableHook::install(LocalPlayerVTableHooks {
            render_model: Some(Box::new(|e, d, t, orig| unsafe { orig(e, d, t) })),
            ..Default::default()
        });
        drop(hook);
        assert_eq!(
            read_vtable(),
            before,
            "VTABLE pointer should be restored after drop"
        );
    }

    #[test]
    #[ignore]
    fn render_tramp_around_calls_original_and_callback() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static CALLED: AtomicBool = AtomicBool::new(false);
        let _hook = LocalPlayerVTableHook::install(LocalPlayerVTableHooks {
            render_model: Some(Box::new(|e, d, t, orig| {
                CALLED.store(true, Ordering::SeqCst);
                unsafe { orig(e, d, t) };
            })),
            ..Default::default()
        });
        // The callback fires on the next render frame; this test requires a live
        // ClassiCube session to drive a render tick before asserting.
        assert!(CALLED.load(Ordering::SeqCst));
        drop(_hook);
    }

    #[test]
    #[ignore]
    fn get_col_tramp_can_replace_return() {
        let lp = unsafe { &mut *lp_entity() };
        let _hook = LocalPlayerVTableHook::install(LocalPlayerVTableHooks {
            get_col: Some(Box::new(|_e, _orig| 0xFF_FF_00_FF)), // opaque yellow
            ..Default::default()
        });
        // Drive a render tick in a live session and verify the player renders
        // with the replaced colour.
        drop(_hook);
        let _ = lp;
    }
}
