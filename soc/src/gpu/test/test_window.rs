use super::*;
use crate::test::*;

fn simple_checkerboard(mut i: usize, mut j: usize) -> Color {
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
}

fn composite_window(image_fn: impl ImageFn, wx: usize, wy: usize) -> Box<impl ImageFn> {
    Box::new(move |i, j| {
        let mut color = image_fn(i, j);
        let left = wx as i32 - 7;
        if left >= 0 && i as i32 >= left && j >= wy {
            color = image_fn((i as i32 - left) as usize, j - wy);
        }
        color
    })
}

#[test]
fn test_simple_wx() {
    for &wx in &[7, 8, 120] {
        ImageBuilder::new()
            .build_default_bg(Box::new(simple_checkerboard))
            .golden_fn(composite_window(simple_checkerboard, wx, 0))
            .wx(wx)
            .enable_window()
            .run_and_assert_is_golden_fn(format!("simple_wx_{}", wx), &IDENTITY_TRANSFORM);
    }
}

#[test]
fn test_simple_wy() {
    for &wy in &[7, 8, 120] {
        ImageBuilder::new()
            .build_default_bg(Box::new(simple_checkerboard))
            .golden_fn(composite_window(simple_checkerboard, 7, wy))
            .wx(7)
            .wy(wy)
            .enable_window()
            .run_and_assert_is_golden_fn(format!("simple_wy_{}", wy), &IDENTITY_TRANSFORM);
    }
}
