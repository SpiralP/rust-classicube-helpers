use std::hint;

use super::*;

// ------------- handlers_eq (pure, using local dummy fns) ------------------

// Distinct, side-effect-free bodies on purpose: two byte-identical empty
// `extern "C"` functions get folded to one address by LLVM's function-merging
// pass in optimized (release) builds -- e.g. the `linux_nix` CI job -- which
// would make `ptr::fn_addr_eq` report them equal. `ptr::fn_addr_eq` does not
// guarantee distinct functions have distinct addresses, so each gets a unique
// `black_box` constant that survives optimization and keeps the addresses apart.
unsafe extern "C" fn dummy_a(_: *mut u8) {
    hint::black_box(0xAA_u8);
}
unsafe extern "C" fn dummy_b(_: *mut u8) {
    hint::black_box(0xBB_u8);
}

#[test]
fn handlers_eq_same_fn_is_true() {
    assert!(handlers_eq(Some(dummy_a), Some(dummy_a)));
}

#[test]
fn handlers_eq_different_fn_is_false() {
    assert!(!handlers_eq(Some(dummy_a), Some(dummy_b)));
}

#[test]
fn handlers_eq_both_none_is_true() {
    assert!(handlers_eq(None, None));
}

#[test]
fn handlers_eq_one_none_is_false() {
    assert!(!handlers_eq(Some(dummy_a), None));
    assert!(!handlers_eq(None, Some(dummy_a)));
}

// ------------- install_step chain model (pure, Linux-CI safe) -------------
//
// Simulates N plugins detour-chaining a single shared slot: a ClassiCube wipe
// that resets the slot to the stock handler WITHOUT clearing each plugin's
// cached `old`/in_chain, re-installing in arbitrary order, and reloading a
// plugin (drop then re-create) while it is buried under another. The harness
// mirrors the live install/reinstall/uninstall logic (re-arm-vs-push gated by
// `in_chain`) and uses the same `install_step` the live `install_inner` uses;
// here the identity type is a plain integer instead of `HandlerId`. `traverse`
// has a cycle-guard so a chain that loops back on itself fails fast.

const STOCK: u32 = 0;

struct Plugin {
    id: u32,
    old: u32,
    // Mirror of the live IN_CHAIN thread-local: whether our id is believed
    // reachable in the chain (as head or buried).
    in_chain: bool,
}

impl Plugin {
    fn new(id: u32) -> Self {
        // Mirrors the live thread-local OLD, which holds nothing meaningful
        // until the first install; STOCK is a safe initial sentinel.
        Self {
            id,
            old: STOCK,
            in_chain: false,
        }
    }

    // Mirror of install_inner (the push primitive): read the shared slot, run
    // install_step, and on Some(displaced) cache it as our `old` and write
    // ourselves into the slot. Marks in_chain on both outcomes.
    fn push(&mut self, slot: &mut u32) {
        if let Some(displaced) = install_step(*slot, self.id) {
            self.old = displaced;
            *slot = self.id;
        }
        self.in_chain = true;
    }

    // Mirror of install(): re-arm in place when already in the chain (a reload
    // while buried), otherwise push. Re-pushing while buried would form a cycle.
    fn install(&mut self, slot: &mut u32) {
        if !self.in_chain {
            self.push(slot);
        }
    }

    // Mirror of reinstall(): always force-push post-wipe, clearing the stale
    // in_chain belief first so it cannot veto the re-push (the reorder bug).
    fn reinstall(&mut self, slot: &mut u32) {
        self.in_chain = false;
        self.push(slot);
    }

    // Mirror of uninstall_inner: splice out only when we are the head, clearing
    // in_chain only then. A buried uninstall leaves the slot, old, and in_chain
    // untouched -- the link above still references us.
    fn uninstall(&mut self, slot: &mut u32) {
        if *slot == self.id {
            *slot = self.old;
            self.in_chain = false;
        }
    }
}

// Mirror of ClassiCube's Protocol wipe: the slot reverts to the stock handler.
// Deliberately does NOT touch any plugin's cached `old` -- that staleness is
// exactly the condition that exposed the original reorder bug.
fn wipe(slot: &mut u32) {
    *slot = STOCK;
}

// Follow the chain from the head of the slot, hopping each plugin's cached
// `old`, returning the visited id sequence. A correct chain visits each live
// plugin exactly once and ends at STOCK.
fn traverse(slot: u32, plugins: &[Plugin]) -> Vec<u32> {
    let mut visited = Vec::new();
    let mut cur = slot;
    while cur != STOCK {
        // Cycle guard so a self-loop regression fails fast instead of hanging.
        assert!(!visited.contains(&cur), "cycle detected at id {cur}");
        visited.push(cur);
        cur = plugins
            .iter()
            .find(|p| p.id == cur)
            .map_or(STOCK, |p| p.old);
    }
    visited
}

#[test]
fn install_step_already_on_top_is_none() {
    assert_eq!(install_step(7_u32, 7_u32), None);
}

#[test]
fn install_step_not_on_top_returns_displaced() {
    assert_eq!(install_step(3_u32, 7_u32), Some(3_u32));
}

#[test]
fn reinstall_reverse_order_rebuilds_chain() {
    // install A then B, wipe, re-install in REVERSE order (B then A). Under the
    // deleted `current != old` bail, B would bail -- its stale `old` still
    // pointed at A while the wiped slot held STOCK -- and be lost. The fix
    // re-installs both regardless of order.
    let mut slot = STOCK;
    let mut a = Plugin::new(1);
    let mut b = Plugin::new(2);

    a.install(&mut slot); // slot: 1, a.old: STOCK
    b.install(&mut slot); // slot: 2, b.old: 1

    wipe(&mut slot); // slot: STOCK; a.old/b.old left stale, in_chain left true

    // Post-wipe re-push is reinstall() (from each component's Reset), which
    // force-pushes despite the stale in_chain flag.
    b.reinstall(&mut slot); // slot: 2, b.old: STOCK
    a.reinstall(&mut slot); // slot: 1, a.old: 2

    let order = traverse(slot, &[a, b]);
    assert!(order.contains(&1), "A must be reachable after reinstall");
    assert!(order.contains(&2), "B must be reachable after reinstall");
    assert_eq!(order.len(), 2, "each plugin reachable exactly once");
}

#[test]
fn reinstall_same_order_rebuilds_chain() {
    let mut slot = STOCK;
    let mut a = Plugin::new(1);
    let mut b = Plugin::new(2);

    a.install(&mut slot);
    b.install(&mut slot);
    wipe(&mut slot);

    a.reinstall(&mut slot); // slot: 1, a.old: STOCK
    b.reinstall(&mut slot); // slot: 2, b.old: 1

    let order = traverse(slot, &[a, b]);
    assert_eq!(order, vec![2, 1], "head B -> A -> STOCK");
}

#[test]
fn double_install_same_plugin_is_noop() {
    // Installing the same plugin twice without a wipe must not self-chain. The
    // second install short-circuits on in_chain before touching the slot.
    let mut slot = STOCK;
    let mut a = Plugin::new(1);

    a.install(&mut slot); // slot: 1, a.old: STOCK, in_chain: true
    a.install(&mut slot); // in_chain already true -> re-arm only, no push

    assert_eq!(a.old, STOCK, "old must not become self");
    let order = traverse(slot, &[a]);
    assert_eq!(order, vec![1], "single hop to STOCK, no self-loop");
}

#[test]
fn reload_while_buried_does_not_cycle() {
    // A installs, B chains on top, then A is reloaded (Free=uninstall while
    // buried, Init=install) while still buried under B. Under the old
    // unconditional push, A.install would set a.old=2 while b.old=1, forming
    // 1 -> 2 -> 1, and traverse's cycle-guard would fire. The in_chain re-arm
    // guard keeps A in place instead of re-pushing.
    let mut slot = STOCK;
    let mut a = Plugin::new(1);
    let mut b = Plugin::new(2);

    a.install(&mut slot); // slot: 1, a.old: STOCK, a.in_chain: true
    b.install(&mut slot); // slot: 2, b.old: 1     (A now buried)

    a.uninstall(&mut slot); // A.Free while buried: slot stays 2, a.in_chain stays true
    a.install(&mut slot); // A.Init: in_chain true -> re-arm only, no push

    assert_eq!(
        a.old, STOCK,
        "buried A's forward target must stay STOCK, not B"
    );
    let order = traverse(slot, &[a, b]);
    assert_eq!(
        order,
        vec![2, 1],
        "B -> A -> STOCK, A still buried once, no cycle"
    );
}

#[test]
#[should_panic(expected = "cycle detected")]
fn reload_while_buried_old_behavior_cycles() {
    // Pins the pre-fix bug: re-pushing A while buried under B (the old
    // unconditional install_inner, modelled by push()) forms a cycle that
    // traverse's guard detects. Guards against a regression that drops the
    // in_chain check.
    let mut slot = STOCK;
    let mut a = Plugin::new(1);
    let mut b = Plugin::new(2);
    a.push(&mut slot);
    b.push(&mut slot);
    a.uninstall(&mut slot); // buried no-op
    a.push(&mut slot); // re-push while buried: a.old=2 while b.old=1 -> cycle
    let _ = traverse(slot, &[a, b]);
}

#[test]
fn reload_while_head_repushes() {
    // A is the head (no plugin above). A.Free splices it out (in_chain false,
    // slot back to STOCK); A.Init pushes again. No false lost hook.
    let mut slot = STOCK;
    let mut a = Plugin::new(1);

    a.install(&mut slot); // slot: 1, a.old: STOCK, in_chain: true
    a.uninstall(&mut slot); // head splice: slot: STOCK, in_chain: false
    assert!(!a.in_chain, "head uninstall clears in_chain");
    a.install(&mut slot); // in_chain false -> push

    assert!(a.in_chain, "re-installed head is back in the chain");
    assert_eq!(a.old, STOCK);
    assert_eq!(traverse(slot, &[a]), vec![1], "A -> STOCK");
}

#[test]
fn drop_while_buried_keeps_in_chain() {
    // Buried uninstall must leave in_chain set, so a later reload re-arms in
    // place instead of re-pushing into a cycle.
    let mut slot = STOCK;
    let mut a = Plugin::new(1);
    let mut b = Plugin::new(2);

    a.install(&mut slot);
    b.install(&mut slot); // A buried under B
    a.uninstall(&mut slot); // buried no-op

    assert!(a.in_chain, "buried uninstall must not clear in_chain");
}

#[test]
fn orphan_wipe_then_reload_is_lost_until_reinstall() {
    // Accepted edge: a buried plugin is dropped, then a wipe happens while it is
    // orphaned (no reinstall fires), then it is reloaded. install() sees a stale
    // in_chain and re-arms without pushing, so the hook is silently missing --
    // never a cycle/crash -- until the next reinstall() heals it.
    let mut slot = STOCK;
    let mut a = Plugin::new(1);
    let mut b = Plugin::new(2);

    a.install(&mut slot);
    b.install(&mut slot); // A buried
    a.uninstall(&mut slot); // A.Free (buried): a.in_chain stays true

    wipe(&mut slot); // disconnect while A orphaned; slot: STOCK, flags stale

    a.install(&mut slot); // A.Init: stale in_chain true -> re-arm only, no push
    assert_eq!(
        slot, STOCK,
        "orphaned reload does not push -- A stays inactive"
    );
    assert!(a.in_chain, "stale in_chain belief remains set");

    a.reinstall(&mut slot); // next disconnect's Reset heals it
    assert!(
        traverse(slot, &[a]).contains(&1),
        "reinstall restores the orphaned hook"
    );
}

// ------------- FFI-touching tests (Windows + non-CI only) ------------------
//
// These link against the ClassiCube import library (only emitted on Windows by
// classicube-sys/build.rs) and exercise the install/uninstall/reinstall chain
// against the live Protocol.Handlers table. They are marked #[ignore] so that
// CI never attempts to run them; a developer can un-ignore from inside a
// running ClassiCube session on Windows.

#[cfg(all(windows, not(feature = "ci")))]
mod windows_ffi {
    use classicube_sys::OPCODE__OPCODE_MESSAGE;

    use super::*;

    /// `OPCODE_MESSAGE` as the `u8` opcode the generic hook takes; opcodes are
    /// fixed engine values indexing the 256-entry handler table, so the
    /// constant always fits `u8`. Used here purely as an example opcode to
    /// exercise the generic install/reinstall/uninstall path.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "OPCODE_MESSAGE is a fixed engine opcode well within u8 range"
    )]
    const OPCODE_MESSAGE: u8 = OPCODE__OPCODE_MESSAGE as u8;

    fn read_slot() -> Net_Handler {
        unsafe {
            (&raw const Protocol.Handlers)
                .cast::<Net_Handler>()
                .add(OPCODE_MESSAGE as usize)
                .read()
        }
    }

    fn write_slot(h: Net_Handler) {
        unsafe {
            (&raw mut Protocol.Handlers)
                .cast::<Net_Handler>()
                .add(OPCODE_MESSAGE as usize)
                .write(h);
        }
    }

    fn saved_old() -> Net_Handler {
        OLD.with(|a| a[OPCODE_MESSAGE as usize].get())
    }

    #[test]
    #[ignore]
    fn install_puts_trampoline_on_top() {
        let hook = ProtocolHook::install(OPCODE_MESSAGE, |_| false);
        assert!(hook.is_some());
        assert!(is_our_handler(OPCODE_MESSAGE, read_slot()));
        drop(hook);
    }

    #[test]
    #[ignore]
    fn drop_restores_prior_handler() {
        let before = read_slot();
        let hook = ProtocolHook::install(OPCODE_MESSAGE, |_| false);
        drop(hook);
        assert!(handlers_eq(before, read_slot()));
    }

    #[test]
    #[ignore]
    fn reinstall_after_wipe_restores_trampoline() {
        let hook = ProtocolHook::install(OPCODE_MESSAGE, |_| false).unwrap();
        // Simulate a ClassiCube reset by writing back the handler that was
        // saved into OLD on install (the pre-hook default).
        write_slot(saved_old());
        assert!(
            !is_our_handler(OPCODE_MESSAGE, read_slot()),
            "slot should appear wiped"
        );
        hook.reinstall();
        assert!(
            is_our_handler(OPCODE_MESSAGE, read_slot()),
            "reinstall should restore trampoline"
        );
        drop(hook);
    }

    #[test]
    #[ignore]
    fn reinstall_on_foreign_top_repushes() {
        // A foreign handler that clobbered the slot (overwriting us without
        // chaining) breaks our chain. reinstall() is order-independent and
        // self-healing: it re-pushes the trampoline on top, saving the foreign
        // handler as our new forward target. No loop here -- the foreign
        // handler does not chain back to us.
        let hook = ProtocolHook::install(OPCODE_MESSAGE, |_| false).unwrap();
        // Foreign handler clobbers the slot.
        write_slot(Some(dummy_a));
        hook.reinstall();
        // We are back on top, with dummy_a saved as our forward target.
        assert!(is_our_handler(OPCODE_MESSAGE, read_slot()));
        assert!(handlers_eq(saved_old(), Some(dummy_a)));
        // Drop restores OLD (dummy_a) since we are on top -- clean teardown.
        drop(hook);
    }
}
