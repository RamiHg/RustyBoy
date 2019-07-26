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

; Test when writes to LYC get picked up for the LY==LYC check.


; Verified results:
;   pass: CGB, AGB
;   fail: MGB
;   not checked: DMG, SGB, SGB2, AGB

.incdir "../../common"
.include "common.s"

  di
  xor a
  ldh (<IE),a
  ldh (<IF),a
  ld a,%01000000
  ldh (<STAT),a
  ld a,2
  ldh (<IE),a

test_round1:
  di
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  ld a,2
  ldh (<LYC),a
  xor a
  ldh (<IF),a
  wait_ly 1
  ld b,0
  ld a,$f0
  ei
  nops 98
  ldh (<LYC),a
  nops 5
  ld a,b        ; no irq triggered
  ld (round1),a

test_round2:
  di
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  ld a,2
  ldh (<LYC),a
  xor a
  ldh (<IF),a
  wait_ly 1
  ld b,0
  ld a,$f0
  ei
  nops 99
  ldh (<LYC),a
  ld a,b        ; irq triggered
  ld (round2),a

test_round3:
  di
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  ld a,$f0
  ldh (<LYC),a
  xor a
  ldh (<IF),a
  wait_ly 1
  ld b,0
  ld a,2
  ei
  nops 211
  ldh (<LYC),a
  ldh a,(<STAT)
  ld (round5),a
  ld a,b         ; irq triggered
  ld (round3),a

test_round4:
  di
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  ld a,$f0
  ldh (<LYC),a
  xor a
  ldh (<IF),a
  wait_ly 1
  ld b,0
  ld a,2
  ei
  nops 212
  ldh (<LYC),a
  ldh a,(<STAT)
  ld (round6),a
  nops 5
  ld a,b         ; no irq triggered
  ld (round4),a

test_round7:
  di
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  ld a,2
  ldh (<LYC),a
  xor a
  ldh (<IF),a
  wait_ly 1
  ld b,0
  ld a,2
  ei
  nops 100
  ldh (<LYC),a
  dec a
  ldh (<LYC),a
  inc a
  ldh (<LYC),a
  dec a
  ldh (<LYC),a
  inc a
  ldh (<LYC),a
  dec a
  ldh (<LYC),a
  inc a
  ldh (<LYC),a
  dec a
  ldh (<LYC),a
  inc a
  ldh (<LYC),a
  dec a
  ldh (<LYC),a
  inc a
  ldh (<LYC),a
  dec a
  ldh (<LYC),a
  inc a
  ldh (<LYC),a
  dec a
  ldh (<LYC),a
  inc a
  ldh (<LYC),a
  dec a
  ldh (<LYC),a
  inc a
  ldh (<LYC),a
  ld a,b        ; irq triggered
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
  assert_a 7
  assert_b 0
  assert_c 1
  assert_d 1
  assert_e 0
  assert_h $C2
  assert_l $C2
  jp process_results

.org INTR_VEC_STAT
  inc b
  reti

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
