//! 2026-11-02: process-only supervised hotkey lifecycle. Registration is injected
//! so failure/ownership checks never need Windows input or a physical device.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Disabled,
    AwaitingValidation,
    Active,
    Stopped,
}

#[derive(Debug)]
pub(crate) struct PilotHotkey {
    phase: Phase,
    validated_refresh: bool,
    owned: bool,
}

impl PilotHotkey {
    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            phase: if enabled {
                Phase::AwaitingValidation
            } else {
                Phase::Disabled
            },
            validated_refresh: false,
            owned: false,
        }
    }

    pub(crate) fn refresh_validated(&mut self) {
        self.validated_refresh = true;
    }

    pub(crate) fn arm(
        &mut self,
        pair_available: bool,
        register: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        if self.phase != Phase::AwaitingValidation || !self.validated_refresh || !pair_available {
            return Ok(());
        }
        // Mark stopped before attempting registration: a conflict is terminal,
        // does not confer ownership and must never be retried automatically.
        self.phase = Phase::Stopped;
        register()?;
        self.owned = true;
        self.phase = Phase::Active;
        Ok(())
    }

    pub(crate) fn stop(&mut self, unregister: impl FnOnce() -> bool) {
        if self.phase != Phase::Disabled {
            self.phase = Phase::Stopped;
        }
        // A failed release remains ours. Dispatch is stopped regardless; normal
        // exit cleanup may release this same owned binding before window teardown.
        if self.owned && unregister() {
            self.owned = false;
        }
    }

    pub(crate) fn dispatch(
        &self,
        writes_enabled: bool,
        initialized: bool,
        connected: bool,
        busy: bool,
        toggle: impl FnOnce(),
    ) -> bool {
        if !self.can_dispatch(writes_enabled, initialized, connected, busy) {
            return false;
        }
        toggle();
        true
    }

    pub(crate) fn can_dispatch(
        &self,
        writes_enabled: bool,
        initialized: bool,
        connected: bool,
        busy: bool,
    ) -> bool {
        self.phase == Phase::Active
            && self.owned
            && writes_enabled
            && initialized
            && connected
            && !busy
    }

    pub(crate) fn status(&self) -> &'static str {
        match self.phase {
            Phase::Disabled => "Supervised profile pilot · hotkey disabled",
            Phase::AwaitingValidation => "Ctrl+Alt+P awaiting validation",
            Phase::Active => "Ctrl+Alt+P active · supervised pilot",
            Phase::Stopped => "Ctrl+Alt+P stopped · reviewed relaunch required",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn no_early_arm_and_success_registers_once_in_either_dependency_order() {
        for library_first in [false, true] {
            let registrations = Cell::new(0);
            let mut hotkey = PilotHotkey::new(true);
            let register = || {
                registrations.set(registrations.get() + 1);
                Ok(())
            };
            hotkey.arm(library_first, register).unwrap();
            assert_eq!(registrations.get(), 0);
            assert!(!hotkey.dispatch(true, true, true, false, || panic!("early toggle")));
            hotkey.refresh_validated();
            hotkey.arm(library_first, register).unwrap();
            assert_eq!(registrations.get(), usize::from(library_first));
            hotkey.arm(true, register).unwrap();
            hotkey.refresh_validated();
            hotkey.arm(true, register).unwrap();
            assert_eq!(registrations.get(), 1);
            let toggles = Cell::new(0);
            assert!(hotkey.dispatch(true, true, true, false, || toggles.set(toggles.get() + 1)));
            assert_eq!(toggles.get(), 1);
        }
    }

    #[test]
    fn disabled_busy_disconnected_uninitialized_and_read_only_cannot_dispatch() {
        let mut disabled = PilotHotkey::new(false);
        disabled.refresh_validated();
        disabled
            .arm(true, || panic!("disabled registration"))
            .unwrap();
        assert!(disabled.status().contains("disabled"));
        disabled.stop(|| panic!("unowned cleanup"));
        assert!(!disabled.dispatch(true, true, true, false, || panic!("disabled toggle")));
        let mut hotkey = PilotHotkey::new(true);
        hotkey.refresh_validated();
        hotkey.arm(true, || Ok(())).unwrap();
        for (writes, initialized, connected, busy) in [
            (false, true, true, false),
            (true, false, true, false),
            (true, true, false, false),
            (true, true, true, true),
        ] {
            assert!(
                !hotkey.dispatch(writes, initialized, connected, busy, || panic!(
                    "blocked toggle/setter"
                ))
            );
        }
    }

    #[test]
    fn conflict_has_zero_toggle_or_setters_and_never_unregisters_or_retries() {
        let mut hotkey = PilotHotkey::new(true);
        hotkey.refresh_validated();
        assert!(
            hotkey
                .arm(true, || Err("synthetic conflict".to_owned()))
                .is_err()
        );
        assert!(!hotkey.dispatch(true, true, true, false, || panic!("conflict toggle/setter")));
        hotkey.arm(true, || panic!("retry")).unwrap();
        hotkey.stop(|| panic!("must not unregister somebody else's binding"));
    }

    #[test]
    fn stopped_never_rearms_and_deferred_input_rechecks_ownership() {
        let releases = Cell::new(0);
        let mut hotkey = PilotHotkey::new(true);
        hotkey.refresh_validated();
        hotkey.arm(true, || Ok(())).unwrap();
        // Simulate input queued while active, then a safety failure before drain.
        hotkey.stop(|| {
            releases.set(releases.get() + 1);
            true
        });
        assert!(!hotkey.dispatch(true, true, true, false, || panic!(
            "stale deferred toggle/setter"
        )));
        hotkey.refresh_validated();
        hotkey.arm(true, || panic!("automatic rearm")).unwrap();
        hotkey.stop(|| {
            releases.set(releases.get() + 1);
            true
        });
        assert_eq!(releases.get(), 1);
        assert!(hotkey.status().contains("reviewed relaunch"));
    }

    #[test]
    fn failed_release_keeps_ownership_until_cleanup_succeeds() {
        let mut hotkey = PilotHotkey::new(true);
        hotkey.refresh_validated();
        hotkey.arm(true, || Ok(())).unwrap();
        hotkey.stop(|| false);
        assert!(hotkey.owned);
        assert!(!hotkey.dispatch(true, true, true, false, || panic!(
            "toggle after failed release"
        )));
        hotkey.stop(|| true);
        assert!(!hotkey.owned);
        hotkey.stop(|| panic!("already released"));
    }

    #[test]
    fn invalid_live_identity_or_baseline_and_refresh_failure_never_register() {
        let session = crate::pilot::tests::fixture(true);
        let mut wrong = session.baseline().clone();
        wrong.device.serial.push('X');
        for validation in [
            session.validate_live(&wrong),
            Err("synthetic refresh failure".to_owned()),
        ] {
            let mut hotkey = PilotHotkey::new(true);
            assert!(validation.is_err());
            hotkey.stop(|| panic!("unowned cleanup"));
            hotkey.refresh_validated();
            hotkey
                .arm(true, || panic!("invalid refresh registration"))
                .unwrap();
        }
        let baseline_path = session
            .paths()
            .baseline_path(&session.baseline().device.serial);
        let mut bytes = std::fs::read(&baseline_path).unwrap();
        bytes.push(b'\n');
        std::fs::write(baseline_path, bytes).unwrap();
        assert!(session.validate_live(session.baseline()).is_err());
        let mut hotkey = PilotHotkey::new(true);
        hotkey.stop(|| panic!("unowned cleanup"));
        hotkey
            .arm(true, || panic!("changed baseline registration"))
            .unwrap();
        std::fs::remove_dir_all(&session.paths().root).unwrap();
    }
}
