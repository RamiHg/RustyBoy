use crate::gpu::Color;

use super::*;

use num_traits::FromPrimitive as _;

fn simple_checkerboard() -> ImageFn {
    Box::new(|mut i, mut j| {
        i /= 8;
        j /= 8;

        let mut color = (((i + j) % 2) == 0) as usize;
        if i == j {
            color += 1;
        }
        if color > 0 && (j % 2) == 0 {
            color += 1;
        }
        Color::from_usize(color).unwrap()
    })
}

/// Tests an empty background with a single square sprite on the top-left corner.
#[test]
fn test_simple_checkerboard() {
    let system = ImageBuilder::new()
        .build_default_bg(simple_checkerboard())
        .run_and_assert_is_golden_fn("simple_checkerboard", &IDENTITY_TRANSFORM);
}

#[test]
fn test_large_xscroll_checkerboard() {
    // Test some edge-cases rather than all possible transforms since it's too slow.
    for xscroll in &[0_usize, 8, 16, 21, 32] {
        let system = ImageBuilder::new()
            .build_default_bg(simple_checkerboard())
            .xscroll(xscroll * 8)
            .run_and_assert_is_golden_fn(format!("large_{}_xscroll", xscroll), &|i, j| {
                (i + xscroll * 8, j)
            });
    }
}

#[test]
fn test_fine_xscroll_checkerboard() {
    // Test some edge-cases rather than all possible transforms since it's too slow.
    for &xscroll in &[0_usize, 1, 2, 3, 8, 9, 10, 128, 129, 251, 252, 255] {
        let system = ImageBuilder::new()
            .build_default_bg(simple_checkerboard())
            .xscroll(xscroll)
            .run_and_assert_is_golden_fn(format!("fine_{}_xscroll", xscroll), &|i, j| {
                (i + xscroll, j)
            });
    }
}

#[test]
fn test_fine_yscroll_checkerboard() {
    // Test some edge-cases rather than all possible transforms since it's too slow.
    for &yscroll in &[0_usize, 1, 2, 7, 8, 9, 10, 128, 129, 251, 252, 255] {
        let system = ImageBuilder::new()
            .build_default_bg(simple_checkerboard())
            .yscroll(yscroll)
            .run_and_assert_is_golden_fn(format!("fine_{}_yscroll", yscroll), &|i, j| {
                (i, j + yscroll)
            });
    }
}

#[test]
fn test_fine_scroll_checkerboard() {
    // Test some edge-cases rather than all possible transforms since it's too slow.
    for &yscroll in &[0_usize, 1, 7, 128, 254, 255] {
        for &xscroll in &[0_usize, 1, 7, 128, 254, 255] {
            let system = ImageBuilder::new()
                .build_default_bg(simple_checkerboard())
                .xscroll(xscroll)
                .yscroll(yscroll)
                .run_and_assert_is_golden_fn(format!("fine_{}_scroll", yscroll), &|i, j| {
                    (i + xscroll, j + yscroll)
                });
        }
    }
}
