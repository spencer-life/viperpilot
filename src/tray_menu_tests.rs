//! Hardware-free checks of actual Win32 menu objects and synthetic dispatch.

use super::*;
use windows::Win32::UI::WindowsAndMessaging::{
    GetMenuItemCount, GetMenuItemID, GetMenuState, GetMenuStringW, GetSubMenu, MF_BYPOSITION,
};

struct TestMenu(HMENU);
impl Drop for TestMenu {
    fn drop(&mut self) {
        let _ = unsafe { DestroyMenu(self.0) };
    }
}

#[derive(Debug)]
struct Item {
    id: u32,
    label: String,
    state: u32,
    children: Vec<Item>,
}

fn items(menu: HMENU) -> Vec<Item> {
    let count = unsafe { GetMenuItemCount(Some(menu)) };
    assert!((0..100).contains(&count));
    (0..count)
        .map(|position| {
            let mut label = [0u16; 4096];
            let length =
                unsafe { GetMenuStringW(menu, position as u32, Some(&mut label), MF_BYPOSITION) };
            let submenu = unsafe { GetSubMenu(menu, position) };
            Item {
                id: unsafe { GetMenuItemID(menu, position) },
                label: String::from_utf16_lossy(&label[..usize::try_from(length).unwrap()]),
                state: unsafe { GetMenuState(menu, position as u32, MF_BYPOSITION) },
                children: if submenu.0.is_null() {
                    vec![]
                } else {
                    items(submenu)
                },
            }
        })
        .collect()
}

fn find(items: &[Item], id: usize) -> Option<&Item> {
    items.iter().find_map(|item| {
        if item.id == id as u32 {
            Some(item)
        } else {
            find(&item.children, id)
        }
    })
}

fn snapshot(app: &TrayApp) -> Vec<Item> {
    let menu = TestMenu(app.build_menu().expect("create synthetic native menu"));
    items(menu.0)
}

#[test]
fn real_menu_preserves_default_and_reversed_pair_labels_and_markers() {
    for reversed in [false, true] {
        let app = preview_app(reversed, false, false).unwrap();
        let tree = snapshot(&app);
        assert!(tree[0].label.contains("PREVIEW"));
        assert!(
            find(&tree, MENU_QUICK_APPLY_FIRST)
                .unwrap()
                .label
                .contains(if reversed { "Gaming" } else { "Developer" })
        );
        assert!(
            find(&tree, MENU_QUICK_APPLY_SECOND)
                .unwrap()
                .label
                .contains(if reversed { "Developer" } else { "Gaming" })
        );
        let selected = MENU_QUICK_FIRST_BASE + usize::from(reversed);
        assert!(find(&tree, selected).unwrap().label.starts_with("✓ "));
        assert!(
            !find(&tree, MENU_QUICK_FIRST_BASE + usize::from(!reversed))
                .unwrap()
                .label
                .starts_with("✓ ")
        );
        assert_eq!(
            find(&tree, MENU_QUICK_FIRST_BASE + usize::from(!reversed))
                .unwrap()
                .state
                & MF_CHECKED.0,
            0
        );
    }
}

#[test]
fn unavailable_library_disables_quick_actions_but_retains_fixed_recovery() {
    let app = preview_app(false, true, false).unwrap();
    let tree = snapshot(&app);
    for id in [MENU_QUICK_APPLY_FIRST, MENU_QUICK_APPLY_SECOND] {
        let item = find(&tree, id).unwrap();
        assert!(item.label.contains("unavailable"));
        assert_ne!(item.state & MF_GRAYED.0, 0);
    }
    assert!(find(&tree, MENU_SAVED_APPLY_BASE).is_none());
    assert_eq!(find(&tree, MENU_DEVELOPER).unwrap().state & MF_GRAYED.0, 0);
}

#[test]
fn busy_disconnected_and_read_only_menus_disable_all_apply_routes() {
    for scenario in 0..3 {
        let mut app = preview_app(false, false, scenario == 2).unwrap();
        if scenario == 0 {
            app.busy = Some(BusyKind::Reading);
        }
        if scenario == 1 {
            app.connection = ConnectionStatus::Disconnected;
        }
        let tree = snapshot(&app);
        for id in [
            MENU_DEVELOPER,
            MENU_GAMING,
            MENU_QUICK_APPLY_FIRST,
            MENU_QUICK_APPLY_SECOND,
            MENU_SAVED_APPLY_BASE,
            MENU_SAVED_APPLY_BASE + 1,
        ] {
            assert_ne!(
                find(&tree, id).unwrap().state & MF_GRAYED.0,
                0,
                "route {id} scenario {scenario}"
            );
        }
        if scenario == 2 {
            assert!(find(&tree, MENU_STARTUP).is_none());
        }
    }
}

#[test]
fn synthetic_menu_commands_change_only_memory_and_preserve_fixed_recovery_semantics() {
    let mut app = preview_app(true, false, false).unwrap();
    // A live receiver catches accidental worker dispatch instead of hiding it
    // behind the deliberately disconnected preview channel.
    let (tx, rx) = mpsc::channel();
    app.command_tx = tx;
    assert!(app.handle_preview_menu_command(MENU_QUICK_APPLY_FIRST));
    assert_eq!(app.current, ProfileMatch::Gaming);
    assert!(app.handle_preview_menu_command(MENU_DEVELOPER));
    assert_eq!(app.current, ProfileMatch::Developer);
    assert!(app.handle_preview_menu_command(MENU_GAMING));
    assert_eq!(app.current, ProfileMatch::Gaming);
    assert!(app.handle_preview_menu_command(MENU_QUICK_APPLY_SECOND));
    assert_eq!(app.current, ProfileMatch::Developer);
    assert!(app.handle_preview_menu_command(MENU_STARTUP));
    assert!(app.config.start_with_windows);
    assert!(app.handle_preview_menu_command(MENU_DIAGNOSTICS));
    assert!(app.config.diagnostics_enabled);
    let tree = snapshot(&app);
    assert_ne!(find(&tree, MENU_STARTUP).unwrap().state & MF_CHECKED.0, 0);
    assert_ne!(
        find(&tree, MENU_DIAGNOSTICS).unwrap().state & MF_CHECKED.0,
        0
    );
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[test]
fn posted_commands_cannot_bypass_disabled_synthetic_apply_or_pilot_startup() {
    let mut app = preview_app(false, false, true).unwrap();
    let (tx, rx) = mpsc::channel();
    app.command_tx = tx;
    for id in [
        MENU_GAMING,
        MENU_QUICK_APPLY_SECOND,
        MENU_SAVED_APPLY_BASE + 1,
        MENU_STARTUP,
    ] {
        assert!(app.handle_preview_menu_command(id));
    }
    assert_eq!(app.current, ProfileMatch::Developer);
    assert!(!app.config.start_with_windows);
    assert!(!app.handle_preview_menu_command(9999));
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[test]
fn synthetic_pair_selection_and_unicode_names_stay_local() {
    let mut app = preview_app(false, false, false).unwrap();
    let (tx, rx) = mpsc::channel();
    app.command_tx = tx;
    let name = "開発 — Long saved profile for menu inspection";
    app.library
        .as_mut()
        .unwrap()
        .duplicate("developer", "unicode-work", name)
        .unwrap();
    let tree = snapshot(&app);
    assert!(
        find(&tree, MENU_SAVED_APPLY_BASE + 2)
            .unwrap()
            .label
            .contains(name)
    );
    assert!(app.handle_preview_menu_command(MENU_QUICK_FIRST_BASE + 2));
    assert_eq!(
        app.library.as_ref().unwrap().quick_switch()[0],
        "unicode-work"
    );
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[test]
fn keyboard_menu_origin_tracks_window_and_dpi_instead_of_cursor() {
    let rect = RECT {
        left: -1100,
        top: 40,
        right: -200,
        bottom: 600,
    };
    for (dpi, expected) in [
        (0, (-1076, 92)),
        (96, (-1076, 92)),
        (144, (-1064, 118)),
        (192, (-1052, 144)),
    ] {
        let point = keyboard_menu_anchor(rect, dpi);
        assert_eq!((point.x, point.y), expected);
        assert!(point.x > rect.left && point.x < rect.right);
        assert!(point.y > rect.top && point.y < rect.bottom);
    }
}

#[test]
fn informational_rows_are_disabled_and_have_no_synthetic_command() {
    let mut app = preview_app(false, false, false).unwrap();
    let tree = snapshot(&app);
    let rows: Vec<_> = tree.iter().filter(|item| item.id == 0).collect();
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|item| item.state & MF_GRAYED.0 != 0));
    assert!(!app.handle_preview_menu_command(0));
}

#[test]
fn keyboard_context_sentinel_accepts_zero_and_sign_extended_coordinates() {
    assert!(context_menu_from_keyboard(-1));
    assert!(context_menu_from_keyboard(
        isize::try_from(u32::MAX).unwrap()
    ));
    assert!(!context_menu_from_keyboard(0));
    assert!(!context_menu_from_keyboard(0xffff_0001));
    assert!(!context_menu_from_keyboard(0x0001_ffff));
}

#[test]
fn pair_swap_is_atomic_and_device_inert_in_read_only_preview() {
    let mut app = preview_app(false, false, true).unwrap();
    let (tx, rx) = mpsc::channel();
    app.command_tx = tx;
    let tree = snapshot(&app);
    assert_eq!(find(&tree, MENU_SWAP_PAIR).unwrap().state & MF_GRAYED.0, 0);
    assert!(app.handle_preview_menu_command(MENU_SWAP_PAIR));
    assert_eq!(
        app.library.as_ref().unwrap().quick_switch(),
        &["gaming".to_owned(), "developer".to_owned()]
    );
    assert_eq!(app.current, ProfileMatch::Developer);
    assert!(app.handle_preview_menu_command(MENU_SWAP_PAIR));
    assert_eq!(
        app.library.as_ref().unwrap().quick_switch(),
        &["developer".to_owned(), "gaming".to_owned()]
    );
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[test]
fn read_only_worker_swaps_only_local_library_and_preserves_baseline() {
    let session = crate::pilot::tests::fixture(false);
    let baseline = std::fs::read(
        session
            .paths()
            .baseline_path(&session.baseline().device.serial),
    )
    .unwrap();
    let result = handle_worker_command(WorkerCommand::SwapQuickSwitch, Some(&session)).unwrap();
    let WorkerEvent::LibraryUpdated(library) = result else {
        panic!("expected local library update")
    };
    assert_eq!(
        library.quick_switch(),
        &["gaming".to_owned(), "developer".to_owned()]
    );
    assert_eq!(load_profile_library(session.paths()).unwrap(), library);
    assert_eq!(
        std::fs::read(
            session
                .paths()
                .baseline_path(&session.baseline().device.serial)
        )
        .unwrap(),
        baseline
    );
    assert!(!session.paths().config.exists());
    assert!(!session.paths().reports.exists());
    std::fs::remove_dir_all(&session.paths().root).unwrap();
}

#[test]
fn shell_v4_and_legacy_activation_open_the_window_without_profile_actions() {
    for event in [NIN_SELECT, NIN_KEYSELECT, WM_LBUTTONUP, WM_LBUTTONDBLCLK] {
        assert!(tray_event_opens_window(event));
    }
    for event in [WM_CONTEXTMENU, WM_RBUTTONUP, WM_HOTKEY, WORKER_EVENT] {
        assert!(!tray_event_opens_window(event));
    }
}
