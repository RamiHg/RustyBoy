use crate::gpu::Color;
use crate::system::System;

use super::*;
use crate::test::*;

use num_traits::FromPrimitive as _;
use std::path::Path;

trait SpriteFn = Fn(usize, usize, &mut Color, SpriteBuilder);

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

fn composite_sprite(img_i: usize, img_j: usize, color: &mut Color, builder: SpriteBuilder) {
    let i = img_i as i32;
    let j = img_j as i32;
    let sprite = builder.sprite;
    let intersects_x = sprite.right() > i && sprite.left() <= i;
    // Subtract 8 from the bottom since we are not doing 8x16 sprites.
    let intersects_y = (sprite.bottom() - 8) > j && sprite.top() <= j;
    if intersects_x && intersects_y {
        *color = Color::Black;
    }
}

fn composite_image(
    image_fn: impl ImageFn,
    sprite_fn: impl SpriteFn,
    builder: SpriteBuilder,
) -> Box<impl ImageFn> {
    Box::new(move |i, j| {
        let mut color = image_fn(i, j);
        sprite_fn(i, j, &mut color, builder.clone());
        color
    })
}

#[test]
fn test_sprite_topleft() {
    let sprite = SpriteBuilder::with_pos(0, 0);
    ImageBuilder::new()
        .build_default_bg(Box::new(WHITE_BG_IMAGE))
        .golden_fn(composite_image(&WHITE_BG_IMAGE, composite_sprite, sprite))
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
            .golden_fn(composite_image(&WHITE_BG_IMAGE, composite_sprite, sprite))
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
            .golden_fn(composite_image(&WHITE_BG_IMAGE, composite_sprite, sprite))
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
                .golden_fn(composite_image(&WHITE_BG_IMAGE, composite_sprite, sprite))
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
            .golden_fn(composite_image(&WHITE_BG_IMAGE, composite_sprite, sprite))
            .add_sprite(sprite)
            .enable_sprites()
            .xscroll(xscroll)
            .run_and_assert_is_golden_fn(format!("sprite_xscroll{}", xscroll), |i, j| {
                (i + xscroll, j)
            });
    }
}

#[test]
fn test_sprite_yscroll() {
    for &yscroll in &[1, 8, 128, 250] {
        let sprite = SpriteBuilder::with_pos(0, 0);
        ImageBuilder::new()
            .build_default_bg(Box::new(WHITE_BG_IMAGE))
            .golden_fn(composite_image(&WHITE_BG_IMAGE, composite_sprite, sprite))
            .add_sprite(sprite)
            .enable_sprites()
            .yscroll(yscroll)
            .run_and_assert_is_golden_fn(format!("sprite_yscroll_{}", yscroll), |i, j| {
                (i, j + yscroll)
            });
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
                        .golden_fn(composite_image(&WHITE_BG_IMAGE, composite_sprite, sprite))
                        .add_sprite(sprite)
                        .enable_sprites()
                        .xscroll(xscroll)
                        .yscroll(yscroll)
                        .run_and_assert_is_golden_fn(
                            format!(
                                "sprite_move_and_scroll_{}_{}_scrollx{}_scrolly{}",
                                i, j, xscroll, yscroll
                            ),
                            |i, j| (i + xscroll, j + yscroll),
                        );
                }
            }
        }
    }
}
