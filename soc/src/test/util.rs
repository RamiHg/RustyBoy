use crate::util::*;

#[test]
fn test_timer() {
    let mut timer = timer(3);
    assert_eq!(timer.next(), Some(2));
    assert_eq!(timer.next(), Some(1));
    assert_eq!(timer.next(), Some(0));
    assert_eq!(timer.next(), None);
    assert_eq!(timer.next(), None);
}

#[test]
fn test_countdown_timer() {
    let mut timer = CountdownTimer::new(2, 3);
    assert_eq!(timer.next(), Some(None));
    assert_eq!(timer.next(), Some(None));
    assert_eq!(timer.next(), Some(Some(1)));
    assert_eq!(timer.next(), Some(None));
    assert_eq!(timer.next(), Some(None));
    assert_eq!(timer.next(), Some(Some(0)));
    assert_eq!(timer.next(), None);
    assert_eq!(timer.next(), None);
}

#[test]
fn test_iterate_bits() {
    assert_eq!(
        iterate_bits(0b1100_u8).collect::<Vec<bool>>(),
        [false, false, true, true, false, false, false, false]
    );
    assert_eq!(
        iterate_bits(0b0011_1100_0011_1100_u16).collect::<Vec<bool>>(),
        [false, false, true, true, true, true, false, false]
            .iter()
            .cycle()
            .take(16)
            .cloned()
            .collect::<Vec<bool>>()
    );
}
