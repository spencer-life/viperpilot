//! Small platform-independent decisions used by the native tray.

#[must_use]
pub const fn notification_event_code(raw_lparam: isize) -> u16 {
    let bytes = raw_lparam.to_le_bytes();
    u16::from_le_bytes([bytes[0], bytes[1]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_icon_v4_event_ignores_the_high_word_icon_id() {
        let mouse_event = 0x0205_u32;
        let icon_id = 1_u32;
        let packed = isize::try_from((icon_id << 16) | mouse_event).expect("packed value fits");
        assert_eq!(u32::from(notification_event_code(packed)), mouse_event);
    }
}
