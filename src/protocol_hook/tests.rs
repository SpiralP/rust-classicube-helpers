use super::*;

// ------------- is_normal_message ------------------

#[test]
fn is_normal_message_zero_is_true() {
    assert!(is_normal_message(0));
}

#[test]
fn is_normal_message_nonzero_is_false() {
    for byte in 1u8..=5 {
        assert!(!is_normal_message(byte), "expected false for byte {byte}");
    }
}

// ------------- handlers_eq (pure, using local dummy fns) ------------------

unsafe extern "C" fn dummy_a(_: *mut u8) {}
unsafe extern "C" fn dummy_b(_: *mut u8) {}

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

// ------------- FFI-touching tests (Windows + non-CI only) ------------------
//
// These link against the ClassiCube import library (only emitted on Windows by
// classicube-sys/build.rs) and exercise the install/uninstall/reinstall chain
// against the live Protocol.Handlers table. They are marked #[ignore] so that
// CI never attempts to run them; a developer can un-ignore from inside a
// running ClassiCube session on Windows.

#[cfg(all(windows, not(feature = "ci")))]
mod windows_ffi {
    use super::*;

    fn read_slot() -> Net_Handler {
        unsafe {
            (&raw const Protocol.Handlers)
                .cast::<Net_Handler>()
                .add(IDX)
                .read()
        }
    }

    fn write_slot(h: Net_Handler) {
        unsafe {
            (&raw mut Protocol.Handlers)
                .cast::<Net_Handler>()
                .add(IDX)
                .write(h);
        }
    }

    #[test]
    #[ignore]
    fn install_puts_trampoline_on_top() {
        let hook = ProtocolMessageHook::install(|_| false);
        assert!(hook.is_some());
        assert!(is_our_handler(read_slot()));
        drop(hook);
    }

    #[test]
    #[ignore]
    fn drop_restores_prior_handler() {
        let before = read_slot();
        let hook = ProtocolMessageHook::install(|_| false);
        drop(hook);
        assert!(handlers_eq(before, read_slot()));
    }

    #[test]
    #[ignore]
    fn reinstall_after_wipe_restores_trampoline() {
        let hook = ProtocolMessageHook::install(|_| false).unwrap();
        // Simulate a ClassiCube reset by writing back the handler that was
        // saved into OLD on install (the pre-hook default).
        let old_saved = OLD.with(Cell::get);
        write_slot(old_saved);
        assert!(!is_our_handler(read_slot()), "slot should appear wiped");
        hook.reinstall();
        assert!(
            is_our_handler(read_slot()),
            "reinstall should restore trampoline"
        );
        drop(hook);
    }

    #[test]
    #[ignore]
    fn install_on_foreign_top_bails() {
        // Simulate a foreign plugin that installed after us by pushing a dummy
        // handler on top, then verify that reinstall does not re-push us
        // (which would loop via the dummy -> us -> dummy chain).
        let hook = ProtocolMessageHook::install(|_| false).unwrap();
        // Stack a foreign handler on top.
        let our_slot = read_slot();
        write_slot(Some(dummy_a));
        // reinstall should bail because current (dummy_a) != OLD.
        hook.reinstall();
        // Slot should still be dummy_a, not our trampoline.
        assert!(handlers_eq(read_slot(), Some(dummy_a)));
        // Restore so drop can clean up correctly (it will no-op since we're
        // not on top, then clear CALLBACK).
        write_slot(our_slot);
        drop(hook);
    }
}
