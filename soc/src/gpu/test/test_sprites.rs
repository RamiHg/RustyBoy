use crate::gpu::Color;

use super::*;

fn composite_sprite(img_i: usize, img_j: usize, color: &mut Color, builder: SpriteBuilder) {
    let i = img_i as i32;
    let j = img_j as i32;
    let sprite = builder.sprite;
    let intersects_x = sprite.right() > i && sprite.left() <= i;
    // Subtract 8 from the bottom since we are not doing 8x16 sprites.
    let intersects_y = sprite.bottom(false) > j && sprite.top() <= j;
    if intersects_x && intersects_y {
        *color = Color::Black;
    }
}

fn composite_image(
    _image_fn: &'static impl Fn(usize, usize) -> Color,
    builder: SpriteBuilder,
) -> ImageFn {
    Box::new(move |i, j| {
        let mut color = WHITE_BG_IMAGE(i, j);
        composite_sprite(i, j, &mut color, builder);
        color
    })
}

#[test]
fn test_sprite_topleft() {
    let sprite = SpriteBuilder::with_pos(0, 0);
    ImageBuilder::new()
        .build_default_bg(Box::new(WHITE_BG_IMAGE))
        .golden_fn(composite_image(&WHITE_BG_IMAGE, sprite))
        .add_sprite(sprite)
        .enable_sprites()
        .run_and_assert_is_golden_fn("sprite_topleft", &IDENTITY_TRANSFORM);
}

#[test]
fn test_sprite_horizontal_move() {
    for &i in &[1, 7, 8, 152, 155, 159] {
        let sprite = SpriteBuilder::with_pos(i, 0);
        ImageBuilder::new()
            .build_default_bg(Box::new(WHITE_BG_IMAGE))
            .golden_fn(composite_image(&WHITE_BG_IMAGE, sprite))
            .add_sprite(sprite)
            .enable_sprites()
            .run_and_assert_is_golden_fn(
                format!("sprite_horizontal_move_{}", i),
                &IDENTITY_TRANSFORM,
            );
    }
}

#[test]
fn test_sprite_vertical_move() {
    for &j in &[1, 7, 8, 135, 139, 140] {
        let sprite = SpriteBuilder::with_pos(0, j);
        ImageBuilder::new()
            .build_default_bg(Box::new(WHITE_BG_IMAGE))
            .golden_fn(composite_image(&WHITE_BG_IMAGE, sprite))
            .add_sprite(sprite)
            .enable_sprites()
            .run_and_assert_is_golden_fn(
                format!("sprite_vertical_move_{}", j),
                &IDENTITY_TRANSFORM,
            );
    }
}

#[test]
fn test_sprite_move() {
    for &i in &[1, 8, 155, 159] {
        for &j in &[1, 7, 139] {
            let sprite = SpriteBuilder::with_pos(i, j);
            ImageBuilder::new()
                .build_default_bg(Box::new(WHITE_BG_IMAGE))
                .golden_fn(composite_image(&WHITE_BG_IMAGE, sprite))
                .add_sprite(sprite)
                .enable_sprites()
                .run_and_assert_is_golden_fn(
                    format!("sprite_move_{}_{}", i, j),
                    &IDENTITY_TRANSFORM,
                );
        }
    }
}

#[test]
fn test_sprite_xscroll() {
    for &xscroll in &[1, 8, 128, 250] {
        let sprite = SpriteBuilder::with_pos(0, 0);
        ImageBuilder::new()
            .build_default_bg(Box::new(WHITE_BG_IMAGE))
            .golden_fn(composite_image(&WHITE_BG_IMAGE, sprite))
            .add_sprite(sprite)
            .enable_sprites()
            .xscroll(xscroll)
            .run_and_assert_is_golden_fn(format!("sprite_xscroll{}", xscroll), &|i, j| (i, j));
    }
}

#[test]
fn test_sprite_yscroll() {
    for &yscroll in &[1, 8, 128, 250] {
        let sprite = SpriteBuilder::with_pos(0, 0);
        ImageBuilder::new()
            .build_default_bg(Box::new(WHITE_BG_IMAGE))
            .golden_fn(composite_image(&WHITE_BG_IMAGE, sprite))
            .add_sprite(sprite)
            .enable_sprites()
            .yscroll(yscroll)
            .run_and_assert_is_golden_fn(format!("sprite_yscroll_{}", yscroll), &|i, j| (i, j));
    }
}

#[test]
fn test_sprite_move_and_scroll() {
    for &i in &[1, 159] {
        for &j in &[1, 7, 139] {
            for &xscroll in &[1, 128, 250] {
                for &yscroll in &[1, 8, 250] {
                    let sprite = SpriteBuilder::with_pos(i, j);
                    ImageBuilder::new()
                        .build_default_bg(Box::new(WHITE_BG_IMAGE))
                        .golden_fn(composite_image(&WHITE_BG_IMAGE, sprite))
                        .add_sprite(sprite)
                        .enable_sprites()
                        .xscroll(xscroll)
                        .yscroll(yscroll)
                        .run_and_assert_is_golden_fn(
                            format!(
                                "sprite_move_and_scroll_{}_{}_scrollx{}_scrolly{}",
                                i, j, xscroll, yscroll
                            ),
                            &|i, j| (i, j),
                        );
                }
            }
        }
    }
}

/// Tests the situation where 8 sprites are in the same exact position, but they are transparent in
/// different places!
///
/// Layout:
///
/// 0:  _ _ _ _ _ _ _ X
/// 1:  X _ _ _ _ _ _ _
/// 2:  _ X _ _ _ _ _ _
/// 3:  _ _ _ X _ _ _ _
/// 4:  _ _ X _ _ _ _ _
/// 5:  _ _ _ _ _ _ X _
/// 6:  _ _ _ _ _ _ _ X
/// 7:  X _ _ _ _ _ _ _
///
/// Expected:
///     1 2 4 3 _ _ 5 0
///  Colors organized to remove overlap:
///     3 1 2 3 _ _ 2 1
fn test_sprite_overlapping_same_pixel_at(x: i32, y: i32) {
    let golden_fn = move |i, j| {
        if j == 0 {
            match i as i32 - x {
                0 => Black,
                1 => LightGray,
                2 => DarkGray,
                3 => Black,
                6 => DarkGray,
                7 => LightGray,
                _ => White,
            }
        } else {
            White
        }
    };

    use Color::*;
    let sprites = [
        SpriteBuilder::with_pos(x, y).color(LightGray).mask_row(0, [0, 0, 0, 0, 0, 0, 0, 1]),
        SpriteBuilder::with_pos(x, y).color(White).mask_row(0, [1, 0, 0, 0, 0, 0, 0, 0]),
        SpriteBuilder::with_pos(x, y).color(LightGray).mask_row(0, [0, 1, 0, 0, 0, 0, 0, 0]),
        SpriteBuilder::with_pos(x, y).color(Black).mask_row(0, [0, 0, 0, 1, 0, 0, 0, 0]),
        SpriteBuilder::with_pos(x, y).color(DarkGray).mask_row(0, [0, 0, 1, 0, 0, 0, 0, 0]),
        SpriteBuilder::with_pos(x, y).color(DarkGray).mask_row(0, [0, 0, 0, 0, 0, 0, 1, 0]),
        SpriteBuilder::with_pos(x, y).mask_row(0, [0, 0, 0, 0, 0, 0, 0, 1]),
        SpriteBuilder::with_pos(x, y).color(Black).mask_row(0, [1, 0, 0, 0, 0, 0, 0, 0]),
    ];

    ImageBuilder::new()
        .golden_fn(Box::new(golden_fn))
        .add_sprites(&sprites)
        .enable_sprites()
        .run_and_assert_is_golden_fn(
            format!("overlapping_same_pixel_{}_{}", x, y),
            &IDENTITY_TRANSFORM,
        );
}

#[test]
fn test_sprite_overlapping_same_pixel() {
    test_sprite_overlapping_same_pixel_at(0, 0);
}
