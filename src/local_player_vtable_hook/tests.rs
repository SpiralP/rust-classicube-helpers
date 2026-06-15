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
// Full-bake field model
//
// Models per-field dispatch under full-bake: every push wraps EVERY field with
// the pushing plugin's trampoline (mirroring `wrap_all_fields`, which patches
// all populated fields, not just a requested subset). A box is a
// `Vt { tick, render }` whose values are the trampoline-owner id per field
// (0 = stock). Because every push overwrites both fields with its own id, every
// field's dispatch traverses the full box chain via the saved `original` link
// -- so a buried plugin stays reachable for *every* field, including one it
// never originally armed. That reachability is what lets a buried reload arm a
// new field without re-pushing.
//
// Exercises the same invariants as before, recast for full-bake:
//   1. Copy all fields on push, then wrap every field with our trampoline.
//   2. Box-pointer identity: uninstall keyed to box id, not field value.
//   3. Keep box alive while buried (it stays in every field's chain).
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
struct Vt {
    tick: u32,
    render: u32,
}

/// A pushed box: its per-field trampoline owners plus the box it sits above
/// (mirrors `ORIGINAL_VTABLE`; 0 = stock).
#[derive(Clone, Copy)]
struct BoxEntry {
    vt: Vt,
    original: u32,
}

struct BoxPlugin {
    id: u32,              // this plugin's trampoline-owner id
    box_id: Option<u32>,  // box it owns (None = not pushed); mirrors VTABLE_BOX
    original_box_id: u32, // box in the slot at push time; mirrors ORIGINAL_VTABLE
    in_chain: bool,
}

impl BoxPlugin {
    fn new(id: u32) -> Self {
        Self {
            id,
            box_id: None,
            original_box_id: 0,
            in_chain: false,
        }
    }

    fn push(&mut self, slot: &mut u32, boxes: &mut Vec<BoxEntry>) {
        if self.box_id == Some(*slot) {
            // Already on top (identity is box_id).
            self.in_chain = true;
            return;
        }
        // Full-bake: wrap BOTH fields with our id. The prior slot is saved as
        // `original`, so layers below stay reachable through the chain.
        self.original_box_id = *slot;
        boxes.push(BoxEntry {
            vt: Vt {
                tick: self.id,
                render: self.id,
            },
            original: *slot,
        });
        #[expect(clippy::cast_possible_truncation)]
        let id = boxes.len() as u32; // box_id = 1-based index (non-zero)
        *slot = id;
        self.box_id = Some(id);
        self.in_chain = true;
    }

    fn uninstall(&mut self, slot: &mut u32) {
        if self.box_id == Some(*slot) {
            // Head uninstall: restore prior slot (identity is box_id).
            *slot = self.original_box_id;
            self.in_chain = false;
        }
        // Buried: leave box alive so the hook above keeps a valid pointer.
    }
}

/// Walk the per-field dispatch chain from the slot head down to stock,
/// collecting each box's trampoline-owner for `field`. Under full-bake this
/// visits every box in the chain, for every field.
fn field_chain(slot: u32, boxes: &[BoxEntry], field: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let mut cur = slot;
    while cur != 0 {
        assert!(out.len() <= boxes.len(), "cycle detected");
        let entry = boxes[cur as usize - 1];
        out.push(match field {
            "tick" => entry.vt.tick,
            "render" => entry.vt.render,
            _ => unreachable!(),
        });
        cur = entry.original;
    }
    out
}

#[test]
fn push_wraps_every_field() {
    let mut slot: u32 = 0;
    let mut boxes: Vec<BoxEntry> = Vec::new();
    let mut a = BoxPlugin::new(11);
    a.push(&mut slot, &mut boxes);
    assert_eq!(
        boxes[0].vt,
        Vt {
            tick: 11,
            render: 11
        },
        "full-bake wraps every field, not just a requested subset",
    );
}

#[test]
fn two_plugins_compose_and_restore() {
    // T installs, R installs on top. Every box wraps every field, so both
    // fields traverse R then T. R head-uninstall restores T's box.
    let mut slot: u32 = 0;
    let mut boxes: Vec<BoxEntry> = Vec::new();

    let mut t = BoxPlugin::new(11);
    let mut r = BoxPlugin::new(22);

    t.push(&mut slot, &mut boxes);
    r.push(&mut slot, &mut boxes);
    assert_eq!(field_chain(slot, &boxes, "tick"), vec![22, 11]);
    assert_eq!(field_chain(slot, &boxes, "render"), vec![22, 11]);

    r.uninstall(&mut slot); // head: restore T's box
    assert_eq!(slot, t.box_id.unwrap(), "slot restored to T's box id");
    assert_eq!(field_chain(slot, &boxes, "tick"), vec![11]);
    assert_eq!(field_chain(slot, &boxes, "render"), vec![11]);
}

#[test]
fn buried_plugin_reachable_for_field_not_originally_armed() {
    // The full-bake payoff. A installs serving only `tick`; B buries it serving
    // `render`. Because A's box wrapped `render` too, A stays in the render
    // chain -- so a buried reload of A can arm `render` and it will fire. Under
    // the old subset-bake, A's box held `render` = stock, B's copy bypassed A
    // for render, and the late-armed callback silently never ran.
    let mut slot: u32 = 0;
    let mut boxes: Vec<BoxEntry> = Vec::new();

    let mut a = BoxPlugin::new(11);
    let mut b = BoxPlugin::new(22);

    a.push(&mut slot, &mut boxes); // box 1: { tick:11, render:11 }
    b.push(&mut slot, &mut boxes); // box 2: { tick:22, render:22 } over box 1
    a.uninstall(&mut slot); // buried no-op
    assert!(a.in_chain, "buried uninstall keeps in_chain");

    assert_eq!(
        field_chain(slot, &boxes, "render"),
        vec![22, 11],
        "A reachable in the render chain despite never originally arming render",
    );
    assert_eq!(field_chain(slot, &boxes, "tick"), vec![22, 11]);
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
