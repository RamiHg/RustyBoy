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
;
; This test was written by Wilbert Pol.

; This test verifies when the STAT and LY registers change when moving
; from line 0 to line 1.

; Verified results:
;   pass: MGB, CGB, AGS
;   fail: 
;   not checked: DMG, MGB, SGB, SGB2, CGB, AGB, AGS

.incdir "../../common"
.include "common.s"

  xor a
  ldh (<IE),a
  ldh (<IF),a
  ld a,$f0     ; make sure the LY=LYC flag never gets set
  ldh (<LYC),a

test_round3:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)
  nop
  set 7,(hl)
  wait_ly 152
  nops 330
  ldh a,(<LY) ; LY = 0
  ld (round3),a

test_round4:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)
  nop
  set 7,(hl)
  wait_ly 152
  nops 330
  ldh a,(<STAT) ; STAT = $80
  ld (round4),a

test_round5:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)
  nop
  set 7,(hl)
  wait_ly 152
  nops 331
  ldh a,(<LY) ; LY = 1
  ld (round5),a

test_round6:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)
  nop
  set 7,(hl)
  wait_ly 152
  nops 331
  ldh a,(<STAT) ; STAT = $80
  ld (round6),a

test_round7:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)
  nop
  set 7,(hl)
  wait_ly 152
  nops 332
  ldh a,(<LY) ; LY = 1
  ld (round7),a

test_round8:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)
  nop
  set 7,(hl)
  wait_ly 152
  nops 332
  ldh a,(<STAT) ; STAT = $82
  ld (round8),a

test_finish:
  ld a,(round3)
  ld b,a
  ld a,(round4)
  ld c,a
  ld a,(round5)
  ld d,a
  ld a,(round6)
  ld e,a
  ld a,(round7)
  ld h,a
  ld a,(round8) 
  ld l,a

  save_results
  assert_b 0
  assert_c $80
  assert_d 1
  assert_e $80
  assert_h 1
  assert_l $82
  jp process_results

.ramsection "Test-State" slot 2
  round1 db
  round2 db
  round3 db
  round4 db
  round5 db
  round6 db
  round7 db
  round8 db
.ends
