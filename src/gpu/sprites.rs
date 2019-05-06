use arrayvec::ArrayVec;
/// There are 40 sprites total. Each sprite is 32bits. Therefore, OAM is 160 bytes of memory
/// (0xA0). When drawing, only the first 10 visible sprites (ordered by their location in RAM)
/// are drawn.
/// If two sprites on the same position, the sprite with the lower number wins.
use bitfield::bitfield;

pub struct Oam([u8; 160]);

impl Oam {
    fn new() -> Oam { Oam([0; 160]) }

    fn enumerate_entries(&self) -> impl Iterator<Item = (usize, SpriteEntry)> + '_ {
        self.0.chunks(4).map(SpriteEntry::from_slice).enumerate()
    }
}

bitfield! {
    pub struct SpriteEntry(u32);
    impl Debug;
    u8;
    pub pos_x, set_pos_x: 7, 0;
    pub pos_y, set_pos_y: 15, 8;
    pub tile_index, _: 23, 16;
    pub palette, _: 28;//, 28;
    pub flip_x, _: 29;
    pub flip_y, _: 30;
    /// If 1, will draw on top of non-zero background pixels. Otherwise, will always draw on top.
    /// (except of course translucent pixels).
    pub priority, _: 31, 31;
}

impl Clone for SpriteEntry {
    fn clone(&self) -> Self { *self }
}
impl Copy for SpriteEntry {}

impl SpriteEntry {
    pub fn from_slice(slice: &[u8]) -> SpriteEntry {
        SpriteEntry(
            (slice[0] as u32)
                | ((slice[1] as u32) << 8)
                | ((slice[2] as u32) << 16)
                | ((slice[3] as u32) << 24),
        )
    }

    pub fn is_visible_on_line(&self, line: i32) -> bool {
        self.top() <= line && self.bottom() > line
    }

    pub fn top(&self) -> i32 { self.pos_y() as i32 - 16 }
    pub fn bottom(&self) -> i32 { self.pos_y() as i32 }

    pub fn left(&self) -> i32 { self.pos_x() as i32 - 8 }
    pub fn right(&self) -> i32 { self.pos_x() as i32 }
}

pub fn find_visible_sprites(oam: &[u8], line: i32) -> ArrayVec<[u8; 10]> {
    debug_assert_eq!(oam.len(), 160);
    let mut sprites = ArrayVec::new();
    for (index, chunk) in oam.chunks(4).enumerate() {
        debug_assert_lt!(index, 40);
        let sprite = SpriteEntry::from_slice(chunk);
        if sprite.is_visible_on_line(line) {
            sprites.push(index as u8);
        }
        if sprites.len() >= 10 {
            break;
        }
    }
    sprites
}

pub fn get_visible_sprite(x: i32, visible_sprites: &[u8], oam: &[u8]) -> Option<u8> {
    let sprite_location = |x| &oam[(x * 4) as usize..];
    for sprite_index in visible_sprites {
        let sprite = SpriteEntry::from_slice(sprite_location(sprite_index));
        if sprite.right() > x && sprite.left() <= x {
            return Some(*sprite_index);
        }
    }
    return None;
}

pub fn num_visible_pixels_in_tile(x: i32, sprite: &SpriteEntry) -> i32 {
    debug_assert_gt!(sprite.right(), x);
    sprite.right() - x
}