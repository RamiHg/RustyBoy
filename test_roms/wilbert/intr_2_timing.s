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

; Check when STAT IF flag gets set if STAT bit 5 is set.

; Verified results:
;   pass: MGB, CGB, AGS
;   fail:
;   not checked: DMG, SGB, SGB2, AGB

.incdir "../../common"
.include "common.s"

  di
  xor a
  ldh (<IE),a
  ldh (<IF),a
  ld a,$f0
  ldh (<LYC),a
  ld a,%00100000
  ldh (<STAT),a
  xor a
  ldh (<IF),a

test_round1:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  xor a
  ldh (<IF),a
  set 7,(hl)   ; LCD on
  ldh a,(<IF)
  ld (round1),a

test_round2:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  xor a
  ldh (<IF),a
  set 7,(hl)   ; LCD on
  nops 109
  ldh a,(<IF)
  ld (round2),a

test_round3:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  xor a
  ldh (<IF),a
  set 7,(hl)   ; LCD on
  nops 110
  ldh a,(<IF)
  ld (round3),a

test_round4:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  xor a
  ldh (<IF),a
  set 7,(hl)   ; LCD on
  nops 130
  ldh a,(<IF)
  ld (round4),a

test_round5:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  xor a
  ldh (<IF),a
  set 7,(hl)   ; LCD on
  wait_ly 143
  nops 70
  xor a
  ldh (<IF),a
  nops 26
  ldh a,(<IF)
  ld (round5),a

test_round6:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  xor a
  ldh (<IF),a
  set 7,(hl)   ; LCD on
  wait_ly 143
  nops 70
  xor a
  ldh (<IF),a
  nops 27
  ldh a,(<IF)
  ld (round6),a

test_round7:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  xor a
  ldh (<IF),a
  set 7,(hl)   ; LCD on
  wait_ly 143
  nops 70
  xor a
  ldh (<IF),a
  nops 28
  ldh a,(<IF)
  ld (round7),a


test_finish:
  ld a,(round1)
  ld b,a
  ld a,(round2)
  ld c,a
  ld a,(round3)
  ld d,a
  ld a,(round4)
  ld e,a
  ld a,(round5)
  ld h,a
  ld a,(round6) 
  ld l,a
  ld a,(round7)

  save_results
  assert_b $E0
  assert_c $E0
  assert_d $E2
  assert_e $E2
  assert_h $E0
  assert_l $E2
  assert_a $E3
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
