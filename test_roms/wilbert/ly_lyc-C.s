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

; Check when the LY==LYC flags get set and when interrupts are triggered.


; Verified results:
;   pass: CGB, AGS
;   fail: MGB
;   not checked: DMG, SGB, SGB2, AGB

.incdir "../../common"
.include "common.s"

  di
  xor a
  ldh (<IE),a
  ldh (<IF),a
  ld a,$02
  ldh (<LYC),a
  ld a,%01000000
  ldh (<STAT),a

test_round1:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 1
  nops 100
  ldh a,(<LY) ; LY = 1
  ld (round1),a

test_round2:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 1
  nops 101
  ldh a,(<LY) ; LY = 2
  ld (round2),a

test_round3:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 1
  nops 101
  ldh a,(<STAT) ; STAT = $C0
  ld (round3),a

test_round4:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 1
  nops 102
  ldh a,(<STAT) ; STAT = $C6
  ld (round4),a

test_round5:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 1
  nops 101
  ldh a,(<IF) ; IF = $E0
  ld (round5),a

test_round6:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 1
  nops 102
  ldh a,(<IF) ; IF = $E4
  ld (round6),a

test_round7:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 1
  nops 215
  ldh a,(<STAT) ; STAT = $C4
  ld (round7),a

test_round8:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 1
  nops 216
  ldh a,(<STAT) ; STAT = $C2
  ld (round8),a


test_finish:
  ld a,(round1)
  ld b,a
  ld a,(round2)
  rla
  rla
  rla
  rla
  ld c,a
  push bc
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
  pop af

  save_results
  assert_a $01
  assert_f $20
  assert_b $C0
  assert_c $C6
  assert_d $E0
  assert_e $E2
  assert_h $C4
  assert_l $C2
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
