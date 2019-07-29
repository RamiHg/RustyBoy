; This file is part of Mooneye GB.
; Copyright (C) 2014-2016 Joonas Javanainen <joonas.javanainen@gmail.com>
;
; Mooneye GB is free software: you can redistribute it and/or modify
; it under the terms of the GNU General Public License as published by
; the Free Software Foundation, either version 3 of the License, or
; (at your option) any later version.
;
; Mooneye GB is distributed in the hope that it will be useful,
; but WITHOUT ANY WARRANTY; without even the implied warranty of
; MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
; GNU General Public License for more details.
;
; You should have received a copy of the GNU General Public License
; along with Mooneye GB.  If not, see <http://www.gnu.org/licenses/>.

; Tests how long does it take to get from STAT=mode2 interrupt to mode0
; when SCX is set to 3.
; Includes sprites in various configurations

; Verified results:
;   pass: DMG, MGB, SGB, SGB2, CGB, AGB, AGS
;   fail: -

.incdir "../../common"
.include "common.s"

.macro clear_interrupts
  xor a
  ldh (<IF), a
.endm

.macro wait_mode ARGS mode
- ldh a, (<STAT)
  and $03
  cp mode
  jr nz, -
.endm

  di
  wait_vblank
  disable_lcd
  call reset_screen
  call print_load_font

  enable_lcd
  ld a, INTR_STAT
  ldh (<IE), a

.macro testcase
  ld a, \@
  ld (testcase_id), a
  ld hl, _testcase_data_\@
  ld d, 41 + \1
  ld e, 40 + \1
  call run_testcase
  jr _testcase_end_\@
_testcase_data_\@:
  .shift
  .db NARGS
  .repeat NARGS
    .db \1
    .shift
  .endr
_testcase_end_\@:
.endm

  ; extra \      / x coordinates for sprites
  ; cycles \     / (varargs)
  ;        |     |
  ; 1-N sprites at X=0/N
  ; test #00
  testcase 3,    0
  testcase 2,    1
  testcase 2,    2
  testcase 2,    3
  ; test #04
  testcase 2,    4
  testcase 3,    5
  testcase 3,    6
  testcase 3,    7
  ; test #08
  testcase 2,    8
  testcase 2,    9
  testcase 2,    10
  testcase 2,    11
  ; test #0C
  testcase 2,    12
  testcase 3,    13
  testcase 3,    14
  testcase 3,    15
  ; test #10
  testcase 2,    16
  testcase 2,    17
  testcase 2,    160
  testcase 2,    161
  ; test #14
  testcase 2,    162
  testcase 2,    163
  testcase 2,    164
  testcase 3,    165
  ; test #18
  testcase 3,    166
  testcase 3,    167

  ; 1-N sprites at X=0
  ; test #1A
  testcase 5,    0,  0
  ; 2 sprites 8 bytes apart starting from X0=N
  testcase 5,    0,  8
  ; test #1C
  testcase 4,    1,  9
  testcase 3,    2,  10
  testcase 3,    3,  11
  testcase 3,    4,  12
  ; test #20
  testcase 6,    5,  13
  testcase 5,    6,  14
  testcase 5,    7,  15
  testcase 4,    8,  16
  ; test #24
  testcase 4,    9,  17
  testcase 3,    10, 18
  testcase 3,    11, 19
  testcase 3,    12, 20
  ; test #28
  testcase 6,    13, 21
  testcase 5,    14, 22
  testcase 5,    15, 23
  testcase 4,    16, 24

  ; 1-N sprites at X=0
  ; test #2C
  testcase 6,    0,  0,  0
  ; 1-N sprites at X=0
  testcase 8,    0,  0,  0,  0
  ; 1-N sprites at X=0
  testcase 9,    0,  0,  0,  0,  0
  ; 1-N sprites at X=0
  testcase 11,   0,  0,  0,  0,  0,  0
  ; 1-N sprites at X=0
  ; test #30
  testcase 12,   0,  0,  0,  0,  0,  0,  0
  ; 1-N sprites at X=0
  testcase 14,   0,  0,  0,  0,  0,  0,  0,  0
  ; 1-N sprites at X=0
  testcase 15,   0,  0,  0,  0,  0,  0,  0,  0,  0
  ; 1-N sprites at X=0
  testcase 17,   0,  0,  0,  0,  0,  0,  0,  0,  0,  0
  ; ==> sprite count affects cycles

  ; 10 sprites at X=N
  ; test #34
  testcase 16,   1,  1,  1,  1,  1,  1,  1,  1,  1,  1
  testcase 15,   2,  2,  2,  2,  2,  2,  2,  2,  2,  2
  testcase 15,   3,  3,  3,  3,  3,  3,  3,  3,  3,  3
  testcase 15,   4,  4,  4,  4,  4,  4,  4,  4,  4,  4
  ; test #38
  testcase 17,   5,  5,  5,  5,  5,  5,  5,  5,  5,  5
  testcase 16,   6,  6,  6,  6,  6,  6,  6,  6,  6,  6
  testcase 16,   7,  7,  7,  7,  7,  7,  7,  7,  7,  7
  testcase 16,   8,  8,  8,  8,  8,  8,  8,  8,  8,  8
  ; test #3C
  testcase 16,   9,  9,  9,  9,  9,  9,  9,  9,  9,  9
  testcase 15,   10, 10, 10, 10, 10, 10, 10, 10, 10, 10
  testcase 15,   11, 11, 11, 11, 11, 11, 11, 11, 11, 11
  testcase 15,   12, 12, 12, 12, 12, 12, 12, 12, 12, 12
  ; test #40
  testcase 17,   13, 13, 13, 13, 13, 13, 13, 13, 13, 13
  testcase 16,   14, 14, 14, 14, 14, 14, 14, 14, 14, 14
  testcase 16,   15, 15, 15, 15, 15, 15, 15, 15, 15, 15
  testcase 16,   16, 16, 16, 16, 16, 16, 16, 16, 16, 16
  ; test #44
  testcase 16,   17, 17, 17, 17, 17, 17, 17, 17, 17, 17
  testcase 16,   32, 32, 32, 32, 32, 32, 32, 32, 32, 32
  testcase 16,   33, 33, 33, 33, 33, 33, 33, 33, 33, 33
  testcase 16,   160,160,160,160,160,160,160,160,160,160
  ; test #48
  testcase 16,   161,161,161,161,161,161,161,161,161,161
  testcase 15,   162,162,162,162,162,162,162,162,162,162
  testcase 16,   167,167,167,167,167,167,167,167,167,167
  testcase 16,   167,167,167,167,167,167,167,167,167,167
  ; test #4C
  testcase 0,    168,168,168,168,168,168,168,168,168,168
  testcase 0,    169,169,169,169,169,169,169,169,169,169
  ; ==> sprite location affects cycles

  ; 10 sprites split to two groups, X=N and X=M
  testcase 17,   0,  0,  0,  0,  0,  160,160,160,160,160
  testcase 16,   1,  1,  1,  1,  1,  161,161,161,161,161
  ; test #50
  testcase 15,   2,  2,  2,  2,  2,  162,162,162,162,162
  testcase 15,   3,  3,  3,  3,  3,  163,163,163,163,163
  testcase 15,   4,  4,  4,  4,  4,  164,164,164,164,164
  testcase 18,   5,  5,  5,  5,  5,  165,165,165,165,165
  ; test #54
  testcase 17,   6,  6,  6,  6,  6,  166,166,166,166,166
  testcase 17,   7,  7,  7,  7,  7,  167,167,167,167,167
  testcase 16,   64, 64, 64, 64, 64, 160,160,160,160,160
  testcase 16,   65, 65, 65, 65, 65, 161,161,161,161,161
  ; test #58
  testcase 15,   66, 66, 66, 66, 66, 162,162,162,162,162
  testcase 15,   67, 67, 67, 67, 67, 163,163,163,163,163
  testcase 15,   68, 68, 68, 68, 68, 164,164,164,164,164
  testcase 18,   69, 69, 69, 69, 69, 165,165,165,165,165
  ; test #5C
  testcase 17,   70, 70, 70, 70, 70, 166,166,166,166,166
  testcase 17,   71, 71, 71, 71, 71, 167,167,167,167,167
  ; ==> non-overlapping locations affect cycles

  testcase 17,   0,  0,  0,  0,  0,  0,  0,  0,  0,  1
  testcase 17,   0,  0,  0,  0,  0,  0,  0,  0,  0,  2
  ; test #60
  testcase 17,   0,  0,  0,  0,  0,  0,  0,  0,  0,  3
  testcase 17,   0,  0,  0,  0,  0,  0,  0,  0,  0,  4
  testcase 17,   0,  0,  0,  0,  0,  0,  0,  0,  1,  2
  testcase 17,   0,  0,  0,  0,  0,  0,  0,  1,  2,  3
  ; test #64
  testcase 17,   0,  0,  0,  0,  0,  0,  1,  2,  3,  4
  testcase 18,   0,  0,  0,  0,  0,  1,  2,  3,  4,  5
  testcase 18,   0,  0,  0,  0,  1,  2,  3,  4,  5,  6
  testcase 18,   0,  0,  0,  1,  2,  3,  4,  5,  6,  7
  ; test #68
  testcase 18,   0,  0,  1,  2,  3,  4,  5,  6,  7,  8
  testcase 18,   0,  1,  2,  3,  4,  5,  6,  7,  8,  9
  testcase 17,   1,  2,  3,  4,  5,  6,  7,  8,  9,  10
  testcase 17,   2,  3,  4,  5,  6,  7,  8,  9,  10, 11
  ; test #6C
  testcase 17,   3,  4,  5,  6,  7,  8,  9,  10, 11, 12
  testcase 18,   4,  5,  6,  7,  8,  9,  10, 11, 12, 13
  testcase 18,   5,  6,  7,  8,  9,  10, 11, 12, 13, 14
  testcase 18,   6,  7,  8,  9,  10, 11, 12, 13, 14, 15
  ; test #70
  testcase 17,   7,  8,  9,  10, 11, 12, 13, 14, 15, 16
  testcase 17,   8,  9,  10, 11, 12, 13, 14, 15, 16, 17
  testcase 17,   9,  10, 11, 12, 13, 14, 15, 16, 17, 18
  testcase 17,   10, 11, 12, 13, 14, 15, 16, 17, 18, 19
  ; test #74
  testcase 17,   11, 12, 13, 14, 15, 16, 17, 18, 19, 20
  testcase 18,   12, 13, 14, 15, 16, 17, 18, 19, 20, 21

  ; 10 sprites 8 bytes apart starting from X0=N
  testcase 21,   0,  8,  16, 24, 32, 40, 48, 56, 64, 72
  testcase 18,   1,  9,  17, 25, 33, 41, 49, 57, 65, 73
  ; test #78
  testcase 15,   2,  10, 18, 26, 34, 42, 50, 58, 66, 74
  testcase 15,   3,  11, 19, 27, 35, 43, 51, 59, 67, 75
  testcase 15,   4,  12, 20, 28, 36, 44, 52, 60, 68, 76
  testcase 28,   5,  13, 21, 29, 37, 45, 53, 61, 69, 77
   ; test #7C
  testcase 25,   6,  14, 22, 30, 38, 46, 54, 62, 70, 78
  testcase 23,   7,  15, 23, 31, 39, 47, 55, 63, 71, 79
  testcase 20,   8,  16, 24, 32, 40, 48, 56, 64, 72, 80

  ; 10 sprites 8 bytes apart starting from X0=N, reverse
  testcase 21,   72, 64, 56, 48, 40, 32, 24, 16, 8,  0
  testcase 18,   73, 65, 57, 49, 41, 33, 25, 17, 9,  1
  ; ==> sprite order does not affect cycles

  test_ok

run_testcase:
  push de
  push hl
  wait_vblank
  disable_lcd
  call clear_oam

  pop de
  call prepare_sprites
  pop de

  ld c, d
  ld hl, nop_area_a
  call prepare_nop_area
  ld c, e
  ld hl, nop_area_b
  call prepare_nop_area

  enable_lcd
  ; Enable sprites
  ld hl, LCDC
  set 1, (HL)
  ld a,$03
  ldh (<SCX),a

testcase_round_a:
  ld hl, testcase_round_a_ret
  push hl
  ld hl, nop_area_a
  push hl
  jp setup_and_wait_mode2

testcase_round_a_ret:
  ld b, $00
- inc b
  ldh a, (<STAT)
  and $03
  jr nz, -
  ld a, b
  ld c, $01
  cp c
  jp nz, test_fail_a

testcase_round_b:
  ld hl, testcase_round_b_ret
  push hl
  ld hl, nop_area_b
  push hl
  jp setup_and_wait_mode2

testcase_round_b_ret:
  ld b, $00
- inc b
  ldh a, (<STAT)
  and $03
  jr nz, -
  ld a, b
  ld c, $02
  cp c
  jp nz, test_fail_b
  ret

prepare_sprites:
  ld a, (de)
  ld c, a    ; amount of sprites
  ld b, $30  ; sprite tile
  ld hl, OAM

- inc de
  ; Sprite Y
  ld a, $52
  ld (hl+), a
  ; Sprite X
  ld a, (de)
  ld (hl+) ,a
  ; Sprite tile
  ld a, b
  ld (hl+), a
  inc b
  ; Sprite flags
  xor a
  ld (hl+), a

  dec c
  jr nz, -
  ret

prepare_nop_area:
  xor a
  inc c
- ld (hl+), a
  dec c
  jr nz, -

  ld a, $C9 ; RET instruction
  ld (hl+), a
  ret

setup_and_wait_mode2:
  disable_lcd
  enable_lcd
  wait_ly $42
  wait_mode $00
  wait_mode $03
  ld a, %00100000
  ldh (<STAT), a
  clear_interrupts
  ei

  nops 200
  jp fail_halt

clear_oam:
  ; Clear OAM
  ld hl, OAM
  ld bc, $a0
  xor a
  jp memset

test_fail_a:
  print_results _test_fail_a_cb
_test_fail_a_cb:
  print_string_literal "TEST A #"
  ld a, (testcase_id)
  call print_a
  print_string_literal " FAILED"
  ld d, $42
  ret

test_fail_b:
  print_results _test_fail_b_cb
_test_fail_b_cb:
  print_string_literal "TEST B #"
  ld a, (testcase_id)
  call print_a
  print_string_literal " FAILED"
  ld d, $42
  ret

fail_halt:
  test_failure_string "FAIL: HALT"

.org INTR_VEC_STAT
  add sp,+2
  ret

.ramsection "Test-State" slot 2
  testcase_id dw
  nop_area_a ds 96
  nop_area_b ds 96
.ends
